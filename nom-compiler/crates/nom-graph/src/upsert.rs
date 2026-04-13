//! Phase 2b: `upsert_entry` + rename chain as a parallel path alongside
//! the existing `from_entries` builder. Existing `Vec<NomtuNode>`
//! adjacency in `lib.rs` is untouched; this module writes to new
//! HashMap-backed fields on `NomtuGraph` so future cycles can migrate
//! callers without a breaking big-bang.
//!
//! Spec: `docs/superpowers/specs/2026-04-14-graph-durability-design.md`
//! (Phase 2). Depends on Phase 2a's `uid::compute_node_uid` (shipped
//! `2453375`).
//!
//! ## What lands in 2b (this wedge)
//!
//! - `UpsertOutcome` enum (Unchanged / Created / Updated / Renamed)
//! - `upsert_entry(&mut self, &NomtuEntry) -> UpsertOutcome`
//! - `history_of(&self, &NodeUid) -> &[NodeUid]` (rename-chain accessor)
//! - `get_node_by_uid(&self, &NodeUid) -> Option<&NomtuNode>`
//! - Tests locking every outcome variant
//!
//! ## What stays out until 2c
//!
//! - Edge reattachment on `Renamed` (confidence-filtered dirty-set
//!   propagation). Current behavior: rename updates `prior_hashes`
//!   chain but existing edges in `Vec<NomtuEdge>` continue to point at
//!   their old (word, variant) pairs. Not a regression — `from_entries`
//!   based graphs never had rename handling either.
//! - Switching `build_call_edges` / `build_import_edges` / `from_entries`
//!   to use the uid-addressed storage. They still read from the
//!   existing Vec fields.
//! - Cypher export consuming the uid-addressed graph (Phase 3).

use nom_types::NomtuEntry;

use crate::uid::{compute_node_uid, NodeUid};
use crate::{NomtuGraph, NomtuNode};

/// Result of an `upsert_entry` call. Every variant carries the uid(s)
/// involved so callers can thread rename bookkeeping downstream
/// (e.g. the resolver cache, glass-box report).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpsertOutcome {
    /// Entry already present with identical (word, kind, body_hash,
    /// language). No-op; returns the existing uid for chaining.
    Unchanged { uid: NodeUid },
    /// First time this uid was seen. Node inserted.
    Created { uid: NodeUid },
    /// Uid matched but some non-identity field differed (language). The
    /// stored node was overwritten with the new row.
    Updated { uid: NodeUid },
    /// (word, kind, variant) already known but the **body_hash** changed
    /// → uid differs. The old uid → new uid link is recorded in
    /// `prior_hashes`. Callers can walk the chain via `history_of`.
    Renamed { from: NodeUid, to: NodeUid },
}

impl UpsertOutcome {
    /// The uid that represents the entry's CURRENT identity after the
    /// upsert — i.e. the node just inserted, updated, or the rename
    /// target. For `Unchanged` this is the prior uid.
    pub fn current_uid(&self) -> &NodeUid {
        match self {
            Self::Unchanged { uid }
            | Self::Created { uid }
            | Self::Updated { uid } => uid,
            Self::Renamed { to, .. } => to,
        }
    }
}

