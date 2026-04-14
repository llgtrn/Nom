//! nom mcp serve — Model Context Protocol server exposing the dict
//! as queryable tools for LLM code authoring. Per user directive:
//! "the nomtu name which is in the word column is also part of
//! syntax"; this server lets an LLM enumerate + describe available
//! nomtu words while composing .nom source.
//!
//! Protocol: line-delimited JSON-RPC 2.0 over stdio. Hand-rolled
//! (no rmcp dep) — the protocol surface is ~100 LOC of serde_json
//! manipulation and adding `rmcp` would pull in an async runtime.
//!
//! Tools exposed:
//!   list_nomtu    — enumerate dict entries with optional filters
//!   get_nomtu     — fetch one entry by id or ≥8-char hex prefix
//!   search_nomtu  — substring search on the `describe` field

use nom_dict::dict::{
    body_kind_histogram, count_concept_members, count_entities, find_entries, get_concept_by_name,
    get_concept_members, get_entry, list_concepts, resolve_prefix, search_describe,
    status_histogram,
};
use nom_dict::{Concept, Dict, EntryFilter};
use nom_types::{EntryKind, EntryStatus};
use serde_json::{Value, json};
use std::io::{BufRead, Write};
use std::path::Path;

pub fn cmd_mcp_serve(dict_path: &Path) -> i32 {
    let dict = match Dict::try_open_from_nomdict_path(dict_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("nom mcp: cannot open dict at {}: {e}", dict_path.display());
            return 1;
        }
    };

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) if l.trim().is_empty() => continue,
            Ok(l) => l,
            Err(_) => break,
        };
        let req: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let _ = writeln!(
                    out,
                    "{}",
                    err_response(Value::Null, -32700, &format!("parse error: {e}"))
                );
                let _ = out.flush();
                continue;
            }
        };
        if let Some(resp) = handle_request(&dict, &req) {
            let _ = writeln!(out, "{resp}");
            let _ = out.flush();
        }
    }
    0
}

fn handle_request(dict: &Dict, req: &Value) -> Option<String> {
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let method = req.get("method").and_then(|v| v.as_str()).unwrap_or("");
    match method {
        "initialize" => Some(initialize_response(id)),
        // Notifications: no reply expected
        "initialized" | "notifications/initialized" => None,
        "tools/list" => Some(tools_list_response(id)),
        "tools/call" => Some(tools_call_response(dict, id, req.get("params"))),
        "ping" => Some(ok_response(id, json!({}))),
        other => Some(err_response(
            id,
            -32601,
            &format!("method not found: {other}"),
        )),
    }
}

// ── Protocol responses ────────────────────────────────────────────────

fn initialize_response(id: Value) -> String {
    ok_response(
        id,
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": {
                "name": "nom-mcp",
                "version": env!("CARGO_PKG_VERSION")
            }
        }),
    )
}

