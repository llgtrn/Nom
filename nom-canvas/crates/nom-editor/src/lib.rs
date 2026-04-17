//! nom-editor — rope-based editor for code + docs.

#![deny(unsafe_code)]

pub mod anchor;
pub mod buffer;
pub mod completion;
pub mod cursor;
pub mod editing;
pub mod highlight;
pub mod hints;
pub mod input;
pub mod movement;
pub mod selection;
pub mod selections_collection;

pub use buffer::Buffer;
pub use cursor::{Cursor, CursorSet};
