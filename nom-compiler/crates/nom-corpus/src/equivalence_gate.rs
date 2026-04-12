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
//! This module is stub-only. Real translators land per-language
//! (rust.rs, typescript.rs, python.rs, …) as separate PRs.

use thiserror::Error;

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

/// Run the equivalence gate for one Partial entry. Currently always
/// returns `NotYetImplemented`. Real implementations land per-
/// language under `equivalence_gate::translators::<lang>`.
pub fn run_gate(
    _entry_id: &str,
    _body_kind: &str,
    _body_bytes: &[u8],
    language: &str,
) -> Result<GateOutcome, GateError> {
    Ok(GateOutcome::NotYetImplemented {
        language: language.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_stub_returns_not_yet_implemented() {
        let out = run_gate("h", "rust_source", b"fn main() {}", "rust").unwrap();
        match out {
            GateOutcome::NotYetImplemented { language } => assert_eq!(language, "rust"),
            other => panic!("expected NotYetImplemented, got {other:?}"),
        }
    }
}
