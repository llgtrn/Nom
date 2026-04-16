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
    ExecuteCommandOptions, ExecuteCommandParams, GotoDefinitionResponse, HoverContents,
    HoverProviderCapability, InitializeParams, Location, MarkupContent, MarkupKind, OneOf, Range,
    ServerCapabilities,
    request::{Completion, ExecuteCommand, GotoDefinition, HoverRequest, Request as LspRequest},
};
use nom_dict::dict::find_entities_by_word;
use nom_dict::{Dict, EntityRow};
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
        definition_provider: Some(OneOf::Left(true)),
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
        GotoDefinition::METHOD => match cast::<GotoDefinition>(req) {
            Ok((id, params)) => handle_definition(id, params),
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
pub const ENV_DICT: &str = "NOM_DICT";

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
fn handle_definition(id: RequestId, params: lsp_types::GotoDefinitionParams) -> Response {
    let Some((token, _range)) = word_at_position(
        &params.text_document_position_params.text_document.uri,
        params.text_document_position_params.position,
    ) else {
        return null_response(id);
    };

    let Some(entity) = lookup_entity_for_token(&token).ok().flatten() else {
        return null_response(id);
    };
    let Some(path) = entity
        .origin_ref
        .as_deref()
        .or(entity.authored_in.as_deref())
    else {
        return null_response(id);
    };
    let Ok(uri) = lsp_types::Url::from_file_path(path) else {
        return null_response(id);
    };

    let range = find_declaration_range(path, &entity.word).unwrap_or_default();
    let response = GotoDefinitionResponse::Scalar(Location { uri, range });
    Response {
        id,
        result: Some(serde_json::to_value(response).expect("definition serializes")),
        error: None,
    }
}

/// Scan `source_path` for the first declaration of `word` and return its
/// precise LSP `Range`. Patterns checked (in order):
///
/// - `the function <word>` / `the data <word>` / `the record <word>` /
///   `the choice <word>` / `define <word>` / `to <word>`
///
/// Returns `None` when the file cannot be read or the word is not found,
/// in which case callers should fall back to `Range::default()` (line 0).
pub fn find_declaration_range(source_path: &str, word: &str) -> Option<Range> {
    let src = std::fs::read_to_string(source_path).ok()?;

    // Declaration prefixes to search for.  Each prefix is tried in order;
    // the first line containing `<prefix><word>` (with at least a word
    // boundary after) is used.
    const DECL_PREFIXES: &[&str] = &[
        "the function ",
        "the data ",
        "the record ",
        "the choice ",
        "define ",
        "to ",
    ];

    for (line_idx, line) in src.lines().enumerate() {
        for prefix in DECL_PREFIXES {
            if let Some(after_prefix) = line.find(prefix).map(|pos| pos + prefix.len()) {
                let candidate = &line[after_prefix..];
                if candidate.starts_with(word) {
                    // Verify it's a whole-word match: next char must be a
                    // non-word character or end of string.
                    let next_char = candidate.chars().nth(word.len());
                    let is_word_boundary = next_char.map(|c| !is_word_char(c)).unwrap_or(true);
                    if is_word_boundary {
                        let char_start = after_prefix as u32;
                        let char_end = char_start + word.len() as u32;
                        return Some(Range {
                            start: lsp_types::Position {
                                line: line_idx as u32,
                                character: char_start,
                            },
                            end: lsp_types::Position {
                                line: line_idx as u32,
                                character: char_end,
                            },
                        });
                    }
                }
            }
        }
    }
    None
}

fn handle_hover(id: RequestId, params: lsp_types::HoverParams) -> Response {
    if let Some((token, range)) = word_at_position(
        &params.text_document_position_params.text_document.uri,
        params.text_document_position_params.position,
    ) {
        if let Ok(Some(entity)) = lookup_entity_for_token(&token) {
            let dict_path = match std::env::var(ENV_DICT) {
                Ok(path) => path,
                Err(_) => return null_response(id),
            };
            let dict = match Dict::try_open_from_nomdict_path(std::path::Path::new(&dict_path)) {
                Ok(d) => d,
                Err(_) => return null_response(id),
            };
            let scores_text = nom_dict::get_scores(&dict, &entity.hash)
                .ok()
                .flatten()
                .map(|s| {
                    format!(
                        "security: {:.2}, reliability: {:.2}, performance: {:.2}, overall: {:.2}",
                        s.security.unwrap_or(0.0),
                        s.reliability.unwrap_or(0.0),
                        s.performance.unwrap_or(0.0),
                        s.overall_score.unwrap_or(0.0)
                    )
                });

            // Scan the source file for effects / retry / format_template.
            let source_meta = scan_source_meta(&entity);

            let md = format_hover_markdown(&entity, source_meta.as_ref(), scores_text.as_deref());
            let hover = lsp_types::Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: md,
                }),
                range: Some(range),
            };
            return Response {
                id,
                result: Some(serde_json::to_value(hover).expect("hover serializes")),
                error: None,
            };
        }
    }
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

