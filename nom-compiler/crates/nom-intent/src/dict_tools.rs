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

use crate::react::{AgentTools, Observation};

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

    fn render(&self, _uid: &str, _target: &str) -> Observation {
        Observation::Error("DictTools::render not yet wired (slice-3c)".into())
    }

    fn explain(&self, _uid: &str, _depth: usize) -> Observation {
        Observation::Error("DictTools::explain not yet wired (slice-3c)".into())
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
    fn compose_verify_render_explain_are_explicit_stubs() {
        let d = NomDict::open_in_memory().unwrap();
        let tools = DictTools::new(&d);
        for obs in [
            tools.compose("anything", &[]),
            tools.verify("anything"),
            tools.render("anything", "llvm-bc"),
            tools.explain("anything", 1),
        ] {
            match obs {
                Observation::Error(msg) => {
                    assert!(msg.contains("not yet wired"));
                }
                other => panic!("expected Error stub, got {other:?}"),
            }
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
