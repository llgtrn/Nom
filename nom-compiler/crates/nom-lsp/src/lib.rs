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
    ExecuteCommandOptions, ExecuteCommandParams, HoverContents, HoverProviderCapability,
    InitializeParams, MarkupContent, MarkupKind, ServerCapabilities,
    request::{Completion, ExecuteCommand, HoverRequest, Request as LspRequest},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Version advertised in `initialize` server_info, so clients can see
/// which Nom compiler shipped the LSP.
pub const SERVER_NAME: &str = "nom-lsp";
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// LSP command id for the "why this Nom?" drill-through. Clients invoke
/// it with `workspace/executeCommand { command: "nom.whyThisNom",
/// arguments: [prose: string, dict_path: string] }`; server returns
/// the `MarkupContent` from `agent::render_agent_transcript`.
pub const CMD_WHY_THIS_NOM: &str = "nom.whyThisNom";

/// `workspace/executeCommand` token for the pattern-search command.
/// Editor clients send `{ command: "nom.searchPatterns", arguments:
/// [prose: string, grammar_db_path: string, threshold?: number,
/// limit?: number] }`. The server returns a JSON array of objects
/// `{ score, pattern_id, intent }` ranked by Jaccard token-overlap
/// against each row's intent — the same backend the CLI's
/// `nom grammar pattern-search` and the CI uniqueness test use, so
/// editor results are byte-identical to the CLI for the same query.
pub const CMD_SEARCH_PATTERNS: &str = "nom.searchPatterns";

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
        execute_command_provider: Some(ExecuteCommandOptions {
            commands: vec![
                CMD_WHY_THIS_NOM.to_string(),
                CMD_SEARCH_PATTERNS.to_string(),
            ],
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
        ExecuteCommand::METHOD => match cast::<ExecuteCommand>(req) {
            Ok((id, params)) => handle_execute_command(id, params),
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

/// Slice-6b: `workspace/executeCommand` handler. Routes on
/// `params.command` and returns a JSON result. Currently supports only
/// `nom.whyThisNom` — extensible via additional match arms as future
/// slices add commands (e.g. `nom.toggleLocalePack`, `nom.dreamTier`).
///
/// `nom.whyThisNom` contract: arguments = [prose: string, dict_path:
/// string]. Returns the `MarkupContent` from
/// `agent::render_agent_transcript` serialized as JSON; editors render
/// it directly. Errors (missing args, malformed args, dict-open failure,
/// agent-loop error) produce LSP `InvalidParams` / `InternalError`
/// responses with structured messages so the client can surface them.
fn handle_execute_command(id: RequestId, params: ExecuteCommandParams) -> Response {
    match params.command.as_str() {
        CMD_WHY_THIS_NOM => handle_why_this_nom(id, params),
        CMD_SEARCH_PATTERNS => handle_search_patterns(id, params),
        _ => Response {
            id,
            result: None,
            error: Some(lsp_server::ResponseError {
                code: lsp_server::ErrorCode::MethodNotFound as i32,
                message: format!(
                    "nom-lsp: command {:?} not handled; supported: [{}, {}]",
                    params.command, CMD_WHY_THIS_NOM, CMD_SEARCH_PATTERNS
                ),
                data: None,
            }),
        },
    }
}

fn handle_search_patterns(id: RequestId, params: ExecuteCommandParams) -> Response {
    // Args: [prose: string, grammar_db_path: string, threshold?: number, limit?: number]
    let prose = match params.arguments.first().and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => {
            return Response {
                id,
                result: None,
                error: Some(lsp_server::ResponseError {
                    code: lsp_server::ErrorCode::InvalidParams as i32,
                    message: "nom.searchPatterns: arg[0] (prose) missing or not a string".into(),
                    data: None,
                }),
            };
        }
    };
    let db_path = match params.arguments.get(1).and_then(|v| v.as_str()) {
        Some(s) => std::path::PathBuf::from(s),
        None => {
            return Response {
                id,
                result: None,
                error: Some(lsp_server::ResponseError {
                    code: lsp_server::ErrorCode::InvalidParams as i32,
                    message: "nom.searchPatterns: arg[1] (grammar_db_path) missing or not a string"
                        .into(),
                    data: None,
                }),
            };
        }
    };
    let threshold = params
        .arguments
        .get(2)
        .and_then(|v| v.as_f64())
        .unwrap_or(0.10);
    let limit = params
        .arguments
        .get(3)
        .and_then(|v| v.as_u64())
        .unwrap_or(10) as usize;

    let conn = match nom_grammar::open_readonly(&db_path) {
        Ok(c) => c,
        Err(e) => {
            return Response {
                id,
                result: None,
                error: Some(lsp_server::ResponseError {
                    code: lsp_server::ErrorCode::InternalError as i32,
                    message: format!("nom.searchPatterns: open grammar db: {e}"),
                    data: None,
                }),
            };
        }
    };
    let hits = match nom_grammar::search_patterns(&conn, &prose, threshold, limit) {
        Ok(h) => h,
        Err(e) => {
            return Response {
                id,
                result: None,
                error: Some(lsp_server::ResponseError {
                    code: lsp_server::ErrorCode::InternalError as i32,
                    message: format!("nom.searchPatterns: search: {e}"),
                    data: None,
                }),
            };
        }
    };
    let arr: Vec<serde_json::Value> = hits
        .iter()
        .map(|m| {
            serde_json::json!({
                "score": m.score,
                "pattern_id": m.pattern_id,
                "intent": m.intent,
            })
        })
        .collect();
    Response {
        id,
        result: Some(serde_json::Value::Array(arr)),
        error: None,
    }
}

fn handle_why_this_nom(id: RequestId, params: ExecuteCommandParams) -> Response {
    // Args: [prose, dict_path] — both required strings.
    let prose = match params.arguments.first().and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => {
            return Response {
                id,
                result: None,
                error: Some(lsp_server::ResponseError {
                    code: lsp_server::ErrorCode::InvalidParams as i32,
                    message: "nom.whyThisNom: missing argument[0] = prose (string)".into(),
                    data: None,
                }),
            };
        }
    };
    let dict_path = match params.arguments.get(1).and_then(|v| v.as_str()) {
        Some(s) => std::path::PathBuf::from(s),
        None => {
            return Response {
                id,
                result: None,
                error: Some(lsp_server::ResponseError {
                    code: lsp_server::ErrorCode::InvalidParams as i32,
                    message: "nom.whyThisNom: missing argument[1] = dict_path (string)".into(),
                    data: None,
                }),
            };
        }
    };
    let budget = nom_intent::react::ReActBudget::default();
    match agent::render_agent_transcript(&prose, &dict_path, &budget) {
        Ok(markup) => Response {
            id,
            result: Some(serde_json::to_value(markup).expect("markup serializes")),
            error: None,
        },
        Err(e) => Response {
            id,
            result: None,
            error: Some(lsp_server::ResponseError {
                code: lsp_server::ErrorCode::InternalError as i32,
                message: format!("nom.whyThisNom: {e}"),
                data: None,
            }),
        },
    }
}

/// Week-1 completion: return canonical `.nomx v2` keyword completions.
/// Later slice will scope these by context (e.g. only inside a `define`
/// body) and add dict-backed symbol completions.
///
/// The keyword set mirrors the shipped subset from doc 06 §1-§4
/// (declaration + control + contract + linkers).
/// Environment variable consulted by [`handle_completion`] to find the
/// grammar.sqlite path. When set + readable, every `patterns.pattern_id`
/// is appended to the completion item list as `CompletionItemKind::SNIPPET`
/// so editor-side filtering narrows the catalog by typed prefix. Absent
/// or unreadable → keyword-only (slice-1 behaviour).
pub const ENV_GRAMMAR_DB: &str = "NOM_GRAMMAR_DB";

fn handle_completion(id: RequestId, _params: lsp_types::CompletionParams) -> Response {
    const KEYWORDS: &[(&str, &str)] = &[
        (
            "define",
            "declare a function: `define X that takes Y and returns Z:`",
        ),
        (
            "to",
            "one-liner: `to greet someone, respond with \"hi, \" + name.`",
        ),
        (
            "record",
            "declare a record: `record Point holds x is a number, y is a number.`",
        ),
        (
            "choice",
            "declare a choice: `choice Color is one of: red, green, blue.`",
        ),
        (
            "when",
            "conditional branch: `when <cond>, <then>. otherwise, <else>.`",
        ),
        ("unless", "negated conditional: `unless <cond>, <then>.`"),
        ("for", "for-each loop: `for each x in xs, <body>.`"),
        ("while", "while loop: `while <cond>, <body>.`"),
        ("require", "precondition contract: `require <predicate>.`"),
        ("ensure", "postcondition contract: `ensure <predicate>.`"),
        (
            "throughout",
            "invariant contract: `throughout <predicate>.`",
        ),
    ];
    let mut items: Vec<CompletionItem> = KEYWORDS
        .iter()
        .map(|(label, detail)| CompletionItem {
            label: (*label).to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some((*detail).to_string()),
            ..Default::default()
        })
        .collect();

    // Pattern-catalog completion (slice-7a). When NOM_GRAMMAR_DB names a
    // readable grammar.sqlite, every pattern_id surfaces as a snippet
    // completion with its intent prose as the detail. Editors filter by
    // typed prefix so the user sees only patterns matching what they
    // type. Absent env var or unreadable DB → keyword-only fallback.
    if let Ok(db_path) = std::env::var(ENV_GRAMMAR_DB) {
        if let Ok(conn) = nom_grammar::open_readonly(&db_path) {
            if let Ok(rows) = nom_grammar::list_pattern_intents(&conn) {
                for (pid, intent) in rows {
                    items.push(CompletionItem {
                        label: pid.clone(),
                        kind: Some(CompletionItemKind::SNIPPET),
                        detail: Some(intent),
                        insert_text: Some(pid),
                        ..Default::default()
                    });
                }
            }
        }
    }

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
    fn server_capabilities_exposes_hover_completion_and_execute_command() {
        let caps = server_capabilities();
        assert!(matches!(
            caps.hover_provider,
            Some(HoverProviderCapability::Simple(true))
        ));
        assert!(
            caps.completion_provider.is_some(),
            "completion_provider must be on"
        );
        let ec = caps
            .execute_command_provider
            .expect("execute_command_provider must be on");
        assert!(
            ec.commands.iter().any(|c| c == CMD_WHY_THIS_NOM),
            "nom.whyThisNom must be advertised; got {:?}",
            ec.commands
        );
        assert!(
            ec.commands.iter().any(|c| c == CMD_SEARCH_PATTERNS),
            "nom.searchPatterns must be advertised; got {:?}",
            ec.commands
        );
        assert!(caps.definition_provider.is_none());
    }

    #[test]
    fn search_patterns_dispatch_returns_top_matches() {
        // Seed an in-memory grammar.sqlite via the canonical helpers
        // and dispatch a workspace/executeCommand for the new
        // nom.searchPatterns command. Verify the response is a JSON
        // array whose top hit matches the seeded pattern.
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("g.sqlite");
        let conn = nom_grammar::init_at(&db).unwrap();
        conn.execute(
            "INSERT INTO patterns \
             (pattern_id, intent, nom_kinds, nom_clauses, typed_slot_refs, \
              example_shape, hazards, favors, source_doc_refs) \
             VALUES \
             ('alpha-cache', 'cache pure function results keyed on input', \
              '[]', '[]', '[]', '', '[]', '[]', '[]'), \
             ('beta-render', 'render typeset glyphs along baseline', \
              '[]', '[]', '[]', '', '[]', '[]', '[]')",
            [],
        )
        .unwrap();
        drop(conn);

        let req = Request::new(
            RequestId::from(7),
            ExecuteCommand::METHOD.to_string(),
            ExecuteCommandParams {
                command: CMD_SEARCH_PATTERNS.into(),
                arguments: vec![
                    serde_json::Value::String("cache pure function results".into()),
                    serde_json::Value::String(db.to_string_lossy().into_owned()),
                ],
                work_done_progress_params: Default::default(),
            },
        );
        let resp = dispatch_request(req);
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let arr = resp.result.expect("search-patterns result set");
        let arr = arr.as_array().expect("array");
        assert!(!arr.is_empty(), "expected at least one match");
        let top_id = arr[0].get("pattern_id").and_then(|v| v.as_str()).unwrap();
        assert_eq!(top_id, "alpha-cache");
    }

    #[test]
    fn search_patterns_dispatch_rejects_missing_args() {
        let req = Request::new(
            RequestId::from(8),
            ExecuteCommand::METHOD.to_string(),
            ExecuteCommandParams {
                command: CMD_SEARCH_PATTERNS.into(),
                arguments: vec![], // no args at all
                work_done_progress_params: Default::default(),
            },
        );
        let resp = dispatch_request(req);
        assert!(resp.result.is_none());
        let err = resp.error.expect("error must be set");
        assert_eq!(err.code, lsp_server::ErrorCode::InvalidParams as i32);
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
                    position: lsp_types::Position {
                        line: 0,
                        character: 0,
                    },
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
    fn completion_appends_pattern_ids_when_grammar_db_env_set() {
        // Slice-7a: when NOM_GRAMMAR_DB names a readable grammar.sqlite,
        // every patterns.pattern_id surfaces as a snippet completion
        // alongside the fixed keyword list.
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("g.sqlite");
        let conn = nom_grammar::init_at(&db).unwrap();
        conn.execute(
            "INSERT INTO patterns \
             (pattern_id, intent, nom_kinds, nom_clauses, typed_slot_refs, \
              example_shape, hazards, favors, source_doc_refs) \
             VALUES \
             ('alpha-cache-pattern', 'cache pure function results', \
              '[]', '[]', '[]', '', '[]', '[]', '[]'), \
             ('beta-render-pattern', 'render typeset glyphs along baseline', \
              '[]', '[]', '[]', '', '[]', '[]', '[]')",
            [],
        )
        .unwrap();
        drop(conn);

        // SAFETY: env mutation in tests races with parallel tests; the
        // built-in test runner serializes tests within one binary by
        // default, but to be safe we restore the var on every exit
        // path even if the assertion fails.
        let prior = std::env::var(ENV_GRAMMAR_DB).ok();
        // Edition-2024 marks env mutation unsafe; the test runner
        // serializes tests within one binary, so the race window is
        // bounded.
        unsafe {
            std::env::set_var(ENV_GRAMMAR_DB, db.to_string_lossy().into_owned());
        }

        let req = Request::new(
            RequestId::from(11),
            Completion::METHOD.to_string(),
            lsp_types::CompletionParams {
                text_document_position: lsp_types::TextDocumentPositionParams {
                    text_document: lsp_types::TextDocumentIdentifier {
                        uri: lsp_types::Url::parse("file:///tmp/fake.nomx").unwrap(),
                    },
                    position: lsp_types::Position {
                        line: 0,
                        character: 0,
                    },
                },
                work_done_progress_params: Default::default(),
                partial_result_params: Default::default(),
                context: None,
            },
        );
        let resp = dispatch_request(req);

        // Restore env var before asserting so failures don't leak state.
        unsafe {
            match prior {
                Some(v) => std::env::set_var(ENV_GRAMMAR_DB, v),
                None => std::env::remove_var(ENV_GRAMMAR_DB),
            }
        }

        assert!(resp.error.is_none());
        let items: CompletionResponse =
            serde_json::from_value(resp.result.expect("completion result set")).unwrap();
        let labels: Vec<String> = match items {
            CompletionResponse::Array(a) => a.into_iter().map(|i| i.label).collect(),
            CompletionResponse::List(l) => l.items.into_iter().map(|i| i.label).collect(),
        };
        // Keyword still present.
        assert!(labels.iter().any(|l| l == "define"));
        // Both seeded pattern ids surface as completions.
        assert!(
            labels.iter().any(|l| l == "alpha-cache-pattern"),
            "missing alpha-cache-pattern; labels: {labels:?}"
        );
        assert!(
            labels.iter().any(|l| l == "beta-render-pattern"),
            "missing beta-render-pattern; labels: {labels:?}"
        );
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
                    position: lsp_types::Position {
                        line: 0,
                        character: 0,
                    },
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
        assert!(
            body.contains(SERVER_NAME),
            "hover body must name server: {body}"
        );
    }

    #[test]
    fn dispatch_execute_command_unknown_command_returns_method_not_found() {
        let params = lsp_types::ExecuteCommandParams {
            command: "nom.unknownCommand".into(),
            arguments: vec![],
            work_done_progress_params: Default::default(),
        };
        let req = Request::new(
            RequestId::from(10),
            ExecuteCommand::METHOD.to_string(),
            params,
        );
        let resp = dispatch_request(req);
        let err = resp.error.expect("unknown command must error");
        assert_eq!(err.code, lsp_server::ErrorCode::MethodNotFound as i32);
        assert!(err.message.contains("nom.unknownCommand"));
    }

    #[test]
    fn dispatch_execute_command_why_missing_prose_arg_returns_invalid_params() {
        let params = lsp_types::ExecuteCommandParams {
            command: CMD_WHY_THIS_NOM.into(),
            arguments: vec![], // missing prose + dict_path
            work_done_progress_params: Default::default(),
        };
        let req = Request::new(
            RequestId::from(11),
            ExecuteCommand::METHOD.to_string(),
            params,
        );
        let resp = dispatch_request(req);
        let err = resp.error.expect("missing args must error");
        assert_eq!(err.code, lsp_server::ErrorCode::InvalidParams as i32);
        assert!(err.message.contains("argument[0]"));
    }

    #[test]
    fn dispatch_execute_command_why_missing_dict_arg_returns_invalid_params() {
        let params = lsp_types::ExecuteCommandParams {
            command: CMD_WHY_THIS_NOM.into(),
            arguments: vec![serde_json::json!("some prose")], // only prose, no dict_path
            work_done_progress_params: Default::default(),
        };
        let req = Request::new(
            RequestId::from(12),
            ExecuteCommand::METHOD.to_string(),
            params,
        );
        let resp = dispatch_request(req);
        let err = resp.error.expect("missing dict_path must error");
        assert_eq!(err.code, lsp_server::ErrorCode::InvalidParams as i32);
        assert!(err.message.contains("argument[1]"));
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
