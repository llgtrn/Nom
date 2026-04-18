#![deny(unsafe_code)]
#![warn(missing_docs)]
//! Block primitives for the NomCanvas workspace — models, connectors, tables, dataviews.

/// AppManifest — Cargo-style dependency manifest for workspace .nomx manifests.
#[allow(missing_docs)]
pub mod app_manifest;
/// Ancestry depth tracking and transitive ancestor caching.
#[allow(missing_docs)]
pub mod ancestry;
/// FNV-1a content hashing and deduplicating content store.
#[allow(missing_docs)]
pub mod content_hash;
pub mod block_model;
/// Composition block types (app, audio, data, document, etc.).
#[allow(missing_docs)]
pub mod compose;
pub mod connector;
pub mod dataview;
pub mod dict_reader;
/// Block diff/patch: compute and apply structural differences between block lists.
#[allow(missing_docs)]
pub mod diff;
/// Drawing/shape block types.
#[allow(missing_docs)]
pub mod drawing;
/// Edgeless (free-floating) text block.
#[allow(missing_docs)]
pub mod edgeless_text;
/// Embed block types.
#[allow(missing_docs)]
pub mod embed;
/// Frame block: a named container for grouping child blocks.
#[allow(missing_docs)]
pub mod frame;
pub mod graph_node;
/// Block event history — undo/redo stack.
#[allow(missing_docs)]
pub mod history;
/// LaTeX formula block.
#[allow(missing_docs)]
pub mod latex;
/// Media block types.
#[allow(missing_docs)]
pub mod media;
/// Nom-source block types.
#[allow(missing_docs)]
pub mod nomx;
/// Prose block types (heading, paragraph, etc.).
#[allow(missing_docs)]
pub mod prose;
pub mod shared_types;
pub mod slot;
/// In-memory stub implementation of [`DictReader`] for tests and Wave B.
pub mod stub_dict;
pub mod table;
pub mod validators;
pub mod workspace;
/// WorkspaceSchema, SchemaVersion, SchemaMigration, and MigrationPlan for workspace versioning.
#[allow(missing_docs)]
pub mod workspace_schema;
/// Workspace manifest as .nomx AppManifest — NomxDep, NomxManifest, NomxModuleGraph.
#[allow(missing_docs)]
pub mod nomx_manifest;

pub use ancestry::{AncestorEntry, AncestryCache};
pub use content_hash::{ContentHash, ContentStore};
pub use block_model::{BlockId, BlockMeta, BlockModel, NomtuRef};
pub use connector::{Connector, ConnectorId};
pub use dict_reader::{ClauseShape, DictReader};
pub use graph_node::{GraphNode, NodeId};
pub use shared_types::{CompositionPlan, DeepThinkEvent, DeepThinkStep, PlanStep, RunEvent};
pub use slot::{SlotBinding, SlotValue};
pub use stub_dict::StubDictReader;
pub use workspace::{CanvasObject, Workspace};
pub use nomx_manifest::{NomxDep, NomxManifest, NomxModuleEdge, NomxModuleGraph};

#[cfg(test)]
mod integration_tests {
    use crate::block_model::NomtuRef;
    use crate::connector::Connector;
    use crate::dict_reader::{ClauseShape, DictReader};
    use crate::prose::HeadingBlock;
    use crate::stub_dict::StubDictReader;

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
        let connector = Connector::new_with_validation(crate::connector::ConnectorValidation {
            id: "wire-1".into(),
            from_node: "node-a".into(),
            from_port: "output".into(),
            to_node: "node-b".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        });
        assert!(
            connector.can_wire_result().0,
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
        let dict = StubDictReader::new().with_shapes(
            "heading",
            vec![make_shape("text", "prose"), make_shape("level", "integer")],
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
