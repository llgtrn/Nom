#![deny(unsafe_code)]
use crate::block_model::NomtuRef;
use crate::dict_reader::{ClauseShape, DictReader, GrammarKindRow};
use std::collections::{HashMap, HashSet};

/// Wave B implementation: no DB required. Used in tests and until Wave C bridge exists.
pub struct StubDictReader {
    known_kinds: HashSet<String>,
    seed_shapes: HashMap<String, Vec<ClauseShape>>,
}

impl StubDictReader {
    /// Construct a [`StubDictReader`] seeded with the standard set of grammar kinds.
    pub fn new() -> Self {
        let mut reader = Self {
            known_kinds: HashSet::new(),
            seed_shapes: HashMap::new(),
        };
        // Seed with common AFFiNE block kinds + nom grammar kinds
        for kind in &[
            "verb",
            "concept",
            "metric",
            "constraint",
            "workflow",
            "agent",
            "noun",
            "relation",
            "attribute",
            "event",
            "document",
            "media",
        ] {
            reader.known_kinds.insert(kind.to_string());
        }
        reader
    }

    /// Construct a [`StubDictReader`] with the standard seed plus additional kinds.
    pub fn with_kinds(kinds: &[&str]) -> Self {
        let mut reader = Self::new();
        for k in kinds {
            reader.known_kinds.insert(k.to_string());
        }
        reader
    }

    /// Add custom clause shapes for the given kind (builder pattern).
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

    fn list_kinds(&self) -> Vec<GrammarKindRow> {
        let mut rows: Vec<_> = self
            .known_kinds
            .iter()
            .map(|name| GrammarKindRow {
                name: name.clone(),
                description: format!("{name} grammar kind"),
            })
            .collect();
        rows.sort_by(|a, b| a.name.cmp(&b.name));
        rows
    }