impl NomtuGraph {
    /// Insert or update an entry in the uid-addressed storage. Returns
    /// an outcome describing what happened so the caller can feed
    /// downstream bookkeeping (resolver cache invalidation, glass-box
    /// report rename warnings, etc.).
    ///
    /// Identity contract (Phase 2a):
    /// `uid = sha256(word || 0x00 || kind || 0x00 || body_hash)`
    ///
    /// Rename detection: looks up by `(word, kind, variant)` in
    /// `word_variant_index`. If found and uid matches → Unchanged or
    /// Updated. If found but uid differs → Renamed (body_hash drifted).
    /// Never matched → Created.
    pub fn upsert_entry(&mut self, entry: &NomtuEntry) -> UpsertOutcome {
        let new_uid = compute_node_uid(entry);
        let key = (entry.word.clone(), entry.kind.clone(), entry.variant.clone());
        let node = NomtuNode {
            word: entry.word.clone(),
            variant: entry.variant.clone(),
            language: entry.language.clone(),
            kind: entry.kind.clone(),
            body_hash: entry.body_hash.clone(),
        };

        if let Some(prior_uid) = self.word_variant_index.get(&key).cloned() {
            if prior_uid == new_uid {
                // Uid unchanged — check if body of node differs. Body-level
                // identity is the uid, but language can drift without the
                // uid changing (variant is not part of uid; language isn't
                // either by design). Treat language drift as Updated.
                if let Some(existing) = self.uid_nodes.get(&new_uid) {
                    if existing.language == node.language
                        && existing.body_hash == node.body_hash
                    {
                        return UpsertOutcome::Unchanged { uid: new_uid };
                    }
                }
                self.uid_nodes.insert(new_uid.clone(), node);
                UpsertOutcome::Updated { uid: new_uid }
            } else {
                // Body changed → new uid. Record old→new in prior_hashes
                // and evict the stale node from uid_nodes (old uid stays
                // reachable ONLY via the rename chain — direct lookups
                // return None, signaling to callers "this uid is retired,
                // walk history_of to find current").
                self.prior_hashes
                    .entry(new_uid.clone())
                    .or_default()
                    .push(prior_uid.clone());
                self.uid_nodes.remove(&prior_uid);
                self.uid_nodes.insert(new_uid.clone(), node);
                self.word_variant_index.insert(key, new_uid.clone());
                UpsertOutcome::Renamed { from: prior_uid, to: new_uid }
            }
        } else {
            self.uid_nodes.insert(new_uid.clone(), node);
            self.word_variant_index.insert(key, new_uid.clone());
            UpsertOutcome::Created { uid: new_uid }
        }
    }

