//! M8 slice-3a: `DictTools` — real `AgentTools` impl backed by `nom-dict`.
//!
//! Wires the first of the 5 grouped tools (`query`) to production code:
//! `find_word_v2` by hash, `find_words_v2_by_kind` by kind. The other 4
//! methods (compose/verify/render/explain) return `Observation::Error`
//! with an explicit "not yet wired" message so the loop doesn't silently
//! pretend to work — matches the discipline established in M8 slice-1.
//!
//! Slice-3b (next cycle) wires compose + verify (depends on MECE
//! integration in `nom-concept`). Slice-3c wires render + explain.
//!
//! Design notes:
//! - Ownership: `DictTools` borrows a `&NomDict`. Callers open the dict
//!   once and pass a reference; we never open/close per-call.
//! - Deterministic: the nom-dict SQL queries already order by the
//!   alphabetical-smallest tiebreak (slice-1 M8 discipline), so query
//!   results are stable across runs.
//! - Budget: `query()` applies a sane upper bound (`max_results`,
//!   default 50) per CRAG's "narrow before LLM" rule — without this the
//!   `Candidates` list could swamp the LLM's context.

use nom_dict::WordV2Row;
use sha2::{Digest, Sha256};

use crate::react::{AgentTools, Observation};

/// SHA-256 over a deterministic serialization of a closure's
/// (uid, body_kind) pairs. Each pair appears once, sorted by uid;
/// pair serialization is `uid + "\t" + body_kind + "\n"`. Two walks
/// over the same dict state produce byte-identical hashes.
fn hash_closure(pairs: &[(String, String)]) -> String {
    let mut hasher = Sha256::new();
    for (uid, body_kind) in pairs {
        hasher.update(uid.as_bytes());
        hasher.update(b"\t");
        hasher.update(body_kind.as_bytes());
        hasher.update(b"\n");
    }
    format!("{:x}", hasher.finalize())
}

/// Build a compact human-readable summary of a dict entry. Depth 0 = one
/// line with word+kind+body_kind+body_size; depth ≥ 1 adds signature +
/// whether contracts exist + authoring origin. Glass-box-report level
/// detail (LayeredDreamReport etc.) is out of scope until slice-3c-full
/// wires nom-app's report module.
fn format_entry_summary(row: &WordV2Row, depth: usize) -> String {
    let uid_short = &row.hash[..12.min(row.hash.len())];
    let body = row
        .body_kind
        .as_deref()
        .map(|k| format!(" body={k}"))
        .unwrap_or_default();
    let size = row
        .body_size
        .map(|n| format!(" size={n}"))
        .unwrap_or_default();
    let base = format!("{}@{}: kind={}{}{}", row.word, uid_short, row.kind, body, size);
    if depth == 0 {
        return base;
    }
    let sig = match &row.signature {
        Some(s) if !s.is_empty() => format!("\n  signature: {s}"),
        _ => "\n  signature: <none>".into(),
    };
    let contracts = match &row.contracts {
        Some(c) if !c.is_empty() && c != "[]" => format!("\n  contracts: present ({} bytes)", c.len()),
        _ => "\n  contracts: <none>".into(),
    };
    let origin = match &row.authored_in {
        Some(p) => format!("\n  authored_in: {p}"),
        None => "\n  authored_in: <corpus>".into(),
    };
    format!("{base}{sig}{contracts}{origin}")
}

/// Production impl of `AgentTools` backed by a live `nom-dict` connection.
/// Slice-3a ships `query` only; other methods are explicit stubs.
pub struct DictTools<'a> {
    dict: &'a nom_dict::NomDict,
    /// Max candidate UIDs returned from `query()`. Higher = more LLM
    /// context cost; lower = risks missing the right answer. Default 50
    /// per CRAG retrieval budgets in doc 11 §2.
    pub max_results: usize,
}

impl<'a> DictTools<'a> {
    pub fn new(dict: &'a nom_dict::NomDict) -> Self {
        Self { dict, max_results: 50 }
    }

