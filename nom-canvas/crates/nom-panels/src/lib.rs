#![deny(unsafe_code)]
pub mod dock;
pub mod left;
pub mod pane;
pub mod right;
pub mod shell;
pub mod bottom;

pub use dock::{Dock, DockPosition, Panel, PanelEntry, PanelSizeState, RenderPrimitive};
pub use left::{FileTreePanel, QuickSearchPanel, FileNode, FileNodeKind, SearchResult, SearchResultKind};
pub use pane::{Pane, PaneAxis, PaneGroup, PaneTab, Member, SplitDirection};
pub use right::{ChatSidebarPanel, ChatMessage, ChatRole, ToolCard, DeepThinkPanel, ThinkingStep};
pub use shell::{Shell, ShellLayout, ShellMode};
pub use bottom::{TerminalPanel, TerminalLine, TerminalLineKind, DiagnosticsPanel, Diagnostic, DiagnosticSeverity};
