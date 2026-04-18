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
}
