#![deny(unsafe_code)]

pub mod block_config;
pub mod block_model;
pub mod block_schema;
pub mod block_selection;
pub mod block_transformer;
pub mod compose;
pub mod drawing;
pub mod embed;
pub mod flavour;
pub mod graph_node;
pub mod media;
pub mod nomx;
pub mod prose;
pub mod table;

// ── Public re-exports ────────────────────────────────────────────────────────

pub use block_model::{BlockComment, BlockId, BlockMeta, BlockModel};
pub use block_schema::{BlockSchema, Role, SchemaError};
pub use block_selection::{BlockSelection, SelectionSet};
pub use block_transformer::{BlockTransformer, Snapshot, TransformError};
pub use block_config::{BlockConfig, ConfigRegistry};
pub use flavour::Flavour;
pub use flavour::{
    CALLOUT, DRAWING, EMBED, GRAPH_NODE, MEDIA_ATTACHMENT, MEDIA_IMAGE, NOTE, NOMX, PROSE,
    SURFACE, TABLE,
};
pub use nomx::{NomxLang, NomxProps};
pub use prose::{ProseKind, ProseProps, TextAlign};
