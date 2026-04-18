/// Async-loop message architecture for LSP stdin/stdout wiring.
/// Uses only std — no tokio dependency required.

// ---- LspAsyncConfig ---------------------------------------------------------

/// Configuration for the async message-processing loop.
#[derive(Debug, Clone)]
pub struct LspAsyncConfig {
    /// Read/write buffer size in bytes.
    pub buffer_size: usize,
    /// When true, the loop terminates after receiving an empty-input batch.
    pub shutdown_on_empty: bool,
}

impl Default for LspAsyncConfig {
    fn default() -> Self {
        Self {
            buffer_size: 8192,
            shutdown_on_empty: false,
        }
    }
}

// ---- LspAsyncMessage --------------------------------------------------------

/// A parsed LSP JSON-RPC message.
#[derive(Debug, Clone, PartialEq)]
pub enum LspAsyncMessage {
    /// An incoming request: has both `id` and `method`.
    Request { id: u64, method: String, body: String },
    /// A server-to-client response: has `id` and `result`.
    Response { id: u64, body: String },
    /// A one-way notification: has `method` but no `id`.
    Notification { method: String, body: String },
}

// ---- LspAsyncLoop -----------------------------------------------------------

/// Stateless message-processing loop for LSP framing.
/// All methods are pure / synchronous — no tokio runtime required.
pub struct LspAsyncLoop {
    pub config: LspAsyncConfig,
}

impl LspAsyncLoop {
    pub fn new(config: LspAsyncConfig) -> Self {
        Self { config }
    }

    /// Parse a raw JSON string into an [`LspAsyncMessage`].
    ///
    /// Rules (in priority order):
    /// 1. Has numeric `"id"` **and** `"method"` → `Request`.
    /// 2. Has numeric `"id"` **and** `"result"` → `Response`.
    /// 3. Has `"method"` only → `Notification`.
    /// 4. Otherwise → `None`.
    pub fn parse_message(raw: &str) -> Option<LspAsyncMessage> {
        if raw.is_empty() {
            return None;
        }

        let id = Self::extract_id(raw);
        let method = Self::extract_str_field(raw, "method");
        let has_result = raw.contains("\"result\"");

        match (id, method, has_result) {
            (Some(id), Some(method), _) => Some(LspAsyncMessage::Request {
                id,
                method,
                body: raw.to_string(),
            }),
            (Some(id), None, true) => Some(LspAsyncMessage::Response {
                id,
                body: raw.to_string(),
            }),
            (None, Some(method), _) => Some(LspAsyncMessage::Notification {
                method,
                body: raw.to_string(),
            }),
            _ => None,
        }
    }

    /// Format a response with LSP `Content-Length` framing.
    pub fn format_response(id: u64, body: &str) -> String {
        let json = format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":{body}}}");
        format!("Content-Length: {}\r\n\r\n{}", json.len(), json)
    }

    /// Parse a batch of raw strings and return all successfully parsed messages.
    pub fn process_batch(messages: Vec<&str>) -> Vec<LspAsyncMessage> {
        messages
            .into_iter()
            .filter_map(Self::parse_message)
            .collect()
    }

    /// Returns `true` if `msg` is a Notification whose method is `"shutdown"` or
    /// `"exit"`.
    pub fn is_shutdown_requested(msg: &LspAsyncMessage) -> bool {
        match msg {
            LspAsyncMessage::Notification { method, .. } => {
                method == "shutdown" || method == "exit"
            }
            _ => false,
        }
    }

    // -- helpers --------------------------------------------------------------

    /// Extract a numeric `"id": <u64>` value from a JSON string.
    fn extract_id(raw: &str) -> Option<u64> {
        let needle = "\"id\":";
        raw.find(needle).and_then(|idx| {
            let rest = raw[idx + needle.len()..].trim_start();
            // Skip if the value is a string (starts with '"') or null/false/true
            if rest.starts_with('"') || rest.starts_with('n') || rest.starts_with('f') || rest.starts_with('t') {
                return None;
            }
            let end = rest
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(rest.len());
            rest[..end].parse().ok()
        })
    }

    /// Extract the string value of a JSON field like `"field":"value"`.
    fn extract_str_field(raw: &str, field: &str) -> Option<String> {
        let needle = format!("\"{}\":\"", field);
        raw.find(&needle).and_then(|idx| {
            let start = idx + needle.len();
            raw[start..].find('"').map(|end| raw[start..start + end].to_string())
        })
    }
}