fn tools_list_response(id: Value) -> String {
    ok_response(
        id,
        json!({
            "tools": [
                {
                    "name": "list_nomtu",
                    "description": "Enumerate dict entries. Use this to discover what nomtu words (functions, structs, media, etc) are available for `use <word>@<hash>` references in .nom code. Returns up to `limit` rows.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "language": {
                                "type": "string",
                                "description": "Filter by source language (rust, typescript, nom, media, ...)"
                            },
                            "body_kind": {
                                "type": "string",
                                "description": "Filter by canonical format tag (bc, avif, png, ...)"
                            },
                            "status": {
                                "type": "string",
                                "description": "Filter by status: complete, partial, opaque"
                            },
                            "kind": {
                                "type": "string",
                                "description": "Filter by entry kind: function, module, media_unit, ..."
                            },
                            "limit": {
                                "type": "integer",
                                "description": "Max entries to return (default 50)"
                            }
                        }
                    }
                },
                {
                    "name": "get_nomtu",
                    "description": "Fetch a single entry by id (full 64-hex SHA-256) or ≥8-char hex prefix. Returns metadata + body_kind + body_bytes length but not the body bytes themselves.",
                    "inputSchema": {
                        "type": "object",
                        "required": ["hash"],
                        "properties": {
                            "hash": {
                                "type": "string",
                                "description": "Full 64-char hex id or ≥8-char unique prefix"
                            }
                        }
                    }
                },
                {
                    "name": "search_nomtu",
                    "description": "Substring search on the `describe` field. Useful for finding nomtu by what they do — e.g. `search_nomtu(\"hash sha256\")` returns entries whose describe mentions SHA-256 hashing.",
                    "inputSchema": {
                        "type": "object",
                        "required": ["query"],
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "Substring to match in describe (case-insensitive)"
                            },
                            "limit": {
                                "type": "integer",
                                "description": "Max entries to return (default 20)"
                            }
                        }
                    }
                },
                {
                    "name": "list_concepts",
                    "description": "Enumerate concepts — named domains grouping related nomtu (e.g. 'cryptography', 'image-codecs'). Each concept name is itself a valid Nom syntax token, addressable via `use <concept>@<hash>` in .nom source.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                },
                {
                    "name": "get_concept",
                    "description": "Get details and up to 50 member summaries for a named concept. Useful for understanding what nomtu entries belong to a given domain (e.g. 'cryptography'). The concept name is a first-class Nom syntax token.",
                    "inputSchema": {
                        "type": "object",
                        "required": ["name"],
                        "properties": {
                            "name": {
                                "type": "string",
                                "description": "Concept name exactly as returned by list_concepts"
                            }
                        }
                    }
                },
                {
                    "name": "criteria_proposals",
                    "description": "Given an app manifest (root page + optional extra root hashes), walk the dict closure and return a list of Proposals — gaps the LLM should fill by authoring new nomtu or lifting Partial ones to Complete. Each proposal carries a `kind` (missing_root, partial_entry, unbalanced_contract, no_tests, empty_closure), rationale, and suggested {entry_kind, word, concept}. This is the engine behind Dreaming mode: compile → proposals → LLM authors → recompile until `is_epic`.",
                    "inputSchema": {
                        "type": "object",
                        "required": ["manifest_hash"],
                        "properties": {
                            "manifest_hash": {
                                "type": "string",
                                "description": "Stable identity of the manifest being examined. May be any string; proposals relate to it via `target`."
                            },
                            "name": {
                                "type": "string",
                                "description": "Human-readable app name used in suggested_word templates."
                            },
                            "target": {
                                "type": "string",
                                "description": "Default target platform: web | desktop | mobile."
                            },
                            "root_page_hash": {
                                "type": "string",
                                "description": "Closure root; usually a Page entry id."
                            },
                            "includes": {
                                "type": "array",
                                "items": {"type": "string"},
                                "description": "Additional closure roots (data sources, actions, media)."
                            }
                        }
                    }
                },
                {
                    "name": "dict_stats",
                    "description": "Return total entry count + body_kind histogram + status histogram. Use to poll dict-health during authoring — e.g. after `nom author translate --write`, check how many Partial entries remain to lift to Complete. No arguments.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                },
                {
                    "name": "parse_nomx",
                    "description": "Parse a .nomx (natural-language Nom) source string and return the declaration shape — decl count + per-decl {kind, name, field_count/variant_count/body_statement_count}. On error, returns a span-carrying diagnostic. Use to validate LLM-generated .nomx before writing it to disk.",
                    "inputSchema": {
                        "type": "object",
                        "required": ["source"],
                        "properties": {
                            "source": {
                                "type": "string",
                                "description": "Raw .nomx source text"
                            }
                        }
                    }
                }
            ]
        }),
    )
}

// ── Tool dispatch ─────────────────────────────────────────────────────

