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

use nom_dict::{Draft, EntryFilter, NomDict};
use nom_types::{EntryKind, EntryStatus};
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::Path;

pub fn cmd_mcp_serve(dict_path: &Path) -> i32 {
    let dict = match NomDict::open_in_place(dict_path) {
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

fn handle_request(dict: &NomDict, req: &Value) -> Option<String> {
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let method = req.get("method").and_then(|v| v.as_str()).unwrap_or("");
    match method {
        "initialize" => Some(initialize_response(id)),
        // Notifications: no reply expected
        "initialized" | "notifications/initialized" => None,
        "tools/list" => Some(tools_list_response(id)),
        "tools/call" => Some(tools_call_response(dict, id, req.get("params"))),
        "ping" => Some(ok_response(id, json!({}))),
        other => Some(err_response(id, -32601, &format!("method not found: {other}"))),
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
                    "name": "list_drafts",
                    "description": "List all drafts (named domain collections of nomtu entries) with their member counts. Use this to discover available domains before calling get_draft for details.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                },
                {
                    "name": "get_draft",
                    "description": "Get details and up to 50 member summaries for a named draft. Useful for understanding what nomtu entries belong to a given domain (e.g. 'cryptography').",
                    "inputSchema": {
                        "type": "object",
                        "required": ["name"],
                        "properties": {
                            "name": {
                                "type": "string",
                                "description": "Draft name exactly as returned by list_drafts"
                            }
                        }
                    }
                }
            ]
        }),
    )
}

// ── Tool dispatch ─────────────────────────────────────────────────────

fn tools_call_response(dict: &NomDict, id: Value, params: Option<&Value>) -> String {
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
        "list_drafts" => call_list_drafts(dict, id),
        "get_draft" => call_get_draft(dict, id, &args),
        _ => err_response(id, -32602, &format!("unknown tool: {name}")),
    }
}

fn call_list_nomtu(dict: &NomDict, id: Value, args: &Value) -> String {
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
        limit: args
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(50) as usize,
    };
    let entries = match dict.find_entries(&filter) {
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

fn call_get_nomtu(dict: &NomDict, id: Value, args: &Value) -> String {
    let hash = match args.get("hash").and_then(|v| v.as_str()) {
        Some(h) => h,
        None => return err_response(id, -32602, "missing required argument: hash"),
    };

    // If 64 chars, try exact lookup first; otherwise resolve prefix.
    let full_id = if hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()) {
        hash.to_string()
    } else {
        match dict.resolve_prefix(hash) {
            Ok(fid) => fid,
            Err(e) => return err_response(id, -32000, &format!("{e}")),
        }
    };

    let entry = match dict.get_entry(&full_id) {
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

fn call_search_nomtu(dict: &NomDict, id: Value, args: &Value) -> String {
    let query = match args.get("query").and_then(|v| v.as_str()) {
        Some(q) => q,
        None => return err_response(id, -32602, "missing required argument: query"),
    };
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(20) as usize;

    let entries = match dict.search_describe(query, limit) {
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

fn call_list_drafts(dict: &NomDict, id: Value) -> String {
    let drafts = match dict.list_drafts() {
        Ok(v) => v,
        Err(e) => return err_response(id, -32000, &format!("query failed: {e}")),
    };
    let items: Vec<Value> = drafts
        .iter()
        .map(|dr| {
            let count = dict.count_draft_members(&dr.id).unwrap_or(0);
            json!({
                "id": dr.id,
                "name": dr.name,
                "describe": dr.describe,
                "member_count": count,
            })
        })
        .collect();
    let summary = format!("{} draft(s) found.", items.len());
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

fn call_get_draft(dict: &NomDict, id: Value, args: &Value) -> String {
    let name = match args.get("name").and_then(|v| v.as_str()) {
        Some(n) => n,
        None => return err_response(id, -32602, "missing required argument: name"),
    };
    let draft: Draft = match dict.get_draft_by_name(name) {
        Ok(Some(d)) => d,
        Ok(None) => return err_response(id, -32000, &format!("draft not found: {name}")),
        Err(e) => return err_response(id, -32000, &format!("lookup failed: {e}")),
    };
    let mut members = match dict.get_draft_members(&draft.id) {
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
        "Draft '{}': {} (showing {} member(s)).",
        draft.name,
        draft.describe.as_deref().unwrap_or("no description"),
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
                        "draft": {
                            "id": draft.id,
                            "name": draft.name,
                            "describe": draft.describe,
                        },
                        "members": member_items,
                    })).unwrap_or_default()
                }
            ]
        }),
    )
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