    /// Return the prior uids that renamed INTO `current_uid`, oldest
    /// first. Empty slice if this uid is its own origin.
    ///
    /// Walking the full rename chain across generations is a slice-
    /// later concern — this accessor returns only direct predecessors,
    /// matching the `prior_hashes: HashMap<Uid, Vec<Uid>>` shape.
    pub fn history_of(&self, current_uid: &NodeUid) -> &[NodeUid] {
        self.prior_hashes
            .get(current_uid)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Look up a node by its content-hash uid in the new storage.
    /// Returns `None` if the uid is unknown OR if it has only been seen
    /// as a prior uid (i.e. the node was renamed and this uid is stale).
    pub fn get_node_by_uid(&self, uid: &NodeUid) -> Option<&NomtuNode> {
        self.uid_nodes.get(uid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk(word: &str, kind: &str, body: Option<&str>, lang: &str) -> NomtuEntry {
        NomtuEntry {
            word: word.into(),
            kind: kind.into(),
            body_hash: body.map(|s| s.into()),
            language: lang.into(),
            ..Default::default()
        }
    }

    fn mk_with_variant(
        word: &str,
        kind: &str,
        body: Option<&str>,
        variant: Option<&str>,
    ) -> NomtuEntry {
        NomtuEntry {
            word: word.into(),
            kind: kind.into(),
            body_hash: body.map(|s| s.into()),
            variant: variant.map(|s| s.into()),
            language: "rust".into(),
            ..Default::default()
        }
    }

    #[test]
    fn upsert_new_entry_returns_created() {
        let mut g = NomtuGraph::new();
        let e = mk("add", "function", Some("h1"), "rust");
        let outcome = g.upsert_entry(&e);
        assert!(matches!(outcome, UpsertOutcome::Created { .. }));
        assert!(g.get_node_by_uid(outcome.current_uid()).is_some());
    }

    #[test]
    fn upsert_same_entry_twice_returns_unchanged() {
        let mut g = NomtuGraph::new();
        let e = mk("add", "function", Some("h1"), "rust");
        let first = g.upsert_entry(&e);
        let second = g.upsert_entry(&e);
        assert!(matches!(first, UpsertOutcome::Created { .. }));
        assert!(matches!(second, UpsertOutcome::Unchanged { .. }));
        assert_eq!(first.current_uid(), second.current_uid());
    }

    #[test]
    fn upsert_language_drift_returns_updated() {
        let mut g = NomtuGraph::new();
        let e1 = mk("add", "function", Some("h1"), "rust");
        let e2 = mk("add", "function", Some("h1"), "python");
        g.upsert_entry(&e1);
        let outcome = g.upsert_entry(&e2);
        // Same uid (body_hash unchanged) but language drifted → Updated.
        assert!(
            matches!(outcome, UpsertOutcome::Updated { .. }),
            "language drift must produce Updated, got {outcome:?}"
        );
        let stored = g.get_node_by_uid(outcome.current_uid()).unwrap();
        assert_eq!(stored.language, "python");
    }

    #[test]
    fn upsert_body_change_returns_renamed_and_records_prior() {
        let mut g = NomtuGraph::new();
        let e1 = mk("add", "function", Some("h1"), "rust");
        let e2 = mk("add", "function", Some("h2"), "rust");
        let first = g.upsert_entry(&e1);
        let renamed = g.upsert_entry(&e2);
        match &renamed {
            UpsertOutcome::Renamed { from, to } => {
                assert_eq!(from, first.current_uid());
                assert_ne!(from, to);
                assert_eq!(g.history_of(to), std::slice::from_ref(from));
            }
            other => panic!("expected Renamed, got {other:?}"),
        }
    }

    #[test]
    fn upsert_different_variant_is_different_key() {
        let mut g = NomtuGraph::new();
        let a = mk_with_variant("add", "function", Some("h"), Some("v1"));
        let b = mk_with_variant("add", "function", Some("h"), Some("v2"));
        let oa = g.upsert_entry(&a);
        let ob = g.upsert_entry(&b);
        // Same uid (variant doesn't affect uid per Phase 2a), but different
        // (word, kind, variant) keys → b is also Created, not Unchanged.
        assert!(matches!(oa, UpsertOutcome::Created { .. }));
        assert!(matches!(ob, UpsertOutcome::Created { .. }));
        // Both resolve to the same uid though.
        assert_eq!(oa.current_uid(), ob.current_uid());
    }

    #[test]
    fn history_of_returns_empty_for_unknown_uid() {
        let g = NomtuGraph::new();
        let empty: &[NodeUid] = g.history_of(&"deadbeef".to_string());
        assert!(empty.is_empty());
    }

    #[test]
    fn renaming_twice_accumulates_prior_uids() {
        let mut g = NomtuGraph::new();
        let e1 = mk("x", "function", Some("h1"), "rust");
        let e2 = mk("x", "function", Some("h2"), "rust");
        let e3 = mk("x", "function", Some("h3"), "rust");
        let u1 = g.upsert_entry(&e1).current_uid().clone();
        let r2 = g.upsert_entry(&e2);
        let u2 = r2.current_uid().clone();
        let r3 = g.upsert_entry(&e3);
        let u3 = r3.current_uid().clone();

        assert_eq!(g.history_of(&u2), std::slice::from_ref(&u1));
        assert_eq!(g.history_of(&u3), std::slice::from_ref(&u2));
        // Old uids resolve to nothing in the current store (only most-
        // recent uid is queryable; to find previous versions walk the chain).
        assert!(g.get_node_by_uid(&u1).is_none());
        assert!(g.get_node_by_uid(&u2).is_none());
        assert!(g.get_node_by_uid(&u3).is_some());
    }

    #[test]
    fn current_uid_matches_stored_node_identity() {
        let mut g = NomtuGraph::new();
        let e = mk("add", "function", Some("h1"), "rust");
        let outcome = g.upsert_entry(&e);
        let stored = g.get_node_by_uid(outcome.current_uid()).unwrap();
        assert_eq!(stored.word, "add");
        assert_eq!(stored.kind, "function");
        assert_eq!(stored.body_hash.as_deref(), Some("h1"));
    }
}