    pub fn with_max_results(mut self, max: usize) -> Self {
        self.max_results = max;
        self
    }

    /// Core lookup: try hash-exact first, fall back to kind-scoped.
    /// Exposed as a pub fn so slice-3b / slice-3c can reuse it when
    /// implementing `verify` (needs UID lookup) and `explain` (same).
    /// Walk the composed_of closure rooted at `uid`. Returns a
    /// deterministic-order `Vec<(uid, body_kind)>` covering every
    /// transitive dependency including the root. Cycle-safe via a
    /// visited set. Returns `Err(msg)` only on dict I/O errors;
    /// unresolved composed_of UIDs are silently skipped (matches the
    /// lenient M6 corpus semantics where not every referenced hash is
    /// necessarily present in-dict).
    ///
    /// Exposed `pub` so slice-3c-full (real linker) and slice-4
    /// (InstrumentedTools glass-box) can reuse the walk.
    pub fn compute_closure(&self, root_uid: &str) -> Result<Vec<(String, String)>, String> {
        let mut visited = std::collections::BTreeSet::<String>::new();
        let mut queue: std::collections::VecDeque<String> =
            std::collections::VecDeque::new();
        queue.push_back(root_uid.to_string());
        while let Some(uid) = queue.pop_front() {
            if visited.contains(&uid) {
                continue;
            }
            let row = match self.dict.find_word_v2(&uid) {
                Ok(Some(r)) => r,
                Ok(None) => continue, // unresolved ref — skip, don't fail
                Err(e) => return Err(format!("dict error on {uid}: {e}")),
            };
            visited.insert(uid.clone());
            // composed_of is a JSON array of uids when present.
            if let Some(composed_json) = row.composed_of.as_deref() {
                if let Ok(children) = serde_json::from_str::<Vec<String>>(composed_json) {
                    for child in children {
                        if !visited.contains(&child) {
                            queue.push_back(child);
                        }
                    }
                }
            }
        }
        // Emit (uid, body_kind) pairs in sorted uid order so two walks
        // over the same dict state produce byte-identical output.
        let mut out: Vec<(String, String)> = visited
            .into_iter()
            .filter_map(|uid| {
                self.dict.find_word_v2(&uid).ok().flatten().map(|row| {
                    let body_kind = row
                        .body_kind
                        .unwrap_or_else(|| "<no-body>".to_string());
                    (row.hash, body_kind)
                })
            })
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(out)
    }

    pub fn lookup_candidates(&self, subject: &str, kind: Option<&str>) -> Vec<String> {
        // Hash-exact match: `subject` may be a 64-hex UID.
        if subject.len() == 64 && subject.chars().all(|c| c.is_ascii_hexdigit()) {
            if let Ok(Some(row)) = self.dict.find_word_v2(subject) {
                return vec![row.hash];
            }
            return Vec::new();
        }

        // Kind-scoped fallback: match every word of the given kind, filter
        // by substring match on `word` field. Slice-3b will add proper
        // full-text / embedding retrieval.
        let Some(kind) = kind else {
            // No kind + no hash = too broad. Return empty and let the LLM
            // reason about why.
            return Vec::new();
        };
        let rows = match self.dict.find_words_v2_by_kind(kind) {
            Ok(rows) => rows,
            Err(_) => return Vec::new(),
        };
        let needle = subject.to_lowercase();
        let mut matches: Vec<String> = rows
            .into_iter()
            .filter(|row| row.word.to_lowercase().contains(&needle))
            .map(|row| row.hash)
            .collect();
        matches.sort();
        matches.truncate(self.max_results);
        matches
    }
}

impl<'a> AgentTools for DictTools<'a> {
    fn query(&self, subject: &str, kind: Option<&str>, _depth: usize) -> Observation {
        let candidates = self.lookup_candidates(subject, kind);
        Observation::Candidates(candidates)
    }

    fn compose(&self, _prose: &str, _context: &[String]) -> Observation {
        Observation::Error("DictTools::compose not yet wired (slice-3b)".into())
    }