fn tools_call_response(dict: &Dict, id: Value, params: Option<&Value>) -> String {
    let params = match params {
        Some(p) => p,
        None => return err_response(id, -32602, "missing params"),
    };
    let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(json!({}));
    match name {
        "list_nomtu" => call_list_nomtu(dict, id, &args),
        "get_nomtu" => call_get_nomtu(dict, id, &args),
        "search_nomtu" => call_search_nomtu(dict, id, &args),
        "list_concepts" => call_list_concepts(dict, id),
        "get_concept" => call_get_concept(dict, id, &args),
        "criteria_proposals" => call_criteria_proposals(dict, id, &args),
        "dict_stats" => call_dict_stats(dict, id),
        "parse_nomx" => call_parse_nomx(id, &args),
        _ => err_response(id, -32602, &format!("unknown tool: {name}")),
    }
}

fn call_list_nomtu(dict: &Dict, id: Value, args: &Value) -> String {
    let filter = EntryFilter {
        body_kind: args
            .get("body_kind")
            .and_then(|v| v.as_str())
            .map(String::from),
        language: args
            .get("language")
            .and_then(|v| v.as_str())
            .map(String::from),
        status: args
            .get("status")
            .and_then(|v| v.as_str())
            .map(EntryStatus::from_str),
        kind: args
            .get("kind")
            .and_then(|v| v.as_str())
            .map(EntryKind::from_str),
        limit: args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize,
    };
    let entries = match find_entries(&dict, &filter) {
        Ok(e) => e,
        Err(e) => return err_response(id, -32000, &format!("query failed: {e}")),
    };
    let items: Vec<Value> = entries
        .iter()
        .map(|e| {
            json!({
                "id": e.id,
                "word": e.word,
                "variant": e.variant,
                "kind": e.kind.as_str(),
                "language": e.language,
                "body_kind": e.body_kind,
                "status": e.status.as_str(),
                "describe": e.describe.clone().unwrap_or_default(),
            })
        })
        .collect();
    let summary = format!("{} entries matched.", items.len());
    ok_response(
        id,
        json!({
            "content": [
                { "type": "text", "text": summary },
                { "type": "text", "text": serde_json::to_string_pretty(&items).unwrap_or_default() }
            ]
        }),
    )
}

fn call_get_nomtu(dict: &Dict, id: Value, args: &Value) -> String {
    let hash = match args.get("hash").and_then(|v| v.as_str()) {
        Some(h) => h,
        None => return err_response(id, -32602, "missing required argument: hash"),
    };

    // If 64 chars, try exact lookup first; otherwise resolve prefix.
    let full_id = if hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()) {
        hash.to_string()
    } else {
        match resolve_prefix(&dict, hash) {
            Ok(fid) => fid,
            Err(e) => return err_response(id, -32000, &format!("{e}")),
        }
    };

    let entry = match get_entry(&dict, &full_id) {
        Ok(Some(e)) => e,
        Ok(None) => return err_response(id, -32000, &format!("entry not found: {full_id}")),
        Err(e) => return err_response(id, -32000, &format!("lookup failed: {e}")),
    };

    let body_bytes_len = entry.body_bytes.as_ref().map(|b| b.len()).unwrap_or(0);
    ok_response(
        id,
        json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&json!({
                    "id": entry.id,
                    "word": entry.word,
                    "variant": entry.variant,
                    "kind": entry.kind.as_str(),
                    "language": entry.language,
                    "body_kind": entry.body_kind,
                    "status": entry.status.as_str(),
                    "describe": entry.describe.unwrap_or_default(),
                    "concept": entry.concept.unwrap_or_default(),
                    "body_bytes_len": body_bytes_len,
                    "input_type": entry.contract.input_type,
                    "output_type": entry.contract.output_type,
                })).unwrap_or_default()
            }]
        }),
    )
}

