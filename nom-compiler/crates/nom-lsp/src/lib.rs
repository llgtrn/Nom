//! M16 LSP scaffold. Week-1 slice per doc 10 §D.
//!
//! Mines Zed's 3-primitive architecture — transport (lsp-server),
//! language plug-in contract (LspAdapter analog lives in a later slice),
//! workspace orchestrator (stub until dict wiring lands) — but starts
//! with the minimum that proves initialize + hover round-trip end-to-end.
//!
//! Library-level entry points (`serve_on_pipes`, `handle_request`) are
//! separately testable; the binary is a thin wrapper added when the
//! `nom lsp serve` CLI subcommand lands.
//!
//! Slice-6a added the `agent` module — pure functions that drive the
//! agentic ReAct loop and format transcripts as LSP markdown. These
//! power the "why-this-Nom?" editor drill-through without yet being
//! wired into a request handler (that's slice-6b).

pub mod agent;

use lsp_server::{Connection, ExtractError, IoThreads, Message, Request, RequestId, Response};
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionOptions, CompletionResponse,
    HoverContents, HoverProviderCapability, InitializeParams, MarkupContent, MarkupKind,
    ServerCapabilities,
    request::{Completion, HoverRequest, Request as LspRequest},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Version advertised in `initialize` server_info, so clients can see
/// which Nom compiler shipped the LSP.
pub const SERVER_NAME: &str = "nom-lsp";
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Error)]
pub enum LspError {
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
}

/// Server capabilities for the current slice — hover + keyword completion.
/// Later slices flip on definition_provider / semantic_tokens_provider as
/// the corresponding handlers land.
pub fn server_capabilities() -> ServerCapabilities {
    ServerCapabilities {
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        completion_provider: Some(CompletionOptions {
            resolve_provider: Some(false),
            trigger_characters: None,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Start stdio transport + main loop. Called by the binary and by
/// integration tests (via `lsp_server::Connection::memory()` pair).
///
/// Not covered by unit tests — the pipe lifecycle is exercised through
/// integration tests below that construct their own in-memory connection.
pub fn serve_on_stdio() -> Result<(), LspError> {
    let (connection, io_threads) = Connection::stdio();
    run_server(connection, io_threads)
}

fn run_server(connection: Connection, io_threads: IoThreads) -> Result<(), LspError> {
    let server_caps = serde_json::to_value(server_capabilities())?;
    let (_id, _params): (_, InitializeParams) = {
        let init_params = connection
            .initialize(server_caps)
            .map_err(|e| LspError::Protocol(format!("initialize failed: {e}")))?;
        let init_params: InitializeParams = serde_json::from_value(init_params)?;
        ((), init_params)
    };

    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection
                    .handle_shutdown(&req)
                    .map_err(|e| LspError::Protocol(format!("shutdown failed: {e}")))?
                {
                    break;
                }
                let response = dispatch_request(req);
                connection
                    .sender
                    .send(Message::Response(response))
                    .map_err(|e| LspError::Protocol(format!("send failed: {e}")))?;
            }
            Message::Response(_) | Message::Notification(_) => {
                // Week-1: ignore. Later slices route notifications to dirty-buffer
                // trackers and server-to-client responses to cancellation bookkeeping.
            }
        }
    }

    io_threads
        .join()
        .map_err(|e| LspError::Protocol(format!("io_threads join failed: {e}")))?;
    Ok(())
}

/// Route an incoming request to its handler. Pure function of
/// (RequestId, method, params) — fully unit-testable.
///
/// This is the place that grows as new LSP methods come online:
/// `textDocument/definition`, `textDocument/completion`,
/// `textDocument/semanticTokens/full`, etc. Mirrors Zed's
/// `on_request` closure registry pattern but statically dispatched.
pub fn dispatch_request(req: Request) -> Response {
    let id = req.id.clone();
    match req.method.as_str() {
        HoverRequest::METHOD => match cast::<HoverRequest>(req) {
            Ok((id, params)) => handle_hover(id, params),
            Err(err) => method_mismatch(id, err),
        },
        Completion::METHOD => match cast::<Completion>(req) {
            Ok((id, params)) => handle_completion(id, params),
            Err(err) => method_mismatch(id, err),
        },
        _ => Response {
            id,
            result: None,
            error: Some(lsp_server::ResponseError {
                code: lsp_server::ErrorCode::MethodNotFound as i32,
                message: "method not implemented in nom-lsp week-1 slice".into(),
                data: None,
            }),
        },
    }
}

fn cast<R>(req: Request) -> Result<(RequestId, R::Params), ExtractError<Request>>
where
    R: LspRequest,
    R::Params: serde::de::DeserializeOwned,
{
    req.extract(R::METHOD)
}

fn method_mismatch(id: RequestId, err: ExtractError<Request>) -> Response {
    Response {
        id,
        result: None,
        error: Some(lsp_server::ResponseError {
            code: lsp_server::ErrorCode::InvalidParams as i32,
            message: format!("extract failed: {err:?}"),
            data: None,
        }),
    }
}

/// Week-1 completion: return canonical `.nomx v2` keyword completions.
/// Later slice will scope these by context (e.g. only inside a `define`
/// body) and add dict-backed symbol completions.
///
/// The keyword set mirrors the shipped subset from doc 06 §1-§4
/// (declaration + control + contract + linkers).
fn handle_completion(
    id: RequestId,
    _params: lsp_types::CompletionParams,
) -> Response {
    const KEYWORDS: &[(&str, &str)] = &[
        ("define", "declare a function: `define X that takes Y and returns Z:`"),
        ("to", "one-liner: `to greet someone, respond with \"hi, \" + name.`"),
        ("record", "declare a record: `record Point holds x is a number, y is a number.`"),
        ("choice", "declare a choice: `choice Color is one of: red, green, blue.`"),
        ("when", "conditional branch: `when <cond>, <then>. otherwise, <else>.`"),
        ("unless", "negated conditional: `unless <cond>, <then>.`"),
        ("for", "for-each loop: `for each x in xs, <body>.`"),
        ("while", "while loop: `while <cond>, <body>.`"),
        ("require", "precondition contract: `require <predicate>.`"),
        ("ensure", "postcondition contract: `ensure <predicate>.`"),
        ("throughout", "invariant contract: `throughout <predicate>.`"),
    ];
    let items: Vec<CompletionItem> = KEYWORDS
        .iter()
        .map(|(label, detail)| CompletionItem {
            label: (*label).to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some((*detail).to_string()),
            ..Default::default()
        })
        .collect();
    let response = CompletionResponse::Array(items);
    Response {
        id,
        result: Some(serde_json::to_value(response).expect("completion serializes")),
        error: None,
    }
}

/// Week-1 hover: constant "nom-lsp alive" marker. Later slice replaces
/// this with a dict lookup (resolve the symbol under the cursor through
/// nom-resolver + surface glass-box report JSON from cmd_build_report).
fn handle_hover(id: RequestId, _params: lsp_types::HoverParams) -> Response {
    let hover = lsp_types::Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!("**{SERVER_NAME}** v{SERVER_VERSION} — hover stub alive"),
        }),
        range: None,
    };
    Response {
        id,
        result: Some(serde_json::to_value(hover).expect("hover serializes")),
        error: None,
    }
}