/// Metadata extracted by scanning the raw source text of a .nomtu file.
/// Used to enrich hover with effects, retry policy, and format template
/// when the entity's source is available, without a full parse dependency.
#[derive(Debug, Default)]
pub struct SourceMeta {
    pub effects: Vec<(String, String)>,      // (valence, effect_name)
    pub retry_policy: Option<(u32, String)>, // (max_attempts, strategy)
    pub format_template: Option<String>,
}

/// Scan the source text of a .nomtu file for a named entity and extract
/// effects, retry policy, and format template using pattern matching.
/// Returns `None` if the source is unavailable or the entity is not found.
fn scan_source_meta(entity: &EntityRow) -> Option<SourceMeta> {
    let source_path = entity
        .origin_ref
        .as_deref()
        .or(entity.authored_in.as_deref())?;
    if !source_path.ends_with(".nomtu") {
        return None;
    }
    let src = std::fs::read_to_string(source_path).ok()?;

    // Find the block that names this entity's word.
    let needle = &entity.word;
    if !src.contains(needle.as_str()) {
        return None;
    }

    let mut meta = SourceMeta::default();
    for line in src.lines() {
        let trimmed = line.trim();
        // Effects: `benefit <name>.` or `hazard <name>.`
        if let Some(rest) = trimmed.strip_prefix("benefit ") {
            let name = rest.trim_end_matches('.');
            meta.effects
                .push(("benefit".into(), name.trim().to_string()));
        } else if let Some(rest) = trimmed.strip_prefix("hazard ") {
            let name = rest.trim_end_matches('.');
            meta.effects
                .push(("hazard".into(), name.trim().to_string()));
        } else if let Some(rest) = trimmed.strip_prefix("boon ") {
            let name = rest.trim_end_matches('.');
            meta.effects
                .push(("benefit".into(), name.trim().to_string()));
        } else if let Some(rest) = trimmed.strip_prefix("bane ") {
            let name = rest.trim_end_matches('.');
            meta.effects
                .push(("hazard".into(), name.trim().to_string()));
        }
        // Retry: `retry at-most N times [with S backoff].`
        if let Some(rest) = trimmed.strip_prefix("retry at-most ") {
            // rest looks like "3 times." or "3 times with exponential backoff."
            let rest = rest.trim_end_matches('.');
            let mut parts = rest.splitn(2, " times");
            if let Some(count_str) = parts.next() {
                if let Ok(n) = count_str.trim().parse::<u32>() {
                    let strategy = if let Some(after_times) = parts.next() {
                        if let Some(s) = after_times.trim().strip_prefix("with ") {
                            s.trim_end_matches(" backoff").trim().to_string()
                        } else {
                            "fixed".to_string()
                        }
                    } else {
                        "fixed".to_string()
                    };
                    meta.retry_policy = Some((n, strategy));
                }
            }
        }
        // Format template: `format "<template>".`
        if let Some(rest) = trimmed.strip_prefix("format \"") {
            let tmpl = rest.trim_end_matches('.').trim_end_matches('"');
            meta.format_template = Some(tmpl.to_string());
        }
    }
    Some(meta)
}