fn call_search_nomtu(dict: &Dict, id: Value, args: &Value) -> String {
    let query = match args.get("query").and_then(|v| v.as_str()) {
        Some(q) => q,
        None => return err_response(id, -32602, "missing required argument: query"),
    };
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;

    let entries = match search_describe(&dict, query, limit) {
        Ok(e) => e,
        Err(e) => return err_response(id, -32000, &format!("search failed: {e}")),
    };
    let items: Vec<Value> = entries
        .iter()
        .map(|e| {
            json!({
                "id": e.id,
                "word": e.word,
                "kind": e.kind.as_str(),
                "language": e.language,
                "status": e.status.as_str(),
                "describe": e.describe.clone().unwrap_or_default(),
            })
        })
        .collect();
    let summary = format!("{} entries matched query {:?}.", items.len(), query);
    ok_response(
        id,
        json!({
            "content": [
                { "type": "text", "text": summary },
                { "type": "text", "text": serde_json::to_string_pretty(&items).unwrap_or_default() }
            ]
        }),
    )
}

fn call_list_concepts(dict: &Dict, id: Value) -> String {
    let concepts = match list_concepts(&dict) {
        Ok(v) => v,
        Err(e) => return err_response(id, -32000, &format!("query failed: {e}")),
    };
    let items: Vec<Value> = concepts
        .iter()
        .map(|c| {
            let count = count_concept_members(&dict, &c.id).unwrap_or(0);
            json!({
                "id": c.id,
                "name": c.name,
                "describe": c.describe,
                "member_count": count,
            })
        })
        .collect();
    let summary = format!("{} concept(s) found.", items.len());
    ok_response(
        id,
        json!({
            "content": [
                { "type": "text", "text": summary },
                { "type": "text", "text": serde_json::to_string_pretty(&items).unwrap_or_default() }
            ]
        }),
    )
}

fn call_get_concept(dict: &Dict, id: Value, args: &Value) -> String {
    let name = match args.get("name").and_then(|v| v.as_str()) {
        Some(n) => n,
        None => return err_response(id, -32602, "missing required argument: name"),
    };
    let concept: Concept = match get_concept_by_name(&dict, name) {
        Ok(Some(c)) => c,
        Ok(None) => return err_response(id, -32000, &format!("concept not found: {name}")),
        Err(e) => return err_response(id, -32000, &format!("lookup failed: {e}")),
    };
    let mut members = match get_concept_members(&dict, &concept.id) {
        Ok(m) => m,
        Err(e) => return err_response(id, -32000, &format!("member query failed: {e}")),
    };
    members.truncate(50);
    let member_items: Vec<Value> = members
        .iter()
        .map(|e| {
            json!({
                "id": e.id,
                "word": e.word,
                "kind": e.kind.as_str(),
                "language": e.language,
                "status": e.status.as_str(),
                "describe": e.describe.clone().unwrap_or_default(),
            })
        })
        .collect();
    let summary = format!(
        "Concept '{}': {} (showing {} member(s)).",
        concept.name,
        concept.describe.as_deref().unwrap_or("no description"),
        member_items.len()
    );
    ok_response(
        id,
        json!({
            "content": [
                { "type": "text", "text": summary },
                {
                    "type": "text",
                    "text": serde_json::to_string_pretty(&json!({
                        "concept": {
                            "id": concept.id,
                            "name": concept.name,
                            "describe": concept.describe,
                        },
                        "members": member_items,
                    })).unwrap_or_default()
                }
            ]
        }),
    )
}

fn call_criteria_proposals(dict: &Dict, id: Value, args: &Value) -> String {
    let manifest_hash = args
        .get("manifest_hash")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if manifest_hash.is_empty() {
        return err_response(id, -32602, "missing required argument: manifest_hash");
    }
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("app")
        .to_string();
    let target = args
        .get("target")
        .and_then(|v| v.as_str())
        .unwrap_or("web")
        .to_string();
    let root_page_hash = args
        .get("root_page_hash")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let includes: Vec<String> = args
        .get("includes")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let manifest = nom_app::AppManifest {
        manifest_hash,
        name,
        default_target: target,
        root_page_hash,
        data_sources: includes,
        actions: vec![],
        media_assets: vec![],
        settings: Value::Null,
    };
    let report = nom_app::dream_report(&manifest, dict);
    let summary = if report.is_epic {
        format!(
            "App is epic — score {}/{}. No further authoring needed.",
            report.app_score, report.score_threshold
        )
    } else {
        format!(
            "Score {}/{} with {} proposal(s). Query the dict (list_nomtu, \
             search_nomtu, get_concept) then author nomtu via `nom store add` \
             and re-run `criteria_proposals` until score ≥ {}. If the dict is \
             exhausted, ask the user whether to skip.",
            report.app_score,
            report.score_threshold,
            report.proposals.len(),
            report.score_threshold,
        )
    };
    ok_response(
        id,
        json!({
            "content": [
                {"type": "text", "text": summary},
                {
                    "type": "text",
                    "text": serde_json::to_string_pretty(&report).unwrap_or_default()
                }
            ]
        }),
    )
}

