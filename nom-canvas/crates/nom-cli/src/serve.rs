use axum::extract::Path;
use axum::{extract::Json, response::Json as RespJson, routing::post, Router};
use serde::{Deserialize, Serialize};

use crate::compose::{compose_logic, ComposeRequest, ComposeResponse};

// ---------------------------------------------------------------------------
// POST /compose — new endpoint with source/format/output_path contract
// ---------------------------------------------------------------------------

/// Axum handler that delegates to the pure `compose_logic` function.
pub async fn compose_handler(Json(body): Json<ComposeRequest>) -> RespJson<ComposeResponse> {
    RespJson(compose_logic(body))
}

/// Returns a Router with the `POST /compose` route registered.
pub fn build_router() -> Router {
    Router::new().route("/compose", post(compose_handler))
}

// ---------------------------------------------------------------------------
// Legacy router (kind/input/stream contract) — kept for backwards compat
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct LegacyComposeRequest {
    pub kind: String,
    pub input: String,
    pub stream: Option<bool>,
}

#[derive(Serialize)]
pub struct LegacyComposeResponse {
    pub kind: String,
    pub output: String,
    pub status: String,
}

async fn handle_compose(
    Json(req): Json<LegacyComposeRequest>,
) -> RespJson<LegacyComposeResponse> {
    let output = format!("Composed {} from: {}", req.kind, req.input);
    RespJson(LegacyComposeResponse {
        kind: req.kind,
        output,
        status: "ok".to_string(),
    })
}

async fn handle_promote(Path(glue_hash): Path<String>) -> impl axum::response::IntoResponse {
    // In production: calls DictWriter::insert_partial_entry() with the cached glue
    // Stub: returns 200 OK with hash confirmation
    axum::Json(serde_json::json!({
        "promoted": true,
        "glue_hash": glue_hash,
        "status": "partial"
    }))
}

pub fn compose_router() -> Router {
    Router::new()
        .route("/compose", post(handle_compose))
        .route("/promote/:glue_hash", post(handle_promote))
}

pub async fn serve(addr: &str) {
    let app = compose_router();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("Nom compose server listening on {}", addr);
    axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::compose_router;
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_post_compose_returns_ok() {
        let app = compose_router();
        let req = Request::builder()
            .method(Method::POST)
            .uri("/compose")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"kind":"video","input":"a sunset timelapse"}"#,
            ))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_promote_route_returns_hash() {
        let app = compose_router();
        let req = Request::builder()
            .method(Method::POST)
            .uri("/promote/abc123def456")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["promoted"], true);
        assert_eq!(json["glue_hash"], "abc123def456");
        assert_eq!(json["status"], "partial");
    }

    #[tokio::test]
    async fn test_post_compose_returns_kind_in_response() {
        let app = compose_router();
        let req = Request::builder()
            .method(Method::POST)
            .uri("/compose")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"kind":"image","input":"a mountain"}"#))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["kind"], "image");
    }
}
