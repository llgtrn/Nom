//! nom-diagnostics: Error reporting for the Nom compiler.
//!
//! Uses the `ariadne` crate for beautiful terminal diagnostics with
//! source highlighting and inline suggestions.

use ariadne::{Color, ColorGenerator, Fmt, Label, Report, ReportKind, Source};
use nom_ast::Span;
use std::ops::Range;

/// Severity level of a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Error,
    Warning,
    Note,
    Help,
}

impl Level {
    fn report_kind(&self) -> ReportKind<'static> {
        match self {
            Level::Error => ReportKind::Error,
            Level::Warning => ReportKind::Warning,
            Level::Note | Level::Help => ReportKind::Advice,
        }
    }
}

/// A single label to highlight in the source.
#[derive(Debug, Clone)]
pub struct DiagLabel {
    pub span: Span,
    pub message: String,
}

/// A single diagnostic message.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub level: Level,
    /// Short code identifying this error class, e.g. `E001`.
    pub code: Option<String>,
    /// Primary message.
    pub message: String,
    /// File name / path for the source cache.
    pub file: String,
    /// Labels shown inline in the source listing.
    pub labels: Vec<DiagLabel>,
    /// Optional note appended after the listing.
    pub note: Option<String>,
    /// Optional suggested fix.
    pub help: Option<String>,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>, file: impl Into<String>, span: Span) -> Self {
        let message = message.into();
        let file = file.into();
        Self {
            level: Level::Error,
            code: None,
            message: message.clone(),
            file,
            labels: vec![DiagLabel { span, message }],
            note: None,
            help: None,
        }
    }

    pub fn warning(message: impl Into<String>, file: impl Into<String>, span: Span) -> Self {
        let message = message.into();
        let file = file.into();
        Self {
            level: Level::Warning,
            code: None,
            message: message.clone(),
            file,
            labels: vec![DiagLabel { span, message }],
            note: None,
            help: None,
        }
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn with_label(mut self, span: Span, msg: impl Into<String>) -> Self {
        self.labels.push(DiagLabel {
            span,
            message: msg.into(),
        });
        self
    }

    fn first_offset(&self) -> usize {
        self.labels.first().map(|l| l.span.start).unwrap_or(0)
    }

    fn span_range(s: &Span) -> Range<usize> {
        s.start..s.end.max(s.start + 1)
    }

    /// Emit this diagnostic to stderr using ariadne.
    pub fn emit(&self, source: &str) {
        let mut colors = ColorGenerator::new();
        let primary_color = match self.level {
            Level::Error => Color::Red,
            Level::Warning => Color::Yellow,
            Level::Note => Color::Cyan,
            Level::Help => Color::Green,
        };

        let kind = self.level.report_kind();
        let offset = self.first_offset();

        let mut builder = Report::build(kind, &self.file, offset).with_message(&self.message);

        if let Some(code) = &self.code {
            builder = builder.with_code(code);
        }

        for (i, label) in self.labels.iter().enumerate() {
            let color = if i == 0 { primary_color } else { colors.next() };
            builder = builder.with_label(
                Label::new((&self.file, Self::span_range(&label.span)))
                    .with_message(label.message.clone().fg(color))
                    .with_color(color),
            );
        }

        if let Some(note) = &self.note {
            builder = builder.with_note(note);
        }
        if let Some(help) = &self.help {
            builder = builder.with_help(help);
        }

        builder
            .finish()
            .eprint((&self.file, Source::from(source)))
            .expect("diagnostic write failed");
    }

    /// Format diagnostic to a string (for tests / non-tty output).
    pub fn to_string_report(&self, source: &str) -> String {
        let mut buf = Vec::new();
        let kind = self.level.report_kind();
        let offset = self.first_offset();

        let mut builder = Report::build(kind, &self.file, offset).with_message(&self.message);

        if let Some(code) = &self.code {
            builder = builder.with_code(code);
        }

        for label in &self.labels {
            builder = builder.with_label(
                Label::new((&self.file, Self::span_range(&label.span)))
                    .with_message(&label.message),
            );
        }

        if let Some(note) = &self.note {
            builder = builder.with_note(note);
        }

        builder
            .finish()
            .write((&self.file, Source::from(source)), &mut buf)
            .expect("diagnostic write failed");

        String::from_utf8_lossy(&buf).into_owned()
    }
}

/// A collection of diagnostics gathered during a compiler phase.
#[derive(Debug, Default)]
pub struct DiagnosticSink {
    pub diagnostics: Vec<Diagnostic>,
}

impl DiagnosticSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, d: Diagnostic) {
        self.diagnostics.push(d);
    }

    pub fn error(&mut self, message: impl Into<String>, file: impl Into<String>, span: Span) {
        self.push(Diagnostic::error(message, file, span));
    }

    pub fn warning(&mut self, message: impl Into<String>, file: impl Into<String>, span: Span) {
        self.push(Diagnostic::warning(message, file, span));
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.level == Level::Error)
    }

    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.level == Level::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.level == Level::Warning)
            .count()
    }

    /// Emit all diagnostics to stderr.
    pub fn emit_all(&self, source: &str) {
        for d in &self.diagnostics {
            d.emit(source);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom_ast::Span;

    #[test]
    fn create_error_diagnostic() {
        let span = Span::new(0, 5, 1, 1);
        let d = Diagnostic::error("type mismatch", "test.nom", span)
            .with_code("E001")
            .with_help("ensure both sides have the same type");
        assert_eq!(d.level, Level::Error);
        assert_eq!(d.code.as_deref(), Some("E001"));
    }

    #[test]
    fn sink_counts() {
        let mut sink = DiagnosticSink::new();
        let span = Span::new(0, 1, 1, 1);
        sink.error("e1", "f.nom", span);
        sink.warning("w1", "f.nom", span);
        assert!(sink.has_errors());
        assert_eq!(sink.error_count(), 1);
        assert_eq!(sink.warning_count(), 1);
    }

    #[test]
    fn to_string_report_smoke() {
        let span = Span::new(0, 4, 1, 1);
        let d = Diagnostic::error("unknown word", "test.nom", span);
        let out = d.to_string_report("hash");
        assert!(!out.is_empty());
    }
}
