#![deny(unsafe_code)]
pub mod buffer;
pub mod clipboard;
pub mod commands;
pub mod completion;
pub mod cursor;
pub mod display_map;
pub mod find_replace;
pub mod highlight;
pub mod hints;
pub mod indent;
pub mod input;
pub mod line_layout;
pub mod lsp_bridge;
pub mod scroll;
pub mod tab_map;
pub mod wrap_map;

pub use buffer::{Buffer, BufferId, Patch};
pub use clipboard::Clipboard;
pub use completion::CompletionMenu;
pub use cursor::{Anchor, Bias, CursorSet, Selection};
pub use find_replace::FindState;
pub use highlight::{HighlightSpan, Highlighter, SpanColor, TokenRole};
pub use hints::{HintKind, InlayHint, InlayHintProvider};
pub use input::{ActionRegistry, ImeState, KeyAction, KeyBinding, KeyCode};
pub use lsp_bridge::{
    CompletionItem, CompletionKind, HoverResult, Location, LspProvider, StubLspProvider,
};
pub use scroll::ScrollPosition;
