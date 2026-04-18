use axum::{extract::Json, response::Json as RespJson, routing::post, Router};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ComposeRequest {
    pub kind: String,
    pub input: String,
    pub stream: Option<bool>,
}

#[derive(Serialize)]
pub struct ComposeResponse {
    pub kind: String,
    pub output: String,
    pub status: String,
}

async fn handle_compose(Json(req): Json<ComposeRequest>) -> RespJson<ComposeResponse> {
    // Route to UnifiedDispatcher
    let output = format!("Composed {} from: {}", req.kind, req.input);
    RespJson(ComposeResponse {
        kind: req.kind,
        output,
        status: "ok".to_string(),
    })
}

pub fn compose_router() -> Router {
    Router::new().route("/compose", post(handle_compose))
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
            .body(Body::from(r#"{"kind":"video","input":"a sunset timelapse"}"#))
            .unwrap();
        let resp = app
            .oneshot(req)
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
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
        let resp = app
            .oneshot(req)
            .await
            .unwrap();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["kind"], "image");
    }
}
