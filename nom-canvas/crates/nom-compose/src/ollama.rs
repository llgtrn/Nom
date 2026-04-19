//! Ollama HTTP client wrapper.
//!
//! Talks to an external Ollama binary via its REST API.
//! Default host: `http://127.0.0.1:11434` (reads `OLLAMA_HOST` or `NOM_OLLAMA_HOST`).

use serde::{Deserialize, Serialize};
use std::env;

/// Error type for Ollama client operations.
#[derive(Debug)]
pub enum OllamaError {
    /// HTTP transport or status error.
    Http(reqwest::Error),
    /// JSON serialization / deserialization error.
    Json(serde_json::Error),
    /// IO error (e.g. while reading a streaming response).
    Io(std::io::Error),
    /// Explicit API error response.
    Api { status: u16, message: String },
}

impl std::fmt::Display for OllamaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OllamaError::Http(e) => write!(f, "HTTP error: {e}"),
            OllamaError::Json(e) => write!(f, "JSON error: {e}"),
            OllamaError::Io(e) => write!(f, "IO error: {e}"),
            OllamaError::Api { status, message } => {
                write!(f, "API error {status}: {message}")
            }
        }
    }
}

impl std::error::Error for OllamaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            OllamaError::Http(e) => Some(e),
            OllamaError::Json(e) => Some(e),
            OllamaError::Io(e) => Some(e),
            OllamaError::Api { .. } => None,
        }
    }
}

impl From<reqwest::Error> for OllamaError {
    fn from(e: reqwest::Error) -> Self {
        OllamaError::Http(e)
    }
}

impl From<serde_json::Error> for OllamaError {
    fn from(e: serde_json::Error) -> Self {
        OllamaError::Json(e)
    }
}

impl From<std::io::Error> for OllamaError {
    fn from(e: std::io::Error) -> Self {
        OllamaError::Io(e)
    }
}

/// A chat message for the `/api/chat` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OllamaMessage {
    /// Role of the message sender, e.g. `"user"`, `"assistant"`, `"system"`.
    pub role: String,
    /// Message content.
    pub content: String,
}

impl OllamaMessage {
    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
        }
    }

    /// Create an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: content.into(),
        }
    }

    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".into(),
            content: content.into(),
        }
    }
}

/// Request body for `/api/generate`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateRequest {
    /// Model name, e.g. `"llama3"`.
    pub model: String,
    /// Prompt text.
    pub prompt: String,
    /// Whether to stream the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// Optional model parameters (temperature, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<serde_json::Value>,
}

/// Response body for `/api/generate`.
#[derive(Debug, Clone, Deserialize)]
pub struct GenerateResponse {
    /// Generated text.
    pub response: String,
    /// True when the model has finished generating.
    #[serde(default)]
    pub done: bool,
    /// Context token IDs (present on the final response when `stream: false`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Vec<u64>>,
}

/// Request body for `/api/chat`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// Model name, e.g. `"llama3"`.
    pub model: String,
    /// Conversation history.
    pub messages: Vec<OllamaMessage>,
    /// Whether to stream the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// Optional model parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<serde_json::Value>,
}

/// Response body for `/api/chat`.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ChatResponse {
    /// The message returned by the model.
    pub message: OllamaMessage,
    /// True when the model has finished generating.
    #[serde(default)]
    pub done: bool,
}

/// Request body for `/api/pull`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    /// Model name to pull.
    pub model: String,
    /// Whether to stream progress events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

/// Progress event streamed during `/api/pull`.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct PullProgress {
    /// Human-readable status, e.g. `"pulling manifest"`.
    pub status: String,
    /// Bytes completed so far.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed: Option<u64>,
    /// Total bytes expected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
}

/// HTTP client for an external Ollama server.
#[derive(Debug, Clone)]
pub struct OllamaClient {
    base_url: String,
    client: reqwest::blocking::Client,
}

