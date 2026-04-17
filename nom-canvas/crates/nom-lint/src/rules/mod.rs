//! Concrete lint rules bundled with the linter.
#![deny(unsafe_code)]

pub mod max_line_length;
pub mod no_double_blank_lines;
pub mod no_tab_indent_after_space;
pub mod no_trailing_whitespace;
pub mod require_trailing_newline;

pub use max_line_length::MaxLineLength;
pub use no_double_blank_lines::NoDoubleBlankLines;
pub use no_tab_indent_after_space::NoTabIndentAfterSpace;
pub use no_trailing_whitespace::NoTrailingWhitespace;
pub use require_trailing_newline::RequireTrailingNewline;
