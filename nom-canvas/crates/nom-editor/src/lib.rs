#![deny(unsafe_code)]
pub mod buffer;
pub mod clipboard;
pub mod multi_cursor;
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
pub mod syntax_highlight;
pub mod selection;
pub mod tab_map;
pub mod wrap_map;
pub mod workspace_rename;
pub mod hover_tooltip;
pub mod diagnostic_squiggle;

pub use buffer::{Buffer, BufferId, Patch};
pub use clipboard::Clipboard;
pub use completion::CompletionMenu;
pub use cursor::{Anchor, Bias, BufferHistory, CursorSet, CursorShape, EditorCursor, Selection};
pub use display_map::{FoldState, LineDisplayMap, LineFoldRegion};
pub use find_replace::FindState;
pub use highlight::{
    highlight_nom_source, HighlightSpan, Highlighter, SpanColor, SyntaxSpan, TokenClass, TokenRole,
};
pub use hints::{HintKind, InlayHint, InlayHintProvider};
pub use input::{ActionRegistry, ImeState, KeyAction, KeyBinding, KeyCode};
pub use lsp_bridge::{
    CompletionItem, CompletionKind, HoverResult, Location, LspProvider, StubLspProvider,
};
pub use scroll::ScrollPosition;
pub use selection::{SelectionAnchor, SelectionManager, SelectionRange};
pub use workspace_rename::{RenameOp, RenamePreview, RenameScope, WorkspaceRenamer};
pub use hover_tooltip::{TooltipKind, TooltipContent, TooltipAnchor, HoverTooltip, TooltipRenderer};
pub use diagnostic_squiggle::{DiagnosticSeverity, DiagnosticSpan, DiagnosticOverlay, SquiggleStyle};
pub mod go_to_def;
pub use go_to_def::{DefinitionKind, DefinitionLocation, DefinitionTarget, GoToDefRequest, GoToDefResolver};
pub mod rename_preview;
pub use rename_preview::{RenamePreviewKind, RenameChange, RenamePreviewModel, RenameConflict, RenameApplier};
pub mod multi_file_edit;
pub use multi_file_edit::{EditScope, MultiFileChange, MultiFileSession, MultiFileDiff, SessionApplier};
pub mod code_lens;
pub use code_lens::{CodeLensKind, CodeLens, CodeLensProvider, CodeLensOverlay, LensResolver};
pub mod breadcrumb;
pub use breadcrumb::{BreadcrumbKind, BreadcrumbSegment, BreadcrumbPath, BreadcrumbNav, BreadcrumbRenderer};
pub mod outline_view;
pub use outline_view::{OutlineItemKind, OutlineItem, OutlineSection, OutlineTree, OutlineRenderer};
pub mod search_index;
pub use search_index::{SearchTokenKind, SearchToken, SearchIndex, SearchQuery, SearchResult};
pub mod format_range;
pub use format_range::{FormatKind, TextRange, FormatRange, FormatMap, FormatApplier};
