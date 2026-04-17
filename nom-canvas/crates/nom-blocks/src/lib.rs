#![deny(unsafe_code)]
pub mod block_model;
pub mod slot;
pub mod shared_types;
pub mod dict_reader;
pub mod stub_dict;
pub mod prose;
pub mod nomx;
pub mod graph_node;
pub mod connector;
pub mod validators;
pub mod media;
pub mod drawing;
pub mod table;
pub mod embed;
pub mod compose;
pub mod workspace;

pub use block_model::{BlockId, BlockModel, BlockMeta, NomtuRef};
pub use slot::{SlotValue, SlotBinding};
pub use shared_types::{DeepThinkStep, DeepThinkEvent, CompositionPlan, PlanStep, RunEvent};
pub use dict_reader::{ClauseShape, DictReader};
pub use stub_dict::StubDictReader;
pub use graph_node::{GraphNode, NodeId};
pub use connector::{Connector, ConnectorId};
pub use workspace::{Workspace, CanvasObject};

#[cfg(test)]
mod integration_tests {
    use crate::connector::Connector;
    use crate::dict_reader::{ClauseShape, DictReader};
    use crate::stub_dict::StubDictReader;
    use crate::prose::HeadingBlock;
    use crate::block_model::NomtuRef;

    fn make_shape(name: &str, grammar_shape: &str) -> ClauseShape {
        ClauseShape {
            name: name.into(),
            grammar_shape: grammar_shape.into(),
            is_required: true,
            description: String::new(),
        }
    }

    /// Creates a Connector via new_with_validation() using StubDictReader,
    /// verifies can_wire returns true for known ports ("output" → "input").
    #[test]
    fn block_with_stub_dict_can_wire_validated() {
        let dict = StubDictReader::new();
        let connector = Connector::new_with_validation(
            "wire-1",
            "node-a", "output",
            "node-b", "input",
            &dict,
            "verb",
            "concept",
        );
        assert!(
            connector.can_wire_result.0,
            "Connector with known ports via StubDictReader must be valid"
        );
    }

    /// Creates a HeadingBlock (a ProseBlock variant), verifies that its entity
    /// field is a NomtuRef with a non-empty id.
    #[test]
    fn nomtu_ref_non_optional_on_block() {
        let entity = NomtuRef::new("heading-entity-01", "heading", "concept");
        let block = HeadingBlock {
            entity: entity.clone(),
            text: vec![],
            level: 1,
            children: vec![],
        };
        assert!(
            !block.entity.id.is_empty(),
            "entity.id must be non-empty — NomtuRef is non-optional on blocks"
        );
        assert_eq!(block.entity.id, "heading-entity-01");
    }

    /// Creates a "BlockSchema" for heading using the dict's clause_shapes,
    /// verifies clause_shapes has at least one entry.
    #[test]
    fn block_schema_validates_required_fields() {
        // BlockSchema is represented by the shapes returned by the DictReader for a kind.
        // We build a dict seeded with heading-specific shapes to confirm at least one entry.
        let dict = StubDictReader::new()
            .with_shapes(
                "heading",
                vec![
                    make_shape("text", "prose"),
                    make_shape("level", "integer"),
                ],
            );
        let clause_shapes = dict.clause_shapes_for("heading");
        assert!(
            !clause_shapes.is_empty(),
            "BlockSchema (clause_shapes) for 'heading' must have at least one entry"
        );
        // Verify the required shape is present
        assert!(
            clause_shapes.iter().any(|s| s.name == "text"),
            "heading BlockSchema must contain a 'text' clause shape"
        );
    }
}