/// Marker record that integration tests use to verify the hover payload
/// shape without depending on lsp-types internal JSON layout.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HoverProbe {
    pub contains: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_capabilities_exposes_hover_and_completion() {
        let caps = server_capabilities();
        assert!(matches!(
            caps.hover_provider,
            Some(HoverProviderCapability::Simple(true))
        ));
        assert!(caps.completion_provider.is_some(), "completion_provider must be on");
        assert!(caps.definition_provider.is_none());
    }

    #[test]
    fn dispatch_completion_returns_keyword_items() {
        let req = Request::new(
            RequestId::from(3),
            Completion::METHOD.to_string(),
            lsp_types::CompletionParams {
                text_document_position: lsp_types::TextDocumentPositionParams {
                    text_document: lsp_types::TextDocumentIdentifier {
                        uri: lsp_types::Url::parse("file:///tmp/fake.nomx").unwrap(),
                    },
                    position: lsp_types::Position { line: 0, character: 0 },
                },
                work_done_progress_params: Default::default(),
                partial_result_params: Default::default(),
                context: None,
            },
        );
        let resp = dispatch_request(req);
        assert!(resp.error.is_none());
        let items: CompletionResponse =
            serde_json::from_value(resp.result.expect("completion result set")).unwrap();
        let labels: Vec<String> = match items {
            CompletionResponse::Array(a) => a.into_iter().map(|i| i.label).collect(),
            CompletionResponse::List(l) => l.items.into_iter().map(|i| i.label).collect(),
        };
        for keyword in ["define", "record", "when", "require", "ensure"] {
            assert!(
                labels.iter().any(|l| l == keyword),
                "completion response missing '{keyword}' (labels: {labels:?})"
            );
        }
    }

    #[test]
    fn dispatch_hover_returns_markdown_with_server_name() {
        let req = Request::new(
            RequestId::from(1),
            HoverRequest::METHOD.to_string(),
            lsp_types::HoverParams {
                text_document_position_params: lsp_types::TextDocumentPositionParams {
                    text_document: lsp_types::TextDocumentIdentifier {
                        uri: lsp_types::Url::parse("file:///tmp/fake.nom").unwrap(),
                    },
                    position: lsp_types::Position { line: 0, character: 0 },
                },
                work_done_progress_params: Default::default(),
            },
        );
        let resp = dispatch_request(req);
        assert!(resp.error.is_none());
        let hover: lsp_types::Hover =
            serde_json::from_value(resp.result.expect("hover result set")).unwrap();
        let body = match hover.contents {
            HoverContents::Markup(m) => m.value,
            _ => panic!("expected markup hover"),
        };
        assert!(body.contains(SERVER_NAME), "hover body must name server: {body}");
    }

    #[test]
    fn dispatch_unknown_method_returns_method_not_found() {
        let req = Request::new(
            RequestId::from(7),
            "textDocument/foo".into(),
            serde_json::json!({}),
        );
        let resp = dispatch_request(req);
        let err = resp.error.expect("unknown method must error");
        assert_eq!(err.code, lsp_server::ErrorCode::MethodNotFound as i32);
    }

    #[test]
    fn server_name_and_version_are_nonempty() {
        assert!(!SERVER_NAME.is_empty());
        assert!(!SERVER_VERSION.is_empty());
    }
}
