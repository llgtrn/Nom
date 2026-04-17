//! nom-editor — rope-based editor for code + docs.

#![deny(unsafe_code)]

pub mod buffer;
pub mod completion;
pub mod cursor;
pub mod highlight;
pub mod hints;
pub mod input;

pub use buffer::Buffer;
pub use cursor::{Cursor, CursorSet};
