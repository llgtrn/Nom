#![deny(unsafe_code)]
pub mod buffer;
pub mod cursor;
pub mod highlight;
pub mod input;
pub mod display_map;
pub mod wrap_map;
pub mod tab_map;
pub mod line_layout;
pub mod lsp_bridge;
pub mod hints;
pub mod completion;
pub mod scroll;
pub mod clipboard;
pub mod find_replace;
pub mod indent;
pub mod commands;

pub use buffer::{Buffer, BufferId, Patch};
pub use cursor::{Anchor, Bias, Selection, CursorSet};
pub use highlight::{Highlighter, HighlightSpan, TokenRole, SpanColor};
pub use input::{KeyAction, KeyBinding, KeyCode, ActionRegistry, ImeState};
pub use lsp_bridge::{LspProvider, StubLspProvider, CompletionItem, CompletionKind, HoverResult, Location};
pub use hints::{InlayHint, InlayHintProvider, HintKind};
pub use completion::CompletionMenu;
pub use scroll::ScrollPosition;
pub use clipboard::Clipboard;
pub use find_replace::FindState;
