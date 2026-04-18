use serde::{Deserialize, Serialize};

/// Request body for `POST /compose`.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ComposeRequest {
    pub source: String,
    pub format: String,
    pub output_path: Option<String>,
}

/// Response body for `POST /compose`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComposeResponse {
    pub success: bool,
    pub message: String,
    pub output_path: Option<String>,
    pub nomx: Option<String>,
}

/// Pure business logic — no axum dependency so it is always compiled and testable.
pub fn compose_logic(req: ComposeRequest) -> ComposeResponse {
    if req.source.is_empty() {
        return ComposeResponse {
            success: false,
            message: "Parse error: source must not be empty".to_string(),
            output_path: req.output_path,
            nomx: None,
        };
    }

    let valid_formats = ["nomx", "json", "text"];
    if !valid_formats.contains(&req.format.as_str()) {
        return ComposeResponse {
            success: false,
            message: format!(
                "Invalid format '{}': must be one of nomx, json, text",
                req.format
            ),
            output_path: req.output_path,
            nomx: None,
        };
    }

    if req.source.contains("define") && req.source.contains("that") {
        ComposeResponse {
            success: true,
            message: "Compiled successfully".to_string(),
            output_path: req.output_path,
            nomx: Some(req.source),
        }
    } else {
        ComposeResponse {
            success: false,
            message: "Parse error: source must use define-that syntax".to_string(),
            output_path: req.output_path,
            nomx: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{compose_logic, ComposeRequest, ComposeResponse};

    fn req(source: &str, format: &str, output_path: Option<&str>) -> ComposeRequest {
        ComposeRequest {
            source: source.to_string(),
            format: format.to_string(),
            output_path: output_path.map(|s| s.to_string()),
        }
    }

    #[test]
    fn compose_valid_nomx() {
        let resp = compose_logic(req("define greet that says hello", "nomx", None));
        assert!(resp.success);
        assert_eq!(resp.message, "Compiled successfully");
        assert!(resp.nomx.is_some());
    }

    #[test]
    fn compose_invalid_format() {
        let resp = compose_logic(req("define greet that says hello", "xml", None));
        assert!(!resp.success);
        assert!(resp.message.contains("Invalid format"));
        assert!(resp.message.contains("xml"));
    }

    #[test]
    fn compose_empty_source() {
        let resp = compose_logic(req("", "nomx", None));
        assert!(!resp.success);
        assert!(resp.message.contains("must not be empty"));
    }

    #[test]
    fn compose_non_nomx_source() {
        let resp = compose_logic(req("fn greet() { println!(\"hello\"); }", "nomx", None));
        assert!(!resp.success);
        assert!(resp.message.contains("define-that syntax"));
        assert!(resp.nomx.is_none());
    }

    #[test]
    fn compose_response_success_fields() {
        let resp = compose_logic(req("define x that returns 1", "json", None));
        assert!(resp.success);
        assert_eq!(resp.nomx.unwrap(), "define x that returns 1");
        assert!(resp.output_path.is_none());
    }

    #[test]
    fn compose_response_error_fields() {
        let resp = compose_logic(req("just some text", "text", None));
        assert!(!resp.success);
        assert!(resp.nomx.is_none());
        assert_eq!(resp.output_path, None);
    }

    #[test]
    fn compose_with_output_path() {
        let resp = compose_logic(req(
            "define save that writes to file",
            "nomx",
            Some("/tmp/out.nomx"),
        ));
        assert!(resp.success);
        assert_eq!(resp.output_path.as_deref(), Some("/tmp/out.nomx"));
    }

    #[test]
    fn compose_request_serde() {
        let original = ComposeRequest {
            source: "define greet that says hello".to_string(),
            format: "nomx".to_string(),
            output_path: Some("/tmp/test.nomx".to_string()),
        };
        let json = serde_json::to_string(&original).unwrap();
        let roundtripped: ComposeRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtripped.source, original.source);
        assert_eq!(roundtripped.format, original.format);
        assert_eq!(roundtripped.output_path, original.output_path);

        // Also verify ComposeResponse round-trips
        let resp = ComposeResponse {
            success: true,
            message: "ok".to_string(),
            output_path: None,
            nomx: Some("define x that y".to_string()),
        };
        let resp_json = serde_json::to_string(&resp).unwrap();
        let resp2: ComposeResponse = serde_json::from_str(&resp_json).unwrap();
        assert_eq!(resp2.success, resp.success);
        assert_eq!(resp2.message, resp.message);
        assert_eq!(resp2.nomx, resp.nomx);
    }
}
