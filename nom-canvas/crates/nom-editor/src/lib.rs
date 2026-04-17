//! nom-editor — rope-based editor for code + docs.

#![deny(unsafe_code)]

pub mod anchor;
pub mod buffer;
pub mod commands;
pub mod completion;
pub mod cursor;
pub mod display_map;
pub mod editing;
pub mod highlight;
pub mod hints;
pub mod inlay_hints;
pub mod input;
pub mod line_layout;
pub mod lsp_bridge;
pub mod movement;
pub mod selection;
pub mod selections_collection;
pub mod syntax_map;
pub mod tab_map;
pub mod wrap_map;

pub use buffer::Buffer;
pub use cursor::{Cursor, CursorSet};
