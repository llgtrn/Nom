//! `resolve_closure` — stub resolver for unresolved refs in a `ConceptClosure`.

use nom_concept::{ConceptClosure, UnresolvedRef};
use nom_dict::{NomDict, WordV2Row};

/// Statistics produced by `resolve_closure`.
#[derive(Debug, Default)]
pub struct ResolveStats {
    pub resolved: usize,
    pub still_unresolved: usize,
    /// Refs with more than one candidate; picked by alphabetical-smallest hash.
    pub ambiguous: usize,
}

/// A single unresolved ref that was matched against `words_v2`.
#[derive(Debug, Clone)]
pub struct ResolvedRef {
    pub word: String,
    pub kind: Option<String>,
    /// The hash that was picked (alphabetically smallest among candidates).
    pub hash: String,
    /// Other candidates' hashes (empty when only one match existed).
    pub alternatives: Vec<String>,
    /// Per-slot inline confidence threshold (doc 07 §6.3), propagated from
    /// the source `EntityRef`. Phase-9 corpus-embedding-resolver enforces this.
    /// Stub resolver records but ignores it.
    pub confidence_threshold: Option<f64>,
    /// The prose matching hint from the source typed-slot `the @Kind matching "..."`,
    /// propagated from `UnresolvedRef::matching`. Used by doc 07 §3.3 diagnostics.
    pub matching: Option<String>,
}

/// Resolve unresolved refs from a closure against the DB's `words_v2` table.
///
/// Strategy (stub — Phase 9 will replace with deterministic per-kind embedding
/// index per doc 08 §5.3):
///
/// **v1 (word-based)**: `uref.typed_slot == false`
/// - Query `find_words_v2_by_word(ref.word)`.
/// - Filter by kind if `ref.kind` is `Some`.
///
/// **v2 (typed-slot, `.nomx v2 keyed`)**: `uref.typed_slot == true`
/// - Query `find_words_v2_by_kind(ref.kind)` — no word to anchor on.
/// - All candidates from this query share the declared kind already, so no
///   additional kind-filter step is needed.
/// - The `ResolvedRef` produced keeps `kind` set and `word` empty (the source
///   had no bare word).  Downstream consumers (nom-app dream, future planner)
///   should treat `typed_slot + word="" + kind=Some(k)` as a kind-only resolution.
/// - Writeback (`apply_hash_locks`) intentionally skips typed-slot refs: there
///   is no word token in the source line to anchor a `@<hash>` splice.  Per
///   doc 07 §3.5 the hash lives in the manifest/DB only.
///
/// For both v1 and v2:
/// - 0 matches → still unresolved.
/// - 1 match → resolved to that hash.
/// - N matches → pick alphabetically-smallest hash (stable, deterministic per
///   §10.3.1).  Record remaining hashes in `alternatives`.
///
/// # TODO: Phase 9 — replace with per-kind embedding index (doc 08 §5.3)
pub fn resolve_closure(
    closure: &ConceptClosure,
    dict: &NomDict,
) -> (Vec<ResolvedRef>, Vec<UnresolvedRef>, ResolveStats) {
    let mut resolved_refs: Vec<ResolvedRef> = Vec::new();
    let mut still_unresolved: Vec<UnresolvedRef> = Vec::new();
    let mut stats = ResolveStats::default();

    for uref in &closure.unresolved {
        // Obtain candidates depending on ref form (v1 word-based vs v2 typed-slot).
        let candidates: Vec<WordV2Row> = if uref.typed_slot {
            // .nomx v2 keyed: lookup by kind alone; word is empty.
            // TODO Phase 9: re-rank by `uref.matching` semantic similarity.
            match &uref.kind {
                Some(k) => match dict.find_words_v2_by_kind(k) {
                    Ok(rows) => rows,
                    Err(e) => {
                        eprintln!("nom: resolve_closure: db error for kind `{k}`: {e}");
                        still_unresolved.push(uref.clone());
                        stats.still_unresolved += 1;
                        continue;
                    }
                },
                None => {
                    // Typed-slot with no kind — cannot resolve.
                    still_unresolved.push(uref.clone());
                    stats.still_unresolved += 1;
                    continue;
                }
            }
        } else {
            // v1: word name lookup, optional kind filter.
            let mut rows = match dict.find_words_v2_by_word(&uref.word) {
                Ok(rows) => rows,
                Err(e) => {
                    eprintln!("nom: resolve_closure: db error for `{}`: {e}", uref.word);
                    still_unresolved.push(uref.clone());
                    stats.still_unresolved += 1;
                    continue;
                }
            };
            // Filter by kind if the ref declares one.
            if let Some(kind) = &uref.kind {
                rows.retain(|r| r.kind == *kind);
            }
            rows
        };

        match candidates.len() {
            0 => {
                still_unresolved.push(uref.clone());
                stats.still_unresolved += 1;
            }
            1 => {
                resolved_refs.push(ResolvedRef {
                    word: uref.word.clone(),
                    kind: uref.kind.clone(),
                    hash: candidates[0].hash.clone(),
                    alternatives: vec![],
                    confidence_threshold: uref.confidence_threshold,
                    matching: uref.matching.clone(),
                });
                stats.resolved += 1;
            }
            _ => {
                // `candidates` is already ordered by hash (ORDER BY hash in the query).
                // The first entry is alphabetically smallest.
                let picked = candidates[0].hash.clone();
                let alternatives: Vec<String> =
                    candidates[1..].iter().map(|r| r.hash.clone()).collect();
                resolved_refs.push(ResolvedRef {
                    word: uref.word.clone(),
                    kind: uref.kind.clone(),
                    hash: picked,
                    alternatives,
                    confidence_threshold: uref.confidence_threshold,
                    matching: uref.matching.clone(),
                });
                stats.resolved += 1;
                stats.ambiguous += 1;
            }
        }
    }

    (resolved_refs, still_unresolved, stats)
}