    fn verify(&self, _target: &str) -> Observation {
        Observation::Error("DictTools::verify not yet wired (slice-3b)".into())
    }

    fn render(&self, uid: &str, target: &str) -> Observation {
        // Slice-3c-render-metadata: walk the closure starting at `uid`,
        // collect all (uid, body_kind) pairs in deterministic sorted
        // order, and return a SHA-256 "render-plan hash" that identifies
        // the set of artifacts a real render would produce. This is a
        // proof-of-closure-walk wedge — slice-3c-full will replace the
        // hash with real linker/asset-bundling output, but the WALK + HASH
        // shape stays stable (byte-identical across runs for the same
        // closure → downstream ReAct loops can assert idempotence).
        //
        // `target` is passed through to the Observation for caller
        // bookkeeping (e.g. "llvm-native" vs "wasm" would produce same
        // render-plan but the agent tracks which target it asked for).
        if uid.len() != 64 || !uid.chars().all(|c| c.is_ascii_hexdigit()) {
            return Observation::Error(format!(
                "DictTools::render: uid {uid:?} is not a 64-char hex hash"
            ));
        }
        let closure = match self.compute_closure(uid) {
            Ok(c) => c,
            Err(msg) => return Observation::Error(msg),
        };
        if closure.is_empty() {
            return Observation::Error(format!(
                "DictTools::render: uid {uid} not found in dict"
            ));
        }
        let plan_hash = hash_closure(&closure);
        Observation::Rendered {
            target: target.into(),
            bytes_hash: plan_hash,
        }
    }