/// Format the enriched hover Markdown from an `EntityRow`, optional source
/// metadata (effects, retry_policy, format_template), and optional quality scores.
pub fn format_hover_markdown(
    entity: &EntityRow,
    source_meta: Option<&SourceMeta>,
    scores_text: Option<&str>,
) -> String {
    let mut out = String::new();

    // Header: **word** `kind`
    out.push_str(&format!("**{}** `{}`\n", entity.word, entity.kind));

    // Signature
    if let Some(sig) = entity.signature.as_deref() {
        out.push_str(&format!("\n**Signature:** `{sig}`\n"));
    }

    // Contracts section — parse the JSON array of ContractClause.
    let contract_lines = parse_contracts_json(entity.contracts.as_deref());
    if !contract_lines.is_empty() {
        out.push_str("\n**Contracts:**\n");
        for line in &contract_lines {
            out.push_str(&format!("- {line}\n"));
        }
    }

    // Effects section — from source scan when available.
    if let Some(meta) = source_meta {
        if !meta.effects.is_empty() {
            out.push_str("\n**Effects:**\n");
            for (valence, name) in &meta.effects {
                out.push_str(&format!("- {valence}: {name}\n"));
            }
        }

        // Retry policy
        if let Some((attempts, strategy)) = &meta.retry_policy {
            out.push_str(&format!(
                "\n**Retry:** at-most {attempts} times with {strategy} backoff\n"
            ));
        }

        // Format template
        if let Some(tmpl) = &meta.format_template {
            out.push_str(&format!("\n**Format template:** `{tmpl}`\n"));
        }
    }

    // Body kind
    if let Some(bk) = entity.body_kind.as_deref() {
        out.push_str(&format!("\n**Body kind:** `{bk}`\n"));
    }

    // Quality scores
    if let Some(s) = scores_text {
        out.push_str(&format!("\n**Scores:** {s}\n"));
    }

    // Source
    let source = entity
        .origin_ref
        .as_deref()
        .or(entity.authored_in.as_deref())
        .unwrap_or("source unavailable");
    out.push_str(&format!("\n**Source:** `{source}`\n"));

    out
}

/// Parse the JSON `contracts` text column (a `Vec<ContractClause>`) into display lines
/// like `"requires: the token is non-empty"` or `"ensures: the result is valid"`.
/// Returns an empty vec if the text is absent, empty, or not valid JSON.
fn parse_contracts_json(json: Option<&str>) -> Vec<String> {
    let Some(text) = json else { return vec![] };
    if text.is_empty() || text == "[]" {
        return vec![];
    }
    let Ok(clauses) = serde_json::from_str::<Vec<serde_json::Value>>(text) else {
        return vec![];
    };
    clauses
        .into_iter()
        .filter_map(|v| {
            // ContractClause serializes as {"Requires":"..."} or {"Ensures":"..."}
            if let Some(pred) = v.get("Requires").and_then(|r| r.as_str()) {
                Some(format!("requires: {pred}"))
            } else if let Some(pred) = v.get("Ensures").and_then(|r| r.as_str()) {
                Some(format!("ensures: {pred}"))
            } else {
                None
            }
        })
        .collect()
}

fn null_response(id: RequestId) -> Response {
    Response {
        id,
        result: Some(serde_json::Value::Null),
        error: None,
    }
}

fn lookup_entity_for_token(token: &str) -> Result<Option<EntityRow>, String> {
    let dict_path = match std::env::var(ENV_DICT) {
        Ok(path) => path,
        Err(_) => return Ok(None),
    };
    let dict = Dict::try_open_from_nomdict_path(std::path::Path::new(&dict_path))
        .map_err(|e| format!("open dict: {e}"))?;

    if token.len() >= 8 && token.chars().all(|c| c.is_ascii_hexdigit()) {
        if let Ok(hash) = nom_dict::resolve_prefix(&dict, token) {
            if let Ok(Some(row)) = nom_dict::find_entity(&dict, &hash) {
                return Ok(Some(row));
            }
        }
    }

    find_entities_by_word(&dict, token)
        .map(|mut rows| rows.drain(..).next())
        .map_err(|e| format!("lookup word: {e}"))
}

