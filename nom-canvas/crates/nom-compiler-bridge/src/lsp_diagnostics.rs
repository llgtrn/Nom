//! LSP diagnostics request/response cycle for nom-compiler-bridge.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

impl DiagnosticSeverity {
    pub fn lsp_code(&self) -> u8 {
        match self {
            DiagnosticSeverity::Error => 1,
            DiagnosticSeverity::Warning => 2,
            DiagnosticSeverity::Information => 3,
            DiagnosticSeverity::Hint => 4,
        }
    }

    pub fn is_error(&self) -> bool {
        matches!(self, DiagnosticSeverity::Error)
    }
}

#[derive(Debug, Clone)]
pub struct LspDiagnostic {
    pub message: String,
    pub severity: DiagnosticSeverity,
    pub line: u32,
    pub character: u32,
    pub source: Option<String>,
}

impl LspDiagnostic {
    pub fn new(
        message: impl Into<String>,
        severity: DiagnosticSeverity,
        line: u32,
        character: u32,
    ) -> Self {
        Self {
            message: message.into(),
            severity,
            line,
            character,
            source: None,
        }
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn to_lsp_json(&self) -> String {
        let source_field = match &self.source {
            Some(s) => format!(r#","source":"{}""#, s),
            None => String::new(),
        };
        format!(
            r#"{{"message":"{}","severity":{},"range":{{"start":{{"line":{},"character":{}}}}}{}}}"#,
            self.message,
            self.severity.lsp_code(),
            self.line,
            self.character,
            source_field,
        )
    }
}

#[derive(Debug, Clone)]
pub struct DiagnosticsRequest {
    pub uri: String,
    pub version: u32,
}

impl DiagnosticsRequest {
    pub fn new(uri: impl Into<String>, version: u32) -> Self {
        Self {
            uri: uri.into(),
            version,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DiagnosticsResponse {
    pub uri: String,
    pub diagnostics: Vec<LspDiagnostic>,
}

impl DiagnosticsResponse {
    pub fn new(uri: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            diagnostics: Vec::new(),
        }
    }

    pub fn add(&mut self, diag: LspDiagnostic) {
        self.diagnostics.push(diag);
    }

    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity.is_error())
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| matches!(d.severity, DiagnosticSeverity::Warning))
            .count()
    }

    pub fn is_clean(&self) -> bool {
        self.error_count() == 0
    }

    pub fn to_publish_notification(&self) -> String {
        let diags: Vec<String> = self.diagnostics.iter().map(|d| d.to_lsp_json()).collect();
        format!(
            r#"{{"method":"textDocument/publishDiagnostics","params":{{"uri":"{}","diagnostics":[{}]}}}}"#,
            self.uri,
            diags.join(","),
        )
    }
}

#[cfg(test)]
mod lsp_diagnostics_tests {
    use super::*;

    #[test]
    fn diagnostic_severity_lsp_code() {
        assert_eq!(DiagnosticSeverity::Error.lsp_code(), 1);
        assert_eq!(DiagnosticSeverity::Warning.lsp_code(), 2);
        assert_eq!(DiagnosticSeverity::Information.lsp_code(), 3);
        assert_eq!(DiagnosticSeverity::Hint.lsp_code(), 4);
    }

    #[test]
    fn diagnostic_severity_is_error() {
        assert!(DiagnosticSeverity::Error.is_error());
        assert!(!DiagnosticSeverity::Warning.is_error());
        assert!(!DiagnosticSeverity::Information.is_error());
        assert!(!DiagnosticSeverity::Hint.is_error());
    }

    #[test]
    fn lsp_diagnostic_to_lsp_json_contains_message() {
        let diag = LspDiagnostic::new("undefined variable", DiagnosticSeverity::Error, 5, 3);
        let json = diag.to_lsp_json();
        assert!(json.contains("undefined variable"), "json: {}", json);
    }

    #[test]
    fn lsp_diagnostic_to_lsp_json_contains_severity_code() {
        let diag = LspDiagnostic::new("unused import", DiagnosticSeverity::Warning, 1, 0);
        let json = diag.to_lsp_json();
        assert!(json.contains("\"severity\":2"), "json: {}", json);
    }

    #[test]
    fn diagnostics_response_error_count() {
        let mut resp = DiagnosticsResponse::new("file:///test.nom");
        resp.add(LspDiagnostic::new("e1", DiagnosticSeverity::Error, 0, 0));
        resp.add(LspDiagnostic::new("w1", DiagnosticSeverity::Warning, 1, 0));
        resp.add(LspDiagnostic::new("e2", DiagnosticSeverity::Error, 2, 0));
        assert_eq!(resp.error_count(), 2);
    }

    #[test]
    fn diagnostics_response_warning_count() {
        let mut resp = DiagnosticsResponse::new("file:///test.nom");
        resp.add(LspDiagnostic::new("e1", DiagnosticSeverity::Error, 0, 0));
        resp.add(LspDiagnostic::new("w1", DiagnosticSeverity::Warning, 1, 0));
        resp.add(LspDiagnostic::new("w2", DiagnosticSeverity::Warning, 2, 0));
        assert_eq!(resp.warning_count(), 2);
    }

    #[test]
    fn is_clean_true_with_no_errors() {
        let mut resp = DiagnosticsResponse::new("file:///clean.nom");
        resp.add(LspDiagnostic::new("hint", DiagnosticSeverity::Hint, 0, 0));
        assert!(resp.is_clean());
    }

    #[test]
    fn is_clean_false_with_errors() {
        let mut resp = DiagnosticsResponse::new("file:///broken.nom");
        resp.add(LspDiagnostic::new("syntax error", DiagnosticSeverity::Error, 3, 5));
        assert!(!resp.is_clean());
    }

    #[test]
    fn to_publish_notification_contains_uri() {
        let resp = DiagnosticsResponse::new("file:///src/main.nom");
        let notification = resp.to_publish_notification();
        assert!(
            notification.contains("file:///src/main.nom"),
            "notification: {}",
            notification
        );
    }
}
