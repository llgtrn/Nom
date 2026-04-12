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
    Lifted {
        nom_source_id: String,
        /// UTF-8 bytes of the translated Nom body.
        nom_body: Vec<u8>,
    },
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

/// Shared post-translation step: compute sha256, return Lifted or PartialRejected.
fn lift_from_translator(result: Result<String, translators::TranslationError>) -> GateOutcome {
    match result {
        Ok(nom_body) => {
            if nom_body.trim().is_empty() {
                GateOutcome::PartialRejected {
                    reason: "translator produced empty body".into(),
                }
            } else {
                use sha2::{Digest, Sha256};
                let mut h = Sha256::new();
                h.update(nom_body.as_bytes());
                let nom_source_id = format!("{:x}", h.finalize());
                GateOutcome::Lifted {
                    nom_source_id,
                    nom_body: nom_body.into_bytes(),
                }
            }
        }
        Err(translators::TranslationError::Parse(r))
        | Err(translators::TranslationError::Unsupported(r)) => {
            GateOutcome::PartialRejected { reason: r }
        }
    }
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
    let _ = body_kind; // caller provides for future use
    match language {
        "rust" => Ok(lift_from_translator(translators::rust::translate(source))),
        "typescript" => {
            Ok(lift_from_translator(translators::typescript::translate(source)))
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
            GateOutcome::Lifted { nom_source_id, nom_body } => {
                assert_eq!(nom_source_id.len(), 64); // sha256 hex
                assert!(!nom_body.is_empty());
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

    #[test]
    fn run_gate_returns_nom_body_for_lifted() {
        let src = b"fn add(a: i64, b: i64) -> i64 { a + b }";
        let out = run_gate("hash_x", "rust_source", src, "rust").unwrap();
        match out {
            GateOutcome::Lifted { nom_source_id, nom_body } => {
                assert_eq!(nom_source_id.len(), 64);
                let body_str = std::str::from_utf8(&nom_body).expect("nom_body must be valid UTF-8");
                assert!(!body_str.trim().is_empty(), "nom_body must be non-empty");
            }
            other => panic!("expected Lifted, got {other:?}"),
        }
    }
}