impl OllamaClient {
    /// Create a new client targeting the given base URL.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: reqwest::blocking::Client::new(),
        }
    }

    /// Create a client from environment (`OLLAMA_HOST` or `NOM_OLLAMA_HOST`),
    /// defaulting to `http://127.0.0.1:11434`.
    pub fn from_env() -> Self {
        let host = env::var("NOM_OLLAMA_HOST")
            .or_else(|_| env::var("OLLAMA_HOST"))
            .unwrap_or_else(|_| "http://127.0.0.1:11434".into());
        Self::new(host)
    }

    /// POST /api/generate
    ///
    /// Sends a prompt to the model and returns the generated text.
    pub fn generate(&self, model: &str, prompt: &str) -> Result<GenerateResponse, OllamaError> {
        let req = GenerateRequest {
            model: model.into(),
            prompt: prompt.into(),
            stream: Some(false),
            options: None,
        };
        let resp = self
            .client
            .post(format!("{}/api/generate", self.base_url))
            .json(&req)
            .send()?;
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().unwrap_or_default();
            return Err(OllamaError::Api { status, message: text });
        }
        Ok(resp.json()?)
    }

    /// POST /api/chat
    ///
    /// Sends a conversation to the model and returns the assistant's reply.
    pub fn chat(
        &self,
        model: &str,
        messages: Vec<OllamaMessage>,
    ) -> Result<ChatResponse, OllamaError> {
        let req = ChatRequest {
            model: model.into(),
            messages,
            stream: Some(false),
            options: None,
        };
        let resp = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&req)
            .send()?;
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().unwrap_or_default();
            return Err(OllamaError::Api { status, message: text });
        }
        Ok(resp.json()?)
    }

    /// POST /api/pull (streaming NDJSON).
    ///
    /// Returns an iterator over each `PullProgress` line.
    pub fn pull(
        &self,
        model: &str,
    ) -> Result<impl Iterator<Item = Result<PullProgress, OllamaError>>, OllamaError> {
        let req = PullRequest {
            model: model.into(),
            stream: Some(true),
        };
        let resp = self
            .client
            .post(format!("{}/api/pull", self.base_url))
            .json(&req)
            .send()?;
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().unwrap_or_default();
            return Err(OllamaError::Api { status, message: text });
        }
        let reader = std::io::BufReader::new(resp);
        Ok(std::io::BufRead::lines(reader).filter_map(|line| {
            match line {
                Ok(line) if line.trim().is_empty() => None,
                Ok(line) => match serde_json::from_str::<PullProgress>(&line) {
                    Ok(progress) => Some(Ok(progress)),
                    Err(e) => Some(Err(OllamaError::Json(e))),
                },
                Err(e) => Some(Err(OllamaError::Io(e))),
            }
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_from_env_uses_default() {
        let client = OllamaClient::from_env();
        assert_eq!(client.base_url, "http://127.0.0.1:11434");
    }

    #[test]
    fn client_new_sets_base_url() {
        let client = OllamaClient::new("http://example.com:8080");
        assert_eq!(client.base_url, "http://example.com:8080");
    }

    #[test]
    fn message_helpers_build_correct_roles() {
        let u = OllamaMessage::user("hi");
        let a = OllamaMessage::assistant("hello");
        let s = OllamaMessage::system("be nice");
        assert_eq!(u.role, "user");
        assert_eq!(a.role, "assistant");
        assert_eq!(s.role, "system");
    }

    #[test]
    fn generate_request_serializes() {
        let req = GenerateRequest {
            model: "llama3".into(),
            prompt: "hello".into(),
            stream: Some(false),
            options: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("llama3"));
        assert!(json.contains("hello"));
        assert!(json.contains("false"));
    }

    #[test]
    fn pull_progress_deserializes() {
        let json = r#"{"status":"pulling manifest","completed":100,"total":200}"#;
        let progress: PullProgress = serde_json::from_str(json).unwrap();
        assert_eq!(progress.status, "pulling manifest");
        assert_eq!(progress.completed, Some(100));
        assert_eq!(progress.total, Some(200));
    }

    #[test]
    fn chat_response_deserializes() {
        let json = r#"{"message":{"role":"assistant","content":"ok"},"done":true}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.message.role, "assistant");
        assert_eq!(resp.message.content, "ok");
        assert!(resp.done);
    }

    #[test]
    fn generate_response_deserializes() {
        let json = r#"{"response":"hi there","done":true,"context":[1,2,3]}"#;
        let resp: GenerateResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.response, "hi there");
        assert!(resp.done);
        assert_eq!(resp.context, Some(vec![1, 2, 3]));
    }

    #[test]
    fn ollama_error_display_formats() {
        let e = OllamaError::Api {
            status: 500,
            message: "boom".into(),
        };
        assert_eq!(format!("{e}"), "API error 500: boom");
    }
}