// ── resolve_closure unit tests ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use nom_concept::{ConceptClosure, UnresolvedRef};
    use nom_dict::{NomDict, WordV2Row};

    use super::resolve_closure;

    fn make_closure(urefs: Vec<UnresolvedRef>) -> ConceptClosure {
        ConceptClosure {
            root: "test_root".to_string(),
            word_hashes: vec![],
            concepts: vec![],
            unresolved: urefs,
        }
    }

    fn make_uref_typed_slot(kind: &str) -> UnresolvedRef {
        UnresolvedRef {
            kind: Some(kind.to_string()),
            word: String::new(),
            matching: None,
            referenced_from: "test_concept".to_string(),
            typed_slot: true,
            confidence_threshold: None,
        }
    }

    fn make_uref_typed_slot_with_matching(kind: &str, matching: &str) -> UnresolvedRef {
        UnresolvedRef {
            kind: Some(kind.to_string()),
            word: String::new(),
            matching: Some(matching.to_string()),
            referenced_from: "test_concept".to_string(),
            typed_slot: true,
            confidence_threshold: None,
        }
    }

    fn make_uref_word(word: &str, kind: Option<&str>) -> UnresolvedRef {
        UnresolvedRef {
            kind: kind.map(String::from),
            word: word.to_string(),
            matching: None,
            referenced_from: "test_concept".to_string(),
            typed_slot: false,
            confidence_threshold: None,
        }
    }

    fn make_fn_row(hash: &str, word: &str) -> WordV2Row {
        WordV2Row {
            hash: hash.to_string(),
            word: word.to_string(),
            kind: "function".to_string(),
            signature: None,
            contracts: None,
            body_kind: None,
            body_size: None,
            origin_ref: None,
            bench_ids: None,
            authored_in: None,
            composed_of: None,
        }
    }

    fn open_dict_with_rows(rows: &[WordV2Row]) -> NomDict {
        let d = NomDict::open_in_memory().expect("in-memory dict");
        for r in rows {
            d.upsert_word_v2(r).expect("upsert");
        }
        d
    }

    /// Typed-slot ref with kind="function" + 1 candidate → resolves to that hash.
    #[test]
    fn typed_slot_one_candidate_resolves() {
        let d = open_dict_with_rows(&[make_fn_row("aaa111", "some_fn")]);
        let closure = make_closure(vec![make_uref_typed_slot("function")]);
        let (resolved, unresolved, stats) = resolve_closure(&closure, &d);

        assert_eq!(stats.resolved, 1);
        assert_eq!(stats.still_unresolved, 0);
        assert!(unresolved.is_empty());
        assert_eq!(resolved.len(), 1);
        let r = &resolved[0];
        assert_eq!(r.hash, "aaa111");
        assert_eq!(r.word, "");  // typed-slot: word stays empty
        assert_eq!(r.kind.as_deref(), Some("function"));
        assert!(r.alternatives.is_empty());
    }

    /// Typed-slot ref with kind="function" + 2 candidates → picks alphabetically-smaller hash;
    /// alternatives list contains the other.
    #[test]
    fn typed_slot_two_candidates_picks_smallest_hash() {
        let d = open_dict_with_rows(&[
            make_fn_row("aaa-first", "fn_a"),
            make_fn_row("zzz-second", "fn_b"),
        ]);
        let closure = make_closure(vec![make_uref_typed_slot("function")]);
        let (resolved, unresolved, stats) = resolve_closure(&closure, &d);

        assert_eq!(stats.resolved, 1);
        assert_eq!(stats.ambiguous, 1);
        assert!(unresolved.is_empty());
        assert_eq!(resolved.len(), 1);
        let r = &resolved[0];
        assert_eq!(r.hash, "aaa-first");
        assert_eq!(r.alternatives, vec!["zzz-second"]);
    }

    /// Typed-slot ref with kind="function" + 0 candidates → stays unresolved.
    #[test]
    fn typed_slot_no_candidates_stays_unresolved() {
        let d = open_dict_with_rows(&[]);
        let closure = make_closure(vec![make_uref_typed_slot("function")]);
        let (resolved, unresolved, stats) = resolve_closure(&closure, &d);

        assert_eq!(stats.still_unresolved, 1);
        assert_eq!(stats.resolved, 0);
        assert!(resolved.is_empty());
        assert_eq!(unresolved.len(), 1);
        assert!(unresolved[0].typed_slot);
    }

    /// v1 word-based ref still resolves as before (regression guard).
    #[test]
    fn word_based_ref_resolves_normally() {
        let d = open_dict_with_rows(&[make_fn_row("abc123", "read_file")]);
        let closure = make_closure(vec![make_uref_word("read_file", Some("function"))]);
        let (resolved, unresolved, stats) = resolve_closure(&closure, &d);

        assert_eq!(stats.resolved, 1);
        assert!(unresolved.is_empty());
        assert_eq!(resolved[0].hash, "abc123");
        assert_eq!(resolved[0].word, "read_file");
    }

    /// Typed-slot ref with 3 candidates and a matching hint: resolved ref carries
    /// the picked hash + 2 alternatives + the matching string propagated through.
    ///
    /// Doc 07 §3.3: alternatives list fed to the `nom build status` diagnostic.
    #[test]
    fn typed_slot_three_candidates_propagates_matching_and_alternatives() {
        let d = open_dict_with_rows(&[
            make_fn_row("aaa-fn-first", "fetch_url"),
            make_fn_row("bbb-fn-second", "list_dir"),
            make_fn_row("ccc-fn-third", "read_file"),
        ]);
        let uref = make_uref_typed_slot_with_matching("function", "fetch the body of an https URL");
        let closure = make_closure(vec![uref]);
        let (resolved, unresolved, stats) = resolve_closure(&closure, &d);

        assert_eq!(stats.resolved, 1, "should resolve to one picked hash");
        assert_eq!(stats.ambiguous, 1, "ambiguous because N>1");
        assert!(unresolved.is_empty());
        assert_eq!(resolved.len(), 1);

        let r = &resolved[0];
        // Picked hash is alphabetically smallest.
        assert_eq!(r.hash, "aaa-fn-first");
        // Two alternatives: the other two hashes in hash order.
        assert_eq!(r.alternatives, vec!["bbb-fn-second", "ccc-fn-third"]);
        // matching propagated from uref.
        assert_eq!(
            r.matching.as_deref(),
            Some("fetch the body of an https URL")
        );
        // kind propagated.
        assert_eq!(r.kind.as_deref(), Some("function"));
        // word stays empty for typed-slot.
        assert_eq!(r.word, "");
    }
}