fn call_dict_stats(dict: &Dict, id: Value) -> String {
    let total = count_entities(&dict).unwrap_or(0);
    let body_hist = body_kind_histogram(&dict).unwrap_or_default();
    let status_hist = status_histogram(&dict).unwrap_or_default();

    let partial_count = status_hist
        .iter()
        .find(|(s, _)| s == "partial")
        .map(|(_, n)| *n)
        .unwrap_or(0);
    let complete_count = status_hist
        .iter()
        .find(|(s, _)| s == "complete")
        .map(|(_, n)| *n)
        .unwrap_or(0);
    let summary = if total == 0 {
        "Dict is empty.".to_string()
    } else {
        format!(
            "{total} entries ({complete_count} complete, {partial_count} partial). \
             Authoring loop: lift Partial entries to Complete via body authoring \
             + re-score via `criteria_proposals`."
        )
    };

    ok_response(
        id,
        json!({
            "content": [
                {"type": "text", "text": summary},
                {
                    "type": "text",
                    "text": serde_json::to_string_pretty(&json!({
                        "total": total,
                        "body_kind_histogram": body_hist
                            .iter()
                            .map(|(k, n)| json!({"body_kind": k, "count": n}))
                            .collect::<Vec<_>>(),
                        "status_histogram": status_hist
                            .iter()
                            .map(|(s, n)| json!({"status": s, "count": n}))
                            .collect::<Vec<_>>(),
                    })).unwrap_or_default()
                }
            ]
        }),
    )
}

fn call_parse_nomx(id: Value, args: &Value) -> String {
    use nom_concept::stages::{PipelineOutput, run_pipeline};

    let source = args.get("source").and_then(|v| v.as_str()).unwrap_or("");
    if source.is_empty() {
        return err_response(id, -32602, "missing required argument: source");
    }

    match run_pipeline(source) {
        Ok(out) => {
            let (surface, count) = match &out {
                PipelineOutput::Nom(f) => ("concept", f.concepts.len()),
                PipelineOutput::Nomtu(f) => ("module", f.items.len()),
            };
            let summary = format!(
                "Parsed {} {} declaration(s) via merged .nomx pipeline",
                count, surface
            );
            ok_response(
                id,
                json!({
                    "content": [
                        {"type": "text", "text": summary},
                        {
                            "type": "text",
                            "text": serde_json::to_string_pretty(&json!({
                                "surface": surface,
                                "decl_count": count,
                            })).unwrap_or_default()
                        }
                    ]
                }),
            )
        }
        Err(e) => {
            let msg = format!(
                "Parse error at byte {} ({}): {}",
                e.position,
                e.diag_id(),
                e.detail
            );
            ok_response(
                id,
                json!({
                    "content": [
                        {"type": "text", "text": msg},
                        {
                            "type": "text",
                            "text": serde_json::to_string_pretty(&json!({
                                "error": e.detail,
                                "diag_id": e.diag_id(),
                                "position": e.position,
                            })).unwrap_or_default()
                        }
                    ],
                    "isError": true
                }),
            )
        }
    }
}

// ── JSON-RPC helpers ──────────────────────────────────────────────────

fn ok_response(id: Value, result: Value) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string()
}

fn err_response(id: Value, code: i64, message: &str) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    })
    .to_string()
}
