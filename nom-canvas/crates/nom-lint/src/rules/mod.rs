//! Concrete lint rules bundled with the linter.
#![deny(unsafe_code)]

pub mod max_line_length;
pub mod no_double_blank_lines;
pub mod no_trailing_whitespace;

pub use max_line_length::MaxLineLength;
pub use no_double_blank_lines::NoDoubleBlankLines;
pub use no_trailing_whitespace::NoTrailingWhitespace;