    fn clause_shapes_for(&self, kind: &str) -> Vec<ClauseShape> {
        self.seed_shapes.get(kind).cloned().unwrap_or_else(|| {
            vec![
                ClauseShape {
                    name: "input".into(),
                    grammar_shape: "any".into(),
                    is_required: false,
                    description: "stub input".into(),
                },
                ClauseShape {
                    name: "output".into(),
                    grammar_shape: "any".into(),
                    is_required: false,
                    description: "stub output".into(),
                },
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

    #[test]
    fn stub_dict_lookup_unknown_kind_returns_none() {
        let dict = StubDictReader::new();
        let result = dict.lookup_entity("anything", "nonexistent_kind_xyz");
        assert!(result.is_none());
    }

    #[test]
    fn stub_dict_with_shapes_overrides_default() {
        let dict = StubDictReader::new().with_shapes(
            "verb",
            vec![ClauseShape {
                name: "custom_port".into(),
                grammar_shape: "prose".into(),
                is_required: true,
                description: "custom".into(),
            }],
        );
        let shapes = dict.clause_shapes_for("verb");
        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].name, "custom_port");
        assert_eq!(shapes[0].grammar_shape, "prose");
    }

    #[test]
    fn stub_dict_unknown_kind_returns_default_shapes() {
        let dict = StubDictReader::new();
        // For a kind with no seeded shapes, default shapes (input + output) are returned
        let shapes = dict.clause_shapes_for("no_seed_kind");
        assert_eq!(shapes.len(), 2);
        assert!(shapes.iter().any(|s| s.name == "input"));
        assert!(shapes.iter().any(|s| s.name == "output"));
    }

    #[test]
    fn stub_dict_with_kinds_adds_custom_kind() {
        let dict = StubDictReader::with_kinds(&["custom_kind", "another_kind"]);
        assert!(dict.is_known_kind("custom_kind"));
        assert!(dict.is_known_kind("another_kind"));
        // Base kinds also present
        assert!(dict.is_known_kind("verb"));
        assert!(!dict.is_known_kind("not_added_kind"));
    }

    #[test]
    fn stub_dict_lookup_entity_id_is_prefixed() {
        let dict = StubDictReader::new();
        let entity = dict.lookup_entity("summarize", "verb").unwrap();
        assert!(entity.id.starts_with("stub-"));
        assert_eq!(entity.word, "summarize");
        assert_eq!(entity.kind, "verb");
    }

    #[test]
    fn stub_dict_lists_known_kinds_sorted() {
        let dict = StubDictReader::with_kinds(&["zeta", "alpha"]);
        let rows = dict.list_kinds();
        assert!(rows.windows(2).all(|pair| pair[0].name <= pair[1].name));
        assert!(rows.iter().any(|row| row.name == "alpha"));
        assert!(rows.iter().any(|row| row.name == "zeta"));
    }

    /// Default StubDictReader contains the "noun" kind
    #[test]
    fn stub_dict_contains_noun() {
        let dict = StubDictReader::new();
        assert!(dict.is_known_kind("noun"));
    }

    /// Default StubDictReader contains the "event" kind
    #[test]
    fn stub_dict_contains_event() {
        let dict = StubDictReader::new();
        assert!(dict.is_known_kind("event"));
    }

    /// Default StubDictReader contains the "media" kind
    #[test]
    fn stub_dict_contains_media() {
        let dict = StubDictReader::new();
        assert!(dict.is_known_kind("media"));
    }

    /// Default StubDictReader contains the "workflow" kind
    #[test]
    fn stub_dict_contains_workflow() {
        let dict = StubDictReader::new();
        assert!(dict.is_known_kind("workflow"));
    }

    /// Default StubDictReader contains the "agent" kind
    #[test]
    fn stub_dict_contains_agent() {
        let dict = StubDictReader::new();
        assert!(dict.is_known_kind("agent"));
    }

    /// list_kinds descriptions contain the kind name
    #[test]
    fn stub_dict_list_kinds_descriptions_contain_name() {
        let dict = StubDictReader::new();
        for row in dict.list_kinds() {
            assert!(
                row.description.contains(&row.name),
                "description '{}' must contain kind name '{}'",
                row.description,
                row.name
            );
        }
    }

    /// with_shapes for a new kind that was not in known_kinds still returns those shapes
    #[test]
    fn stub_dict_with_shapes_for_unknown_kind_returns_shapes() {
        let dict = StubDictReader::new().with_shapes(
            "custom_new_kind",
            vec![ClauseShape {
                name: "slot_x".into(),
                grammar_shape: "number".into(),
                is_required: false,
                description: "custom slot".into(),
            }],
        );
        let shapes = dict.clause_shapes_for("custom_new_kind");
        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].name, "slot_x");
    }

    /// clause_shapes_for returns input and output for concept kind (default)
    #[test]
    fn stub_dict_concept_default_shapes() {
        let dict = StubDictReader::new();
        let shapes = dict.clause_shapes_for("concept");
        assert!(shapes.iter().any(|s| s.name == "input"));
        assert!(shapes.iter().any(|s| s.name == "output"));
    }

    /// StubDictReader with multiple custom kinds all are recognized
    #[test]
    fn stub_dict_with_kinds_multiple_are_known() {
        let dict = StubDictReader::with_kinds(&["kind_a", "kind_b", "kind_c"]);
        assert!(dict.is_known_kind("kind_a"));
        assert!(dict.is_known_kind("kind_b"));
        assert!(dict.is_known_kind("kind_c"));
    }

    /// lookup_entity returns NomtuRef with the provided word
    #[test]
    fn stub_dict_lookup_entity_returns_word() {
        let dict = StubDictReader::new();
        let r = dict.lookup_entity("compose", "verb").unwrap();
        assert_eq!(r.word, "compose");
    }

    /// clause_shapes_for metric kind returns default shapes
    #[test]
    fn stub_dict_metric_default_shapes() {
        let dict = StubDictReader::new();
        let shapes = dict.clause_shapes_for("metric");
        // metric has no custom shapes → default input + output
        assert!(!shapes.is_empty());
    }

    /// StubDictReader default is equivalent to new()
    #[test]
    fn stub_dict_default_same_as_new() {
        let d1 = StubDictReader::new();
        let d2 = StubDictReader::default();
        assert_eq!(d1.list_kinds().len(), d2.list_kinds().len());
    }
}
