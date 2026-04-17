#![deny(unsafe_code)]
use std::collections::{HashMap, HashSet};
use crate::block_model::NomtuRef;
use crate::dict_reader::{ClauseShape, DictReader};

/// Wave B implementation: no DB required. Used in tests and until Wave C bridge exists.
pub struct StubDictReader {
    known_kinds: HashSet<String>,
    seed_shapes: HashMap<String, Vec<ClauseShape>>,
}

impl StubDictReader {
    pub fn new() -> Self {
        let mut reader = Self {
            known_kinds: HashSet::new(),
            seed_shapes: HashMap::new(),
        };
        // Seed with common AFFiNE block kinds + nom grammar kinds
        for kind in &["verb", "concept", "metric", "constraint", "workflow", "agent",
                       "noun", "relation", "attribute", "event", "document", "media"] {
            reader.known_kinds.insert(kind.to_string());
        }
        reader
    }

    pub fn with_kinds(kinds: &[&str]) -> Self {
        let mut reader = Self::new();
        for k in kinds {
            reader.known_kinds.insert(k.to_string());
        }
        reader
    }

    pub fn with_shapes(mut self, kind: &str, shapes: Vec<ClauseShape>) -> Self {
        self.seed_shapes.insert(kind.to_string(), shapes);
        self
    }
}

impl Default for StubDictReader {
    fn default() -> Self {
        Self::new()
    }
}

impl DictReader for StubDictReader {
    fn is_known_kind(&self, kind: &str) -> bool {
        self.known_kinds.contains(kind)
    }

    fn clause_shapes_for(&self, kind: &str) -> Vec<ClauseShape> {
        self.seed_shapes.get(kind).cloned().unwrap_or_else(|| {
            vec![
                ClauseShape { name: "input".into(), grammar_shape: "any".into(), is_required: false, description: "stub input".into() },
                ClauseShape { name: "output".into(), grammar_shape: "any".into(), is_required: false, description: "stub output".into() },
            ]
        })
    }

    fn lookup_entity(&self, word: &str, kind: &str) -> Option<NomtuRef> {
        if self.known_kinds.contains(kind) {
            Some(NomtuRef::new(format!("stub-{word}"), word, kind))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_dict_known_kinds() {
        let dict = StubDictReader::new();
        assert!(dict.is_known_kind("verb"));
        assert!(dict.is_known_kind("concept"));
        assert!(!dict.is_known_kind("unknown_alien_kind"));
    }

    #[test]
    fn stub_dict_lookup() {
        let dict = StubDictReader::new();
        let result = dict.lookup_entity("summarize", "verb");
        assert!(result.is_some());
        assert_eq!(result.unwrap().word, "summarize");
    }

    #[test]
    fn stub_dict_clause_shapes() {
        let dict = StubDictReader::new();
        let shapes = dict.clause_shapes_for("verb");
        assert!(!shapes.is_empty());
    }
}
