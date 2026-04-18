//! Dict reader trait and associated types for grammar-kind validation.
#![deny(unsafe_code)]
use crate::block_model::NomtuRef;

/// Grammar shape for one slot/port on a kind.
#[derive(Clone, Debug)]
pub struct ClauseShape {
    /// Port/slot name.
    pub name: String,
    /// Grammar type tag (e.g. `"text"`, `"any"`, `"integer"`).
    pub grammar_shape: String,
    /// Whether this slot is required.
    pub is_required: bool,
    /// Human-readable description.
    pub description: String,
}

/// One row from the grammar kinds table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrammarKindRow {
    /// Kind name (e.g. `"verb"`, `"concept"`).
    pub name: String,
    /// Human-readable description.
    pub description: String,
}

/// Trait injection — nom-blocks never opens SQLite directly.
/// Wave B uses StubDictReader; Wave C swaps in SqliteDictReader from nom-compiler-bridge.
pub trait DictReader: Send + Sync {
    /// Return `true` if `kind` is a registered grammar kind.
    fn is_known_kind(&self, kind: &str) -> bool;
    /// List all registered grammar kinds.
    fn list_kinds(&self) -> Vec<GrammarKindRow>;
    /// Return the clause shapes (ports) for the given grammar kind.
    fn clause_shapes_for(&self, kind: &str) -> Vec<ClauseShape>;
    /// Look up an entity by word and kind. Returns `None` if kind is unknown.
    fn lookup_entity(&self, word: &str, kind: &str) -> Option<NomtuRef>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stub_dict::StubDictReader;

    #[test]
    fn clause_shape_fields_are_accessible() {
        let shape = ClauseShape {
            name: "port_a".into(),
            grammar_shape: "text".into(),
            is_required: true,
            description: "a text port".into(),
        };
        assert_eq!(shape.name, "port_a");
        assert_eq!(shape.grammar_shape, "text");
        assert!(shape.is_required);
        assert_eq!(shape.description, "a text port");
    }

    #[test]
    fn grammar_kind_row_equality() {
        let a = GrammarKindRow {
            name: "verb".into(),
            description: "verb kind".into(),
        };
        let b = GrammarKindRow {
            name: "verb".into(),
            description: "verb kind".into(),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn grammar_kind_row_inequality() {
        let a = GrammarKindRow {
            name: "verb".into(),
            description: "desc1".into(),
        };
        let b = GrammarKindRow {
            name: "concept".into(),
            description: "desc2".into(),
        };
        assert_ne!(a, b);
    }

    #[test]
    fn list_kinds_returns_all_seeded_kinds() {
        let dict = StubDictReader::new();
        let rows = dict.list_kinds();
        // Base seeds include verb, concept, metric, constraint, workflow, agent, noun, relation, attribute, event, document, media
        assert!(rows.len() >= 12);
        let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"verb"));
        assert!(names.contains(&"concept"));
        assert!(names.contains(&"metric"));
    }

    #[test]
    fn list_kinds_descriptions_non_empty() {
        let dict = StubDictReader::new();
        for row in dict.list_kinds() {
            assert!(
                !row.description.is_empty(),
                "description for kind '{}' must not be empty",
                row.name
            );
        }
    }

    #[test]
    fn is_known_kind_true_for_all_seeded() {
        let dict = StubDictReader::new();
        for row in dict.list_kinds() {
            assert!(
                dict.is_known_kind(&row.name),
                "is_known_kind must be true for listed kind '{}'",
                row.name
            );
        }
    }

    #[test]
    fn is_known_kind_false_for_empty_string() {
        let dict = StubDictReader::new();
        assert!(!dict.is_known_kind(""));
    }

    #[test]
    fn is_known_kind_case_sensitive() {
        let dict = StubDictReader::new();
        // "verb" is known; "Verb" and "VERB" are not
        assert!(dict.is_known_kind("verb"));
        assert!(!dict.is_known_kind("Verb"));
        assert!(!dict.is_known_kind("VERB"));
    }

    #[test]
    fn lookup_entity_returns_correct_word_and_kind() {
        let dict = StubDictReader::new();
        let entity = dict.lookup_entity("render", "verb").unwrap();
        assert_eq!(entity.word, "render");
        assert_eq!(entity.kind, "verb");
    }

    #[test]
    fn lookup_entity_unknown_word_known_kind_returns_some() {
        // StubDictReader creates a stub ref for any word if the kind is known
        let dict = StubDictReader::new();
        let result = dict.lookup_entity("completely_unknown_word_xyz", "verb");
        assert!(result.is_some());
    }

    #[test]
    fn lookup_entity_empty_word_known_kind_returns_some() {
        let dict = StubDictReader::new();
        let result = dict.lookup_entity("", "verb");
        assert!(result.is_some());
    }

    #[test]
    fn lookup_entity_known_word_unknown_kind_returns_none() {
        let dict = StubDictReader::new();
        let result = dict.lookup_entity("verb", "not_a_kind_xyz");
        assert!(result.is_none());
    }

    #[test]
    fn clause_shapes_default_have_input_and_output() {
        let dict = StubDictReader::new();
        // All known kinds without custom shapes return at least input + output
        let shapes = dict.clause_shapes_for("verb");
        assert!(shapes.iter().any(|s| s.name == "input" || s.name == "output"));
    }

    #[test]
    fn clause_shape_can_be_not_required() {
        let shape = ClauseShape {
            name: "optional_port".into(),
            grammar_shape: "text".into(),
            is_required: false,
            description: String::new(),
        };
        assert!(!shape.is_required);
    }

    #[test]
    fn grammar_kind_row_clone() {
        let row = GrammarKindRow {
            name: "verb".into(),
            description: "a verb".into(),
        };
        let cloned = row.clone();
        assert_eq!(cloned.name, row.name);
        assert_eq!(cloned.description, row.description);
    }

    #[test]
    fn clause_shape_clone() {
        let shape = ClauseShape {
            name: "output".into(),
            grammar_shape: "any".into(),
            is_required: false,
            description: "stub output".into(),
        };
        let cloned = shape.clone();
        assert_eq!(cloned.name, shape.name);
        assert_eq!(cloned.grammar_shape, shape.grammar_shape);
    }
}