fn word_at_position(
    uri: &lsp_types::Url,
    position: lsp_types::Position,
) -> Option<(String, Range)> {
    let path = uri.to_file_path().ok()?;
    let text = std::fs::read_to_string(path).ok()?;
    let line = text.lines().nth(position.line as usize)?;
    let chars: Vec<char> = line.chars().collect();
    let mut idx = (position.character as usize).min(chars.len());
    if idx == chars.len() && idx > 0 {
        idx -= 1;
    }
    if !is_word_char(*chars.get(idx)?) && idx > 0 && is_word_char(chars[idx - 1]) {
        idx -= 1;
    }
    if !is_word_char(*chars.get(idx)?) {
        return None;
    }

    let mut start = idx;
    while start > 0 && is_word_char(chars[start - 1]) {
        start -= 1;
    }
    let mut end = idx + 1;
    while end < chars.len() && is_word_char(chars[end]) {
        end += 1;
    }

    Some((
        chars[start..end].iter().collect(),
        Range {
            start: lsp_types::Position {
                line: position.line,
                character: start as u32,
            },
            end: lsp_types::Position {
                line: position.line,
                character: end as u32,
            },
        },
    ))
}

fn is_word_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'
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
        assert!(matches!(caps.definition_provider, Some(OneOf::Left(true))));
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
    fn dispatch_hover_uses_dict_row_for_word_under_cursor() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.nomtu");
        std::fs::write(&source, "call hash_password now").unwrap();

        let dict_dir = dir.path().join("dict");
        let dict = Dict::open_dir(&dict_dir).unwrap();
        nom_dict::upsert_entity(
            &dict,
            &EntityRow {
                hash: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into(),
                word: "hash_password".into(),
                kind: "function".into(),
                signature: Some("given a password, returns a digest".into()),
                contracts: None,
                body_kind: None,
                body_size: None,
                origin_ref: Some(source.to_string_lossy().into_owned()),
                bench_ids: None,
                authored_in: None,
                composed_of: None,
                status: "complete".into(),
            },
        )
        .unwrap();
        drop(dict);

        let prior = std::env::var(ENV_DICT).ok();
        unsafe {
            std::env::set_var(ENV_DICT, dict_dir.to_string_lossy().into_owned());
        }

        let req = Request::new(
            RequestId::from(13),
            HoverRequest::METHOD.to_string(),
            lsp_types::HoverParams {
                text_document_position_params: lsp_types::TextDocumentPositionParams {
                    text_document: lsp_types::TextDocumentIdentifier {
                        uri: lsp_types::Url::from_file_path(&source).unwrap(),
                    },
                    position: lsp_types::Position {
                        line: 0,
                        character: 7,
                    },
                },
                work_done_progress_params: Default::default(),
            },
        );
        let resp = dispatch_request(req);

        unsafe {
            match prior {
                Some(v) => std::env::set_var(ENV_DICT, v),
                None => std::env::remove_var(ENV_DICT),
            }
        }

        assert!(resp.error.is_none());
        let hover: lsp_types::Hover =
            serde_json::from_value(resp.result.expect("hover result set")).unwrap();
        let body = match hover.contents {
            HoverContents::Markup(m) => m.value,
            _ => panic!("expected markup hover"),
        };
        assert!(body.contains("hash_password"), "hover body: {body}");
        assert!(body.contains("given a password"), "hover body: {body}");
    }

    #[test]
    fn dispatch_definition_returns_origin_ref_for_word_under_cursor() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("caller.nom");
        let def = dir.path().join("defs.nomtu");
        std::fs::write(&source, "use hash_password").unwrap();
        std::fs::write(
            &def,
            "the function hash_password is given a password, returns a digest.",
        )
        .unwrap();

        let dict_dir = dir.path().join("dict");
        let dict = Dict::open_dir(&dict_dir).unwrap();
        nom_dict::upsert_entity(
            &dict,
            &EntityRow {
                hash: "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210".into(),
                word: "hash_password".into(),
                kind: "function".into(),
                signature: Some("given a password, returns a digest".into()),
                contracts: None,
                body_kind: None,
                body_size: None,
                origin_ref: Some(def.to_string_lossy().into_owned()),
                bench_ids: None,
                authored_in: None,
                composed_of: None,
                status: "complete".into(),
            },
        )
        .unwrap();
        drop(dict);

        let prior = std::env::var(ENV_DICT).ok();
        unsafe {
            std::env::set_var(ENV_DICT, dict_dir.to_string_lossy().into_owned());
        }

        let req = Request::new(
            RequestId::from(14),
            GotoDefinition::METHOD.to_string(),
            lsp_types::GotoDefinitionParams {
                text_document_position_params: lsp_types::TextDocumentPositionParams {
                    text_document: lsp_types::TextDocumentIdentifier {
                        uri: lsp_types::Url::from_file_path(&source).unwrap(),
                    },
                    position: lsp_types::Position {
                        line: 0,
                        character: 5,
                    },
                },
                work_done_progress_params: Default::default(),
                partial_result_params: Default::default(),
            },
        );
        let resp = dispatch_request(req);

        unsafe {
            match prior {
                Some(v) => std::env::set_var(ENV_DICT, v),
                None => std::env::remove_var(ENV_DICT),
            }
        }

        assert!(resp.error.is_none());
        let response: GotoDefinitionResponse =
            serde_json::from_value(resp.result.expect("definition result set")).unwrap();
        let GotoDefinitionResponse::Scalar(location) = response else {
            panic!("expected scalar definition response");
        };
        assert_eq!(location.uri, lsp_types::Url::from_file_path(&def).unwrap());
    }

    #[test]
    fn dispatch_definition_returns_precise_source_range() {
        // Verifies that goto-definition resolves to the correct line and
        // character offsets within the definition file, not just line 0.
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("caller.nom");
        let def = dir.path().join("defs.nomtu");
        // Put the declaration on line 1 (0-indexed), preceded by a comment.
        std::fs::write(&source, "use hash_password").unwrap();
        std::fs::write(
            &def,
            "# module header\nthe function hash_password is given a password, returns a digest.\n",
        )
        .unwrap();

        let dict_dir = dir.path().join("dict2");
        let dict = Dict::open_dir(&dict_dir).unwrap();
        nom_dict::upsert_entity(
            &dict,
            &EntityRow {
                hash: "aaaa111111111111aaaa111111111111aaaa111111111111aaaa111111111111".into(),
                word: "hash_password".into(),
                kind: "function".into(),
                signature: Some("given a password, returns a digest".into()),
                contracts: None,
                body_kind: None,
                body_size: None,
                origin_ref: Some(def.to_string_lossy().into_owned()),
                bench_ids: None,
                authored_in: None,
                composed_of: None,
                status: "complete".into(),
            },
        )
        .unwrap();
        drop(dict);

        let prior = std::env::var(ENV_DICT).ok();
        unsafe {
            std::env::set_var(ENV_DICT, dict_dir.to_string_lossy().into_owned());
        }

        let req = Request::new(
            RequestId::from(15),
            GotoDefinition::METHOD.to_string(),
            lsp_types::GotoDefinitionParams {
                text_document_position_params: lsp_types::TextDocumentPositionParams {
                    text_document: lsp_types::TextDocumentIdentifier {
                        uri: lsp_types::Url::from_file_path(&source).unwrap(),
                    },
                    position: lsp_types::Position {
                        line: 0,
                        character: 5,
                    },
                },
                work_done_progress_params: Default::default(),
                partial_result_params: Default::default(),
            },
        );
        let resp = dispatch_request(req);

        unsafe {
            match prior {
                Some(v) => std::env::set_var(ENV_DICT, v),
                None => std::env::remove_var(ENV_DICT),
            }
        }

        assert!(resp.error.is_none());
        let response: GotoDefinitionResponse =
            serde_json::from_value(resp.result.expect("definition result set")).unwrap();
        let GotoDefinitionResponse::Scalar(location) = response else {
            panic!("expected scalar definition response");
        };
        assert_eq!(location.uri, lsp_types::Url::from_file_path(&def).unwrap());
        // Declaration is on line 1, after "the function " (13 chars).
        assert_eq!(
            location.range.start.line, 1,
            "expected declaration on line 1, got {}",
            location.range.start.line
        );
        assert_eq!(
            location.range.start.character, 13,
            "expected character offset 13 (after 'the function '), got {}",
            location.range.start.character
        );
        assert_eq!(
            location.range.end.character,
            13 + "hash_password".len() as u32,
            "end character should cover the word"
        );
    }

    #[test]
    fn find_declaration_range_returns_none_for_missing_file() {
        assert!(find_declaration_range("/nonexistent/path/file.nomtu", "foo").is_none());
    }

    #[test]
    fn find_declaration_range_returns_none_when_word_absent() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("x.nomtu");
        std::fs::write(&f, "the function other_word is given x, returns y.\n").unwrap();
        assert!(find_declaration_range(&f.to_string_lossy(), "missing_word").is_none());
    }

    #[test]
    fn find_declaration_range_finds_define_prefix() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("x.nomtu");
        std::fs::write(&f, "define greet that takes name and returns text.\n").unwrap();
        let range = find_declaration_range(&f.to_string_lossy(), "greet")
            .expect("should find define greet");
        assert_eq!(range.start.line, 0);
        // "define " = 7 chars
        assert_eq!(range.start.character, 7);
        assert_eq!(range.end.character, 7 + 5);
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

    // ── Enriched hover tests ─────────────────────────────────────────────────

    #[test]
    fn parse_contracts_json_empty_returns_empty() {
        assert!(parse_contracts_json(None).is_empty());
        assert!(parse_contracts_json(Some("")).is_empty());
        assert!(parse_contracts_json(Some("[]")).is_empty());
    }

    #[test]
    fn parse_contracts_json_requires_and_ensures() {
        let json =
            r#"[{"Requires":"the url is non-empty"},{"Ensures":"the response is received"}]"#;
        let lines = parse_contracts_json(Some(json));
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "requires: the url is non-empty");
        assert_eq!(lines[1], "ensures: the response is received");
    }

    #[test]
    fn parse_contracts_json_malformed_returns_empty() {
        let lines = parse_contracts_json(Some("not json at all"));
        assert!(lines.is_empty());
    }

    #[test]
    fn format_hover_markdown_includes_word_and_kind() {
        let entity = EntityRow {
            hash: "abc123".into(),
            word: "fetch_url".into(),
            kind: "function".into(),
            signature: Some("given url of text, returns text".into()),
            contracts: Some(r#"[{"Requires":"response is not empty"}]"#.into()),
            body_kind: Some("bc".into()),
            body_size: None,
            origin_ref: None,
            bench_ids: None,
            authored_in: None,
            composed_of: None,
            status: "complete".into(),
        };
        let md = format_hover_markdown(&entity, None, None);
        assert!(md.contains("**fetch_url**"), "missing word: {md}");
        assert!(md.contains("`function`"), "missing kind: {md}");
        assert!(
            md.contains("given url of text, returns text"),
            "missing sig: {md}"
        );
        assert!(
            md.contains("requires: response is not empty"),
            "missing contract: {md}"
        );
        assert!(
            md.contains("**Body kind:** `bc`"),
            "missing body_kind: {md}"
        );
    }

    #[test]
    fn format_hover_markdown_shows_effects_from_source_meta() {
        let entity = EntityRow {
            hash: "abc456".into(),
            word: "fetch_url".into(),
            kind: "function".into(),
            signature: None,
            contracts: None,
            body_kind: None,
            body_size: None,
            origin_ref: None,
            bench_ids: None,
            authored_in: None,
            composed_of: None,
            status: "complete".into(),
        };
        let meta = SourceMeta {
            effects: vec![
                ("benefit".into(), "cache_hit".into()),
                ("hazard".into(), "timeout".into()),
            ],
            retry_policy: Some((3, "exponential".into())),
            format_template: Some("{base}/{path}".into()),
        };
        let md = format_hover_markdown(&entity, Some(&meta), None);
        assert!(md.contains("benefit: cache_hit"), "missing benefit: {md}");
        assert!(md.contains("hazard: timeout"), "missing hazard: {md}");
        assert!(
            md.contains("at-most 3 times with exponential backoff"),
            "missing retry: {md}"
        );
        assert!(
            md.contains("{base}/{path}"),
            "missing format template: {md}"
        );
    }

    #[test]
    fn format_hover_markdown_shows_scores() {
        let entity = EntityRow {
            hash: "abc789".into(),
            word: "do_work".into(),
            kind: "function".into(),
            signature: None,
            contracts: None,
            body_kind: None,
            body_size: None,
            origin_ref: None,
            bench_ids: None,
            authored_in: None,
            composed_of: None,
            status: "complete".into(),
        };
        let md = format_hover_markdown(&entity, None, Some("security: 0.90, overall: 0.85"));
        assert!(md.contains("security: 0.90"), "missing scores: {md}");
    }

    #[test]
    fn scan_source_meta_extracts_effects_and_retry() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("example.nomtu");
        std::fs::write(
            &source,
            "the function fetch_url is given url, returns text.\n\
             requires the url is non-empty.\n\
             benefit cache_hit.\n\
             hazard timeout.\n\
             retry at-most 3 times with exponential backoff.\n",
        )
        .unwrap();

        let entity = EntityRow {
            hash: "deadbeef".into(),
            word: "fetch_url".into(),
            kind: "function".into(),
            signature: None,
            contracts: None,
            body_kind: None,
            body_size: None,
            origin_ref: Some(source.to_string_lossy().into_owned()),
            bench_ids: None,
            authored_in: None,
            composed_of: None,
            status: "complete".into(),
        };

        let meta = scan_source_meta(&entity).expect("meta should be found");
        assert!(
            meta.effects
                .iter()
                .any(|(v, n)| v == "benefit" && n == "cache_hit"),
            "missing benefit cache_hit: {:?}",
            meta.effects
        );
        assert!(
            meta.effects
                .iter()
                .any(|(v, n)| v == "hazard" && n == "timeout"),
            "missing hazard timeout: {:?}",
            meta.effects
        );
        assert_eq!(meta.retry_policy, Some((3, "exponential".into())));
    }

    #[test]
    fn dispatch_hover_shows_enriched_markdown_for_entity_with_contracts() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("auth.nomtu");
        std::fs::write(
            &source,
            "the function verify_token is given token of text, returns bool.\n\
             requires the token is non-empty.\n\
             ensures the session is valid.\n\
             benefit fast_path.\n",
        )
        .unwrap();

        let dict_dir = dir.path().join("dict");
        let dict = Dict::open_dir(&dict_dir).unwrap();
        nom_dict::upsert_entity(
            &dict,
            &EntityRow {
                hash: "1111111111111111111111111111111111111111111111111111111111111111".into(),
                word: "verify_token".into(),
                kind: "function".into(),
                signature: Some("given token of text, returns bool".into()),
                contracts: Some(
                    r#"[{"Requires":"the token is non-empty"},{"Ensures":"the session is valid"}]"#
                        .into(),
                ),
                body_kind: None,
                body_size: None,
                origin_ref: Some(source.to_string_lossy().into_owned()),
                bench_ids: None,
                authored_in: None,
                composed_of: None,
                status: "complete".into(),
            },
        )
        .unwrap();
        drop(dict);

        let prior = std::env::var(ENV_DICT).ok();
        unsafe {
            std::env::set_var(ENV_DICT, dict_dir.to_string_lossy().into_owned());
        }

        let req = Request::new(
            RequestId::from(20),
            HoverRequest::METHOD.to_string(),
            lsp_types::HoverParams {
                text_document_position_params: lsp_types::TextDocumentPositionParams {
                    text_document: lsp_types::TextDocumentIdentifier {
                        uri: lsp_types::Url::from_file_path(&source).unwrap(),
                    },
                    position: lsp_types::Position {
                        line: 0,
                        character: 15,
                    },
                },
                work_done_progress_params: Default::default(),
            },
        );
        let resp = dispatch_request(req);

        unsafe {
            match prior {
                Some(v) => std::env::set_var(ENV_DICT, v),
                None => std::env::remove_var(ENV_DICT),
            }
        }

        assert!(resp.error.is_none());
        let hover: lsp_types::Hover =
            serde_json::from_value(resp.result.expect("hover result")).unwrap();
        let body = match hover.contents {
            HoverContents::Markup(m) => m.value,
            _ => panic!("expected markup hover"),
        };
        assert!(body.contains("verify_token"), "missing word: {body}");
        assert!(
            body.contains("requires: the token is non-empty"),
            "missing requires: {body}"
        );
        assert!(
            body.contains("ensures: the session is valid"),
            "missing ensures: {body}"
        );
        assert!(
            body.contains("benefit: fast_path"),
            "missing effect: {body}"
        );
    }
}
