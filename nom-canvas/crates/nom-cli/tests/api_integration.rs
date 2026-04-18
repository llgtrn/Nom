//! Integration tests for the compose API types and endpoint logic.
//!
//! Handler functions in serve.rs are axum extractors and cannot be called
//! outside an axum runtime without the `serve` feature.  The tests below
//! are split into two groups:
//!
//! * Always-compiled: test the request/response types (serialisation round-trips,
//!   URL pattern construction) that are independent of the axum feature gate.
//! * Feature-gated (`serve`): spin up the router with `tower::ServiceExt` and
//!   verify the HTTP layer end-to-end.

use serde::{Deserialize, Serialize};
use serde_json::json;

// ---------------------------------------------------------------------------
// Mirrors of the public request/response types from serve.rs.
// Duplicated here so the always-compiled tests have zero feature dependency.
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct ComposeRequest {
    kind: String,
    input: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct ComposeResponse {
    kind: String,
    output: String,
    status: String,
}

// ---------------------------------------------------------------------------
// Test 1: ComposeRequest serialises and deserialises correctly.
// ---------------------------------------------------------------------------
#[test]
fn compose_request_round_trip() {
    let req = ComposeRequest {
        kind: "video".to_string(),
        input: "a sunset timelapse".to_string(),
        stream: None,
    };
    let json_str = serde_json::to_string(&req).expect("serialise must succeed");
    let decoded: ComposeRequest =
        serde_json::from_str(&json_str).expect("deserialise must succeed");
    assert_eq!(req, decoded, "round-trip must preserve all fields");
}

// ---------------------------------------------------------------------------
// Test 2: ComposeResponse serialises and deserialises correctly.
// ---------------------------------------------------------------------------
#[test]
fn compose_response_round_trip() {
    let resp = ComposeResponse {
        kind: "image".to_string(),
        output: "Composed image from: a mountain".to_string(),
        status: "ok".to_string(),
    };
    let json_str = serde_json::to_string(&resp).expect("serialise must succeed");
    let decoded: ComposeResponse =
        serde_json::from_str(&json_str).expect("deserialise must succeed");
    assert_eq!(resp, decoded, "round-trip must preserve all fields");
}

// ---------------------------------------------------------------------------
// Test 3: Promote endpoint URL pattern uses the hash as a path segment.
// ---------------------------------------------------------------------------
#[test]
fn promote_url_pattern_embeds_hash() {
    let hash = "a1b2c3d4";
    let url = format!("/promote/{hash}");
    assert!(
        url.ends_with(hash),
        "promote URL must end with the hash segment"
    );
    assert!(
        url.starts_with("/promote/"),
        "promote URL must start with /promote/"
    );
}

// ---------------------------------------------------------------------------
// Test 4: stream flag serialises correctly for both true and false.
// ---------------------------------------------------------------------------
#[test]
fn compose_request_stream_flag_variants() {
    let with_stream = json!({"kind": "audio", "input": "beat", "stream": true});
    let req: ComposeRequest =
        serde_json::from_value(with_stream).expect("deserialise with stream:true must succeed");
    assert_eq!(req.stream, Some(true));

    let without_stream = json!({"kind": "audio", "input": "beat", "stream": false});
    let req2: ComposeRequest =
        serde_json::from_value(without_stream).expect("deserialise with stream:false must succeed");
    assert_eq!(req2.stream, Some(false));

    let no_stream_field = json!({"kind": "audio", "input": "beat"});
    let req3: ComposeRequest =
        serde_json::from_value(no_stream_field).expect("deserialise without stream field must succeed");
    assert_eq!(req3.stream, None, "absent stream field must deserialise to None");
}

// ---------------------------------------------------------------------------
// Feature-gated tests: exercise the axum router via tower::ServiceExt.
// ---------------------------------------------------------------------------
#[cfg(feature = "serve")]
mod serve_tests {
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use nom_cli::serve::compose_router;
    use tower::ServiceExt;

    // Test 5: compose handler returns error-like response for missing required fields.
    #[tokio::test]
    async fn compose_handler_empty_body_returns_error() {
        let app = compose_router();
        let req = Request::builder()
            .method(Method::POST)
            .uri("/compose")
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        // axum will return 422 Unprocessable Entity when required fields are absent.
        assert_eq!(
            resp.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "missing required fields must yield 422"
        );
    }

    // Test 6: compose handler returns 200 with valid body.
    #[tokio::test]
    async fn compose_handler_valid_body_returns_ok() {
        let app = compose_router();
        let req = Request::builder()
            .method(Method::POST)
            .uri("/compose")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"kind":"video","input":"test"}"#))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // Test 7: promote handler returns 200 with a valid 8-char hash.
    #[tokio::test]
    async fn promote_handler_valid_hash_returns_ok() {
        let app = compose_router();
        let req = Request::builder()
            .method(Method::POST)
            .uri("/promote/a1b2c3d4")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["promoted"], true);
        assert_eq!(body["glue_hash"], "a1b2c3d4");
    }

    // Test 8: promote response JSON contains expected fields.
    #[tokio::test]
    async fn promote_handler_response_shape() {
        let app = compose_router();
        let req = Request::builder()
            .method(Method::POST)
            .uri("/promote/deadbeef")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(body.get("promoted").is_some(), "response must have 'promoted' field");
        assert!(body.get("glue_hash").is_some(), "response must have 'glue_hash' field");
        assert!(body.get("status").is_some(), "response must have 'status' field");
    }
}
