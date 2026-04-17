pub mod terminal;
pub mod diagnostics;

pub use terminal::{TerminalPanel, TerminalLine, TerminalLineKind};
pub use diagnostics::{DiagnosticsPanel, Diagnostic, DiagnosticSeverity};