// ---- Tests ------------------------------------------------------------------

#[cfg(test)]
mod lsp_async_tests {
    use super::*;

    // 1. LspAsyncConfig defaults
    #[test]
    fn test_lsp_async_config_defaults() {
        let cfg = LspAsyncConfig::default();
        assert_eq!(cfg.buffer_size, 8192);
        assert!(!cfg.shutdown_on_empty);
    }

    // 2. parse_message returns Request for request-shaped JSON
    #[test]
    fn test_parse_message_request() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
        let msg = LspAsyncLoop::parse_message(raw);
        assert!(msg.is_some());
        match msg.unwrap() {
            LspAsyncMessage::Request { id, method, .. } => {
                assert_eq!(id, 1);
                assert_eq!(method, "initialize");
            }
            other => panic!("expected Request, got {:?}", other),
        }
    }

    // 3. parse_message returns Response for response-shaped JSON
    #[test]
    fn test_parse_message_response() {
        let raw = r#"{"jsonrpc":"2.0","id":42,"result":{"capabilities":{}}}"#;
        let msg = LspAsyncLoop::parse_message(raw);
        assert!(msg.is_some());
        match msg.unwrap() {
            LspAsyncMessage::Response { id, .. } => {
                assert_eq!(id, 42);
            }
            other => panic!("expected Response, got {:?}", other),
        }
    }

    // 4. parse_message returns Notification for notification-shaped JSON
    #[test]
    fn test_parse_message_notification() {
        let raw = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{}}"#;
        let msg = LspAsyncLoop::parse_message(raw);
        assert!(msg.is_some());
        match msg.unwrap() {
            LspAsyncMessage::Notification { method, .. } => {
                assert_eq!(method, "textDocument/didOpen");
            }
            other => panic!("expected Notification, got {:?}", other),
        }
    }

    // 5. parse_message returns None for empty string
    #[test]
    fn test_parse_message_empty_returns_none() {
        assert!(LspAsyncLoop::parse_message("").is_none());
    }

    // 6. format_response has correct Content-Length header
    #[test]
    fn test_format_response_content_length() {
        let framed = LspAsyncLoop::format_response(7, "null");
        assert!(framed.starts_with("Content-Length: "));
        // Extract the declared length
        let after_prefix = &framed["Content-Length: ".len()..];
        let len_end = after_prefix.find('\r').expect("\\r present");
        let declared: usize = after_prefix[..len_end].parse().expect("numeric length");
        // Find the separator and measure actual body length
        let sep = "\r\n\r\n";
        let sep_pos = framed.find(sep).expect("separator present") + sep.len();
        let body = &framed[sep_pos..];
        assert_eq!(declared, body.len());
    }

    // 7. process_batch processes multiple messages
    #[test]
    fn test_process_batch_multiple() {
        let req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
        let notif = r#"{"jsonrpc":"2.0","method":"$/cancelRequest","params":{}}"#;
        let empty = "";
        let results = LspAsyncLoop::process_batch(vec![req, notif, empty]);
        assert_eq!(results.len(), 2);
    }

    // 8. is_shutdown_requested detects shutdown/exit
    #[test]
    fn test_is_shutdown_requested() {
        let shutdown = LspAsyncMessage::Notification {
            method: "shutdown".to_string(),
            body: "{}".to_string(),
        };
        let exit = LspAsyncMessage::Notification {
            method: "exit".to_string(),
            body: "{}".to_string(),
        };
        let other = LspAsyncMessage::Notification {
            method: "textDocument/didSave".to_string(),
            body: "{}".to_string(),
        };
        assert!(LspAsyncLoop::is_shutdown_requested(&shutdown));
        assert!(LspAsyncLoop::is_shutdown_requested(&exit));
        assert!(!LspAsyncLoop::is_shutdown_requested(&other));
    }
}
