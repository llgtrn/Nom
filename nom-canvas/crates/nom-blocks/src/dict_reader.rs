#![deny(unsafe_code)]
use crate::block_model::NomtuRef;

#[derive(Clone, Debug)]
pub struct ClauseShape {
    pub name: String,
    pub grammar_shape: String,
    pub is_required: bool,
    pub description: String,
}

/// Trait injection — nom-blocks never opens SQLite directly.
/// Wave B uses StubDictReader; Wave C swaps in SqliteDictReader from nom-compiler-bridge.
pub trait DictReader: Send + Sync {
    fn is_known_kind(&self, kind: &str) -> bool;
    fn clause_shapes_for(&self, kind: &str) -> Vec<ClauseShape>;
    fn lookup_entity(&self, word: &str, kind: &str) -> Option<NomtuRef>;
}