    fn explain(&self, uid: &str, depth: usize) -> Observation {
        // Slice-3c partial: wire explain against nom-dict. Looks up the entry
        // and emits a short one-line summary. Deeper depths (e.g. full
        // LayeredDreamReport + glass-box JSON) land in slice-3c-full.
        if uid.len() != 64 || !uid.chars().all(|c| c.is_ascii_hexdigit()) {
            return Observation::Error(format!(
                "DictTools::explain: uid {uid:?} is not a 64-char hex hash"
            ));
        }
        match self.dict.find_word_v2(uid) {
            Ok(Some(row)) => {
                let summary = format_entry_summary(&row, depth);
                Observation::Explanation { summary }
            }
            Ok(None) => {
                Observation::Error(format!("DictTools::explain: uid {uid} not found in dict"))
            }
            Err(e) => Observation::Error(format!("DictTools::explain: dict error: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom_dict::{NomDict, WordV2Row};

    fn seed_word(d: &NomDict, hash: &str, word: &str, kind: &str) {
        d.upsert_word_v2(&WordV2Row {
            hash: hash.into(),
            word: word.into(),
            kind: kind.into(),
            signature: None,
            contracts: None,
            body_kind: None,
            body_size: None,
            origin_ref: None,
            bench_ids: None,
            authored_in: None,
            composed_of: None,
        })
        .unwrap();
    }

    /// A 64-char hex hash; content doesn't matter for the query test, we
    /// just need to hit the hash-exact branch.
    const HASH_ADD: &str = "a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0";
    const HASH_MUL: &str = "b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1";

    #[test]
    fn query_by_hash_returns_exact_match() {
        let d = NomDict::open_in_memory().unwrap();
        seed_word(&d, HASH_ADD, "add", "function");
        let tools = DictTools::new(&d);
        let obs = tools.query(HASH_ADD, None, 0);
        match obs {
            Observation::Candidates(c) => {
                assert_eq!(c, vec![HASH_ADD.to_string()]);
            }
            other => panic!("expected Candidates, got {other:?}"),
        }
    }

    #[test]
    fn query_by_kind_substring_matches_word_field() {
        let d = NomDict::open_in_memory().unwrap();
        seed_word(&d, HASH_ADD, "add", "function");
        seed_word(&d, HASH_MUL, "multiply", "function");
        seed_word(
            &d,
            "c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2",
            "add_vec",
            "function",
        );
        let tools = DictTools::new(&d);
        let obs = tools.query("add", Some("function"), 0);
        match obs {
            Observation::Candidates(c) => {
                // Both "add" and "add_vec" contain the needle; sorted ascending.
                assert_eq!(c.len(), 2);
                assert_eq!(c[0], HASH_ADD);
                assert!(c[1].starts_with("c2c2"));
            }
            other => panic!("expected Candidates, got {other:?}"),
        }
    }

    #[test]
    fn query_with_no_kind_and_non_hash_subject_returns_empty() {
        let d = NomDict::open_in_memory().unwrap();
        seed_word(&d, HASH_ADD, "add", "function");
        let tools = DictTools::new(&d);
        let obs = tools.query("add", None, 0);
        match obs {
            Observation::Candidates(c) => assert!(c.is_empty()),
            other => panic!("expected Candidates, got {other:?}"),
        }
    }

    #[test]
    fn compose_verify_are_explicit_stubs() {
        let d = NomDict::open_in_memory().unwrap();
        let tools = DictTools::new(&d);
        for obs in [tools.compose("anything", &[]), tools.verify("anything")] {
            match obs {
                Observation::Error(msg) => {
                    assert!(msg.contains("not yet wired"));
                }
                other => panic!("expected Error stub, got {other:?}"),
            }
        }
    }

    #[test]
    fn render_leaf_uid_produces_single_entry_plan_hash() {
        let d = NomDict::open_in_memory().unwrap();
        seed_word(&d, HASH_ADD, "add", "function");
        let tools = DictTools::new(&d);
        let obs = tools.render(HASH_ADD, "llvm-native");
        match obs {
            Observation::Rendered { target, bytes_hash } => {
                assert_eq!(target, "llvm-native");
                assert_eq!(bytes_hash.len(), 64, "plan hash must be SHA-256 hex");
                assert!(bytes_hash.chars().all(|c| c.is_ascii_hexdigit()));
            }
            other => panic!("expected Rendered, got {other:?}"),
        }
    }

    #[test]
    fn render_is_deterministic_across_calls() {
        let d = NomDict::open_in_memory().unwrap();
        seed_word(&d, HASH_ADD, "add", "function");
        let tools = DictTools::new(&d);
        let obs1 = tools.render(HASH_ADD, "llvm-native");
        let obs2 = tools.render(HASH_ADD, "llvm-native");
        match (&obs1, &obs2) {
            (
                Observation::Rendered { bytes_hash: h1, .. },
                Observation::Rendered { bytes_hash: h2, .. },
            ) => assert_eq!(h1, h2),
            _ => panic!("both calls must succeed"),
        }
    }

    #[test]
    fn render_target_tag_round_trips_without_affecting_hash() {
        let d = NomDict::open_in_memory().unwrap();
        seed_word(&d, HASH_ADD, "add", "function");
        let tools = DictTools::new(&d);
        let native = tools.render(HASH_ADD, "llvm-native");
        let wasm = tools.render(HASH_ADD, "wasm");
        match (&native, &wasm) {
            (
                Observation::Rendered { target: t1, bytes_hash: h1 },
                Observation::Rendered { target: t2, bytes_hash: h2 },
            ) => {
                assert_eq!(t1, "llvm-native");
                assert_eq!(t2, "wasm");
                // Same closure → same plan hash even across targets;
                // the agent uses `target` for its own bookkeeping only.
                assert_eq!(h1, h2);
            }
            _ => panic!("both calls must succeed"),
        }
    }

    #[test]
    fn render_walks_composed_of_and_differs_on_extra_child() {
        let d = NomDict::open_in_memory().unwrap();
        // Seed a leaf and a composite entry whose composed_of references it.
        seed_word(&d, HASH_ADD, "add", "function");
        d.upsert_word_v2(&WordV2Row {
            hash: HASH_MUL.into(),
            word: "arith".into(),
            kind: "module".into(),
            signature: None,
            contracts: None,
            body_kind: Some("module".into()),
            body_size: None,
            origin_ref: None,
            bench_ids: None,
            authored_in: None,
            composed_of: Some(format!("[\"{HASH_ADD}\"]")),
        })
        .unwrap();
        let tools = DictTools::new(&d);
        let leaf = tools.render(HASH_ADD, "t");
        let composite = tools.render(HASH_MUL, "t");
        match (&leaf, &composite) {
            (
                Observation::Rendered { bytes_hash: h_leaf, .. },
                Observation::Rendered { bytes_hash: h_comp, .. },
            ) => {
                assert_ne!(
                    h_leaf, h_comp,
                    "composite closure includes its child, so hash must differ from leaf"
                );
            }
            _ => panic!("both calls must succeed"),
        }
    }

    #[test]
    fn render_rejects_bad_uid() {
        let d = NomDict::open_in_memory().unwrap();
        let tools = DictTools::new(&d);
        match tools.render("not-a-hash", "llvm-native") {
            Observation::Error(msg) => assert!(msg.contains("not a 64-char hex hash")),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn render_missing_uid_reports_not_found() {
        let d = NomDict::open_in_memory().unwrap();
        let tools = DictTools::new(&d);
        match tools.render(HASH_ADD, "t") {
            Observation::Error(msg) => assert!(msg.contains("not found in dict")),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn explain_by_hash_emits_summary() {
        let d = NomDict::open_in_memory().unwrap();
        seed_word(&d, HASH_ADD, "add", "function");
        let tools = DictTools::new(&d);
        let obs = tools.explain(HASH_ADD, 0);
        match obs {
            Observation::Explanation { summary } => {
                assert!(summary.starts_with("add@"));
                assert!(summary.contains("kind=function"));
            }
            other => panic!("expected Explanation, got {other:?}"),
        }
    }

    #[test]
    fn explain_depth_1_adds_signature_contracts_origin() {
        let d = NomDict::open_in_memory().unwrap();
        seed_word(&d, HASH_ADD, "add", "function");
        let tools = DictTools::new(&d);
        let obs = tools.explain(HASH_ADD, 1);
        match obs {
            Observation::Explanation { summary } => {
                assert!(summary.contains("signature:"));
                assert!(summary.contains("contracts:"));
                assert!(summary.contains("authored_in:"));
            }
            other => panic!("expected Explanation, got {other:?}"),
        }
    }

    #[test]
    fn explain_rejects_bad_uid() {
        let d = NomDict::open_in_memory().unwrap();
        let tools = DictTools::new(&d);
        let obs = tools.explain("not-a-hash", 0);
        match obs {
            Observation::Error(msg) => assert!(msg.contains("not a 64-char hex hash")),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn explain_missing_uid_reports_not_found() {
        let d = NomDict::open_in_memory().unwrap();
        let tools = DictTools::new(&d);
        // Valid hex, but no row seeded.
        let obs = tools.explain(HASH_ADD, 0);
        match obs {
            Observation::Error(msg) => assert!(msg.contains("not found in dict")),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn max_results_truncates_result_set() {
        let d = NomDict::open_in_memory().unwrap();
        // Seed 5 words of the same kind, all containing "foo".
        for i in 0..5 {
            let hash = format!("{:0>64}", format!("{i:x}"));
            seed_word(&d, &hash, &format!("foo_{i}"), "function");
        }
        let tools = DictTools::new(&d).with_max_results(2);
        let obs = tools.query("foo", Some("function"), 0);
        match obs {
            Observation::Candidates(c) => {
                assert_eq!(c.len(), 2, "must truncate to max_results");
            }
            other => panic!("expected Candidates, got {other:?}"),
        }
    }

    #[test]
    fn invalid_hash_length_falls_through_to_kind_search() {
        let d = NomDict::open_in_memory().unwrap();
        seed_word(&d, HASH_ADD, "add", "function");
        let tools = DictTools::new(&d);
        // 63 hex chars — not a valid 64-char hash, treat as kind search needle.
        let subject = "a".repeat(63);
        let obs = tools.query(&subject, Some("function"), 0);
        match obs {
            Observation::Candidates(c) => assert!(c.is_empty()),
            other => panic!("expected Candidates, got {other:?}"),
        }
    }
}
