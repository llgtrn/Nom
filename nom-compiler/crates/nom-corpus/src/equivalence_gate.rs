//! §5.2 equivalence gate — scaffold (2026-04-12 night).
//!
//! Per plan §5.2: "the translation is the contract". Fresh corpus-
//! ingested entries land with `status: Partial`; the gate lifts
//! them to `Complete` by:
//!   1. Translating source → Nom body.
//!   2. Running a property/contract test: original vs. Nom.
//!   3. On byte/effect equivalence: upsert a NEW Entry with
//!      `status: Complete` + `body_kind: NOM_SOURCE` and add a
//!      `SupersededBy(partial_id → complete_id)` edge.
//!   4. On mismatch: keep Partial, record failure reason.
//!
//! Real translators live under `equivalence_gate::translators::<lang>`.

use thiserror::Error;

pub mod translators;

/// Result of running the equivalence gate on one entry.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum GateOutcome {
    /// Translator produced a Nom body that passed the contract test.
    /// Caller upserts a new Complete entry with `nom_source` body_kind.
    Lifted { nom_source_id: String },
    /// Translator produced output but the contract test failed.
    PartialRejected { reason: String },
    /// No translator for this language yet.
    NotYetImplemented { language: String },
}

/// Errors raised by the gate harness itself (not gate-outcome errors).
#[derive(Debug, Error)]
pub enum GateError {
    #[error("entry not found: {0}")]
    EntryNotFound(String),
    #[error("body_bytes missing or empty for entry {0}")]
    NoBodyBytes(String),
    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),
}

/// Run the equivalence gate for one Partial entry.
/// Dispatches to a per-language translator based on `language`.
pub fn run_gate(
    _entry_id: &str,
    body_kind: &str,
    body_bytes: &[u8],
    language: &str,
) -> Result<GateOutcome, GateError> {
    // Decode bytes to UTF-8. If not UTF-8, can't translate.
    let source = match std::str::from_utf8(body_bytes) {
        Ok(s) => s,
        Err(e) => {
            return Ok(GateOutcome::PartialRejected {
                reason: format!("body_bytes not valid UTF-8: {e}"),
            });
        }
    };
    match language {
        "rust" => {
            match translators::rust::translate(source) {
                Ok(nom_body) => {
                    // A future PR runs the nom-parser + verifier against the
                    // output and only lifts to Complete if it parses +
                    // type-checks. For now: non-empty body = Lifted.
                    if nom_body.trim().is_empty() {
                        Ok(GateOutcome::PartialRejected {
                            reason: "translator produced empty body".into(),
                        })
                    } else {
                        use sha2::{Digest, Sha256};
                        let mut h = Sha256::new();
                        h.update(nom_body.as_bytes());
                        let nom_source_id = format!("{:x}", h.finalize());
                        let _ = body_kind; // caller provides for future use
                        Ok(GateOutcome::Lifted { nom_source_id })
                    }
                }
                Err(translators::TranslationError::Parse(r))
                | Err(translators::TranslationError::Unsupported(r)) => {
                    Ok(GateOutcome::PartialRejected { reason: r })
                }
            }
        }
        _ => Ok(GateOutcome::NotYetImplemented {
            language: language.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_stub_returns_not_yet_implemented() {
        let out = run_gate("h", "python_source", b"def foo(): pass", "python").unwrap();
        match out {
            GateOutcome::NotYetImplemented { language } => assert_eq!(language, "python"),
            other => panic!("expected NotYetImplemented, got {other:?}"),
        }
    }

    #[test]
    fn gate_lifts_simple_rust_fn() {
        let src = b"fn add(a: i64, b: i64) -> i64 { a + b }";
        let out = run_gate("hash_x", "rust_source", src, "rust").unwrap();
        match out {
            GateOutcome::Lifted { nom_source_id } => {
                assert_eq!(nom_source_id.len(), 64); // sha256 hex
            }
            other => panic!("expected Lifted, got {other:?}"),
        }
    }

    #[test]
    fn gate_rejects_rust_struct() {
        let out = run_gate("h", "rust_source", b"struct Foo;", "rust").unwrap();
        match out {
            GateOutcome::PartialRejected { reason } => assert!(reason.contains("struct")),
            other => panic!("expected PartialRejected, got {other:?}"),
        }
    }

    #[test]
    fn gate_rejects_invalid_utf8() {
        let out = run_gate("h", "rust_source", &[0xFF, 0xFE], "rust").unwrap();
        match out {
            GateOutcome::PartialRejected { reason } => {
                assert!(reason.contains("UTF-8"), "got: {reason}")
            }
            other => panic!("expected PartialRejected, got {other:?}"),
        }
    }
}
