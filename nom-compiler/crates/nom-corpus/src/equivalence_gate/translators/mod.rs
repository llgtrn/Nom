//! Per-language translators for §5.2 equivalence gate.
//!
//! Each submodule (rust, typescript, python, …) provides a
//! `translate(source: &str) -> Result<String, TranslationError>`
//! that returns a Nom-source body if translation succeeds, or an
//! error describing what was unsupported. `run_gate` dispatches
//! based on language.

pub mod rust;
pub mod typescript;

#[derive(Debug, thiserror::Error)]
pub enum TranslationError {
    #[error("parse error: {0}")]
    Parse(String),
    #[error("unsupported: {0}")]
    Unsupported(String),
}
