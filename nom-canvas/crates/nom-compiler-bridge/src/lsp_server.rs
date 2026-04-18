/// LSP server stub — stdin/stdout JSON-RPC handshake protocol
/// Full implementation requires tokio; this provides the message types and dispatch logic.

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LspRequest {
    pub jsonrpc: String,
    pub id: Option<i64>,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LspResponse {
    pub jsonrpc: String,
    pub id: Option<i64>,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

impl LspResponse {
    pub fn ok(id: Option<i64>, result: serde_json::Value) -> Self {
        Self { jsonrpc: "2.0".into(), id, result: Some(result), error: None }
    }
    pub fn error(id: Option<i64>, msg: impl Into<String>) -> Self {
        Self { jsonrpc: "2.0".into(), id, result: None, error: Some(msg.into()) }
    }
}

/// Dispatch an LSP request to the appropriate handler
pub fn dispatch_lsp_request(req: &LspRequest) -> LspResponse {
    match req.method.as_str() {
        "initialize" => LspResponse::ok(req.id, serde_json::json!({
            "capabilities": {
                "hoverProvider": true,
                "completionProvider": { "triggerCharacters": ["."] },
                "definitionProvider": true,
                "referencesProvider": true,
                "renameProvider": true
            }
        })),
        "textDocument/hover" => LspResponse::ok(req.id, serde_json::json!({
            "contents": { "kind": "markdown", "value": "**Nom kind**" }
        })),
        "textDocument/completion" => LspResponse::ok(req.id, serde_json::json!({
            "items": []
        })),
        "textDocument/definition" => LspResponse::ok(req.id, serde_json::json!(null)),
        "textDocument/references" => LspResponse::ok(req.id, serde_json::json!([])),
        "workspace/symbol" => LspResponse::ok(req.id, serde_json::json!([])),
        "$/cancelRequest" => LspResponse::ok(None, serde_json::json!(null)),
        method => LspResponse::error(req.id, format!("method not found: {}", method)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_req(method: &str, id: Option<i64>) -> LspRequest {
        LspRequest {
            jsonrpc: "2.0".into(),
            id,
            method: method.into(),
            params: None,
        }
    }

    #[test]
    fn test_lsp_dispatch_initialize() {
        let req = make_req("initialize", Some(1));
        let resp = dispatch_lsp_request(&req);
        assert_eq!(resp.id, Some(1));
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert!(result["capabilities"]["hoverProvider"].as_bool().unwrap());
        assert!(result["capabilities"]["definitionProvider"].as_bool().unwrap());
    }

    #[test]
    fn test_lsp_dispatch_hover() {
        let req = make_req("textDocument/hover", Some(2));
        let resp = dispatch_lsp_request(&req);
        assert_eq!(resp.id, Some(2));
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert_eq!(result["contents"]["kind"].as_str().unwrap(), "markdown");
    }

    #[test]
    fn test_lsp_dispatch_unknown_method_errors() {
        let req = make_req("unknownMethod/foo", Some(99));
        let resp = dispatch_lsp_request(&req);
        assert_eq!(resp.id, Some(99));
        assert!(resp.result.is_none());
        let err = resp.error.unwrap();
        assert!(err.contains("method not found"));
        assert!(err.contains("unknownMethod/foo"));
    }

    #[test]
    fn test_lsp_response_ok_has_result() {
        let resp = LspResponse::ok(Some(5), serde_json::json!({"key": "value"}));
        assert_eq!(resp.jsonrpc, "2.0");
        assert_eq!(resp.id, Some(5));
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
        assert_eq!(resp.result.unwrap()["key"].as_str().unwrap(), "value");
    }

    // --- LspTransport tests ---

    #[test]
    fn transport_parse_header() {
        let data = b"Content-Length: 42\r\n\r\n";
        assert_eq!(LspTransport::parse_header(data), Some(42));
    }

    #[test]
    fn transport_parse_header_none() {
        let data = b"garbage";
        assert_eq!(LspTransport::parse_header(data), None);
    }

    #[test]
    fn transport_frame() {
        let payload = r#"{"id":1}"#;
        let framed = LspTransport::frame(payload);
        let framed_str = std::str::from_utf8(&framed).unwrap();
        assert!(framed_str.contains("Content-Length: 8"));
        assert!(framed_str.contains(r#"{"id":1}"#));
    }

    #[test]
    fn transport_try_read_message() {
        let payload = r#"{"jsonrpc":"2.0","id":1}"#;
        let framed = LspTransport::frame(payload);
        let mut transport = LspTransport::new();
        let result = transport.try_read_message(&framed);
        assert_eq!(result, Some(payload.to_string()));
    }

    // --- AuthoringProtocol tests ---

    #[test]
    fn authoring_protocol_new() {
        let proto = AuthoringProtocol::new();
        assert!(proto.events.is_empty());
        assert!(proto.current_file.is_none());
        assert_eq!(proto.event_count(), 0);
    }

    #[test]
    fn authoring_emit_and_event_count() {
        let proto = AuthoringProtocol::new()
            .emit(AuthoringEvent::FileChanged { path: "main.nom".into() })
            .emit(AuthoringEvent::ParseStarted { path: "main.nom".into() });
        assert_eq!(proto.event_count(), 2);
    }

    #[test]
    fn authoring_diagnostics_for() {
        let proto = AuthoringProtocol::new()
            .emit(AuthoringEvent::DiagnosticsReady {
                path: "a.nom".into(),
                diagnostics: vec!["error at line 1".into()],
            })
            .emit(AuthoringEvent::DiagnosticsReady {
                path: "b.nom".into(),
                diagnostics: vec![],
            });
        let diags = proto.diagnostics_for("a.nom");
        assert_eq!(diags.len(), 1);
        let diags_b = proto.diagnostics_for("b.nom");
        assert_eq!(diags_b.len(), 1);
        let diags_none = proto.diagnostics_for("c.nom");
        assert!(diags_none.is_empty());
    }

    #[test]
    fn authoring_has_errors() {
        let proto = AuthoringProtocol::new()
            .emit(AuthoringEvent::ParseCompleted { path: "x.nom".into(), error_count: 2 })
            .emit(AuthoringEvent::ParseCompleted { path: "y.nom".into(), error_count: 0 });
        assert!(proto.has_errors("x.nom"));
        assert!(!proto.has_errors("y.nom"));
    }
}

// ---- LSP transport (framed JSON-RPC over stdin/stdout) ----
// Header format: "Content-Length: N\r\n\r\n{json}"

/// Framed JSON-RPC transport for stdin/stdout LSP communication.
#[derive(Debug)]
pub struct LspTransport {
    pub buffer: Vec<u8>,
}

impl LspTransport {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    /// Parse Content-Length value from raw header bytes.
    /// Returns `Some(n)` when a valid `Content-Length: N\r\n\r\n` prefix is found.
    pub fn parse_header(data: &[u8]) -> Option<usize> {
        let text = std::str::from_utf8(data).ok()?;
        let prefix = "Content-Length: ";
        let start = text.find(prefix)?;
        let rest = &text[start + prefix.len()..];
        let end = rest.find('\r')?;
        rest[..end].trim().parse::<usize>().ok()
    }

    /// Frame a JSON payload with a Content-Length header.
    pub fn frame(payload: &str) -> Vec<u8> {
        let header = format!("Content-Length: {}\r\n\r\n", payload.len());
        let mut out = header.into_bytes();
        out.extend_from_slice(payload.as_bytes());
        out
    }

    /// Try to extract a complete JSON-RPC message from the incoming bytes.
    /// Appends `data` to the internal buffer, then attempts to parse one message.
    /// Consumes the message bytes from the buffer on success.
    pub fn try_read_message(&mut self, data: &[u8]) -> Option<String> {
        self.buffer.extend_from_slice(data);
        let header_end = {
            let windows = self.buffer.windows(4);
            let mut found = None;
            for (i, w) in windows.enumerate() {
                if w == b"\r\n\r\n" {
                    found = Some(i + 4);
                    break;
                }
            }
            found?
        };
        let content_length = Self::parse_header(&self.buffer[..header_end])?;
        let total = header_end + content_length;
        if self.buffer.len() < total {
            return None;
        }
        let message = std::str::from_utf8(&self.buffer[header_end..total]).ok()?.to_string();
        self.buffer.drain(..total);
        Some(message)
    }
}

impl Default for LspTransport {
    fn default() -> Self {
        Self::new()
    }
}

// ---- Authoring event stream ----

/// Events emitted during the edit-compile cycle.
#[derive(Debug, Clone, PartialEq)]
pub enum AuthoringEvent {
    FileChanged { path: String },
    ParseStarted { path: String },
    ParseCompleted { path: String, error_count: u32 },
    TypeCheckStarted { path: String },
    TypeCheckCompleted { path: String, error_count: u32 },
    DiagnosticsReady { path: String, diagnostics: Vec<String> },
}

/// Accumulates authoring events for a single edit-compile cycle.
#[derive(Debug)]
pub struct AuthoringProtocol {
    pub events: Vec<AuthoringEvent>,
    pub current_file: Option<String>,
}

impl AuthoringProtocol {
    pub fn new() -> Self {
        Self { events: Vec::new(), current_file: None }
    }

    /// Append an event and return `self` for builder-style chaining.
    pub fn emit(mut self, event: AuthoringEvent) -> Self {
        self.events.push(event);
        self
    }

    /// Set the active file and return `self`.
    pub fn set_current_file(mut self, path: &str) -> Self {
        self.current_file = Some(path.to_string());
        self
    }

    /// Return all `DiagnosticsReady` events for the given path.
    pub fn diagnostics_for(&self, path: &str) -> Vec<&AuthoringEvent> {
        self.events
            .iter()
            .filter(|e| matches!(e, AuthoringEvent::DiagnosticsReady { path: p, .. } if p == path))
            .collect()
    }

    /// Return `true` if any `ParseCompleted` or `TypeCheckCompleted` event for `path`
    /// has `error_count > 0`.
    pub fn has_errors(&self, path: &str) -> bool {
        self.events.iter().any(|e| match e {
            AuthoringEvent::ParseCompleted { path: p, error_count }
            | AuthoringEvent::TypeCheckCompleted { path: p, error_count } => {
                p == path && *error_count > 0
            }
            _ => false,
        })
    }

    /// Total number of events recorded.
    pub fn event_count(&self) -> usize {
        self.events.len()
    }
}

impl Default for AuthoringProtocol {
    fn default() -> Self {
        Self::new()
    }
}
