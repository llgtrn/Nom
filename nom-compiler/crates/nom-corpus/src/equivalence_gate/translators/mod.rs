//! Per-language translators for §5.2 equivalence gate.
//!
//! Each submodule (rust, typescript, python, …) provides a
//! `translate(source: &str) -> Result<Vec<TranslatedItem>, TranslationError>`
//! that returns one item per top-level declaration if translation succeeds,
//! or an error describing a file-level parse failure. Item-level rejection
//! (unsupported generics, async, etc.) silently drops that item and continues.
//! `run_gate` dispatches based on language.

pub mod rust;
pub mod typescript;

#[derive(Debug, thiserror::Error)]
pub enum TranslationError {
    #[error("parse error: {0}")]
    Parse(String),
    #[error("unsupported: {0}")]
    Unsupported(String),
}

/// One successfully-translated top-level declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct TranslatedItem {
    /// Identifier of the translated item (fn name, const name, etc.).
    /// Used as the `word` column in the dictionary.
    pub name: String,
    /// Short summary — fn signature line e.g. `"fn add(a: integer, b: integer) -> integer"`.
    /// Prepended to `describe`.
    pub summary: String,
    /// Full translated Nom body (what gets compiled to .bc).
    pub nom_body: String,
}
