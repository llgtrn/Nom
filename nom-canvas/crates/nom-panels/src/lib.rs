#![deny(unsafe_code)]
pub mod dock;
pub mod left;
pub mod pane;
pub mod right;
pub mod shell;
pub mod bottom;
pub mod command_palette;
pub mod toolbar;
pub mod statusbar;

pub use dock::{Dock, DockPosition, Panel, PanelEntry, PanelSizeState, rgba_to_hsla, fill_quad, focus_ring_quad};
pub use left::{FileTreePanel, QuickSearchPanel, FileNode, FileNodeKind, SearchResult, SearchResultKind, NodePalette, PaletteEntry, LibraryPanel, LibraryKind};
pub use pane::{Pane, PaneAxis, PaneGroup, PaneTab, Member, SplitDirection};
pub use right::{ChatSidebarPanel, ChatMessage, ChatRole, ToolCard, DeepThinkPanel, ThinkingStep, PropertiesPanel, PropertyRow};
pub use shell::{Shell, ShellLayout, ShellMode};
pub use bottom::{TerminalPanel, TerminalLine, TerminalLineKind, DiagnosticsPanel, Diagnostic, DiagnosticSeverity};
pub use command_palette::{CommandPalette, CommandPaletteItem};
pub use toolbar::{Toolbar, ToolbarButton};
pub use statusbar::{StatusBar, StatusSlot};
