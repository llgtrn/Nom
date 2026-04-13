//! `nom store` core CLI commands.
//!
//! Contains `cmd_store_{add,get,closure,verify,stats,list,gc}` plus the two
//! public helpers `try_build_by_hash` and `materialize_closure_body` that
//! are consumed by the build pipeline.
//!
//! All shared helpers (`open_dict`, `resolve_prefix`, `json_array`, etc.)
//! live in the parent `store::mod` at `pub(super)` visibility and are
//! accessed here via `super::`.

use std::path::Path;

use nom_ast::{Declaration, SourceFile, Statement};
use nom_dict::{EntryFilter, NomDict};
use nom_parser::parse_source;
use nom_resolver::v2::{ResolutionTable, resolve_use_statements};
use nom_types::{Contract, Entry, EntryKind, EntryStatus};
use sha2::{Digest, Sha256};

use super::{
    chrono_like_now, escape_json, json_array, load_roots, open_dict, resolve_prefix, truncate,
};

// ── Private helpers (commands-only) ──────────────────────────────────

/// Compile an already-parsed Nom `SourceFile` to LLVM bitcode bytes.
/// Uses an empty in-memory resolver; missing word-refs produce a degraded
/// plan (same as `plan_unchecked`) but never block compilation.
fn compile_source_to_bc(sf: &SourceFile, _raw_source: &str) -> Result<Vec<u8>, String> {
    let resolver = nom_resolver::Resolver::open_in_memory()
        .map_err(|e| format!("resolver: {e}"))?;
    let planner = nom_planner::Planner::new(&resolver);
    let plan = planner
        .plan_unchecked(sf)
        .map_err(|e| format!("plan: {e}"))?;
    let output = nom_llvm::compile(&plan)
        .map_err(|e| format!("codegen: {e}"))?;
    Ok(output.bitcode)
}

/// Derive a Contract from the first ContractStmt in a declaration, or
/// return Contract::default() if none exists.
fn contract_from_decl(decl: &Declaration) -> Contract {
    for stmt in &decl.statements {
        if let Statement::Contract(cs) = stmt {
            let input = if cs.inputs.is_empty() {
                None
            } else {
                Some(
                    cs.inputs
                        .iter()
                        .map(|p| match &p.typ {
                            Some(t) => format!("{}: {}", p.name.name, t.name),
                            None => p.name.name.clone(),
                        })
                        .collect::<Vec<_>>()
                        .join(", "),
                )
            };
            let output = if cs.outputs.is_empty() {
                None
            } else {
                Some(
                    cs.outputs
                        .iter()
                        .map(|p| match &p.typ {
                            Some(t) => t.name.clone(),
                            None => p.name.name.clone(),
                        })
                        .collect::<Vec<_>>()
                        .join(", "),
                )
            };
            // Expr pretty-print isn't free; store debug repr as a
            // stopgap. Canonical id hashes the AST not this string.
            let pre = if cs.preconditions.is_empty() {
                None
            } else {
                Some(format!("{:?}", cs.preconditions))
            };
            let post = if cs.postconditions.is_empty() {
                None
            } else {
                Some(format!("{:?}", cs.postconditions))
            };
            return Contract {
                input_type: input,
                output_type: output,
                pre,
                post,
            };
        }
    }
    Contract::default()
}

fn describe_from_decl(decl: &Declaration) -> Option<String> {
    for stmt in &decl.statements {
        if let Statement::Describe(d) = stmt {
            return Some(d.text.clone());
        }
    }
    None
}

fn kind_from_classifier(c: nom_ast::Classifier) -> EntryKind {
    match c {
        nom_ast::Classifier::Nom => EntryKind::Function,
        nom_ast::Classifier::Flow => EntryKind::Function,
        nom_ast::Classifier::Test => EntryKind::TestCase,
        nom_ast::Classifier::Store => EntryKind::Schema,
        _ => EntryKind::Other,
    }
}

/// Try to resolve every use-statement. Names that don't resolve (or
/// collide) are returned alongside the partial table — the caller
/// decides whether to treat them as warnings or errors.
fn resolve_uses_best_effort(sf: &SourceFile, dict: &NomDict) -> (ResolutionTable, Vec<String>) {
    match resolve_use_statements(sf, dict) {
        Ok(t) => (t, Vec::new()),
        Err(e) => {
            // Downgrade the error to a single-name diagnostic; collect
            // remaining uses that still resolve so the entry can at
            // least be indexed by its known deps.
            let mut table = ResolutionTable::new();
            let mut missing: Vec<String> = Vec::new();
            let err_name = match &e {
                nom_resolver::v2::ResolveError::NotFound { name, .. } => name.clone(),
                nom_resolver::v2::ResolveError::Ambiguous { name, .. } => name.clone(),
                nom_resolver::v2::ResolveError::UnknownHash { hash, .. } => hash.clone(),
                nom_resolver::v2::ResolveError::AmbiguousHash { hash, .. } => hash.clone(),
            };
            missing.push(err_name);
            // Walk uses directly so surviving resolutions aren't lost.
            for decl in &sf.declarations {
                for stmt in &decl.statements {
                    if let Statement::Use(u) = stmt {
                        if let nom_ast::UseImport::Single(ident) = &u.imports {
                            match dict.find_by_word(&ident.name) {
                                Ok(entries) if entries.len() == 1 => {
                                    table.insert(ident.name.clone(), entries[0].id.clone());
                                }
                                Ok(entries) if entries.is_empty() => {
                                    if !missing.contains(&ident.name) {
                                        missing.push(ident.name.clone());
                                    }
                                }
                                _ => {
                                    if !missing.contains(&ident.name) {
                                        missing.push(ident.name.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
            (table, missing)
        }
    }
}

// ── Public CLI entry points ──────────────────────────────────────────

pub fn cmd_store_add(file: &Path, dict: &Path, json: bool) -> i32 {
    let source = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("nom: cannot read {}: {e}", file.display());
            return 1;
        }
    };
    let sf = match parse_source(&source) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("nom: parse error: {e}");
            return 1;
        }
    };

    // Compile the .nom source to LLVM bitcode.  The plan is built from the
    // parsed SourceFile so we don't parse twice.
    let bc_bytes = match compile_source_to_bc(&sf, &source) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("nom: compile error: {e}");
            return 1;
        }
    };

    let dict_db = match open_dict(dict) {
        Some(d) => d,
        None => return 1,
    };

    // Resolve use-statements; missing refs are diagnostics, not hard errors.
    let (table, missing) = resolve_uses_best_effort(&sf, &dict_db);

    // Status: Partial if any refs failed to resolve, else Complete.
    let status = if missing.is_empty() {
        EntryStatus::Complete
    } else {
        EntryStatus::Partial
    };

    // Content-addressed id from the compiled bitcode.
    let mut ids: Vec<String> = Vec::new();
    let mut refs: Vec<String> = Vec::new();

    for decl in &sf.declarations {
        let contract = contract_from_decl(decl);
        let word = decl.name.name.clone();
        let kind = kind_from_classifier(decl.classifier);
        let describe = describe_from_decl(decl);

        // Use sha256 of bc_bytes as the canonical id.
        let mut h = Sha256::new();
        h.update(&bc_bytes);
        h.update(word.as_bytes()); // per-decl disambiguation
        let id = format!("{:x}", h.finalize());

        let entry = Entry {
            id: id.clone(),
            word,
            variant: None,
            kind,
            language: "nom".to_string(),
            describe,
            concept: None,
            body: None,
            body_nom: None,
            body_bytes: Some(bc_bytes.clone()),
            body_kind: Some(nom_types::body_kind::BC.to_owned()),
            contract,
            status,
            translation_score: None,
            is_canonical: true,
            deprecated_by: None,
            created_at: chrono_like_now(),
            updated_at: None,
        };

        if let Err(e) = dict_db.upsert_entry(&entry) {
            eprintln!("nom: upsert error for {id}: {e}");
            return 1;
        }

        // Populate entry_refs from the resolution table. Only refs whose
        // target exists in the dict may be stored — FK would reject the
        // others. The spec says missing refs go to diagnostics already.
        for target in table.values() {
            if dict_db.get_entry(target).ok().flatten().is_some() {
                let _ = dict_db.add_ref(&id, target);
                refs.push(target.clone());
            }
        }

        ids.push(id);
    }

    if json {
        let refs_json = json_array(&refs);
        let missing_json = json_array(&missing);
        // Emit the primary id + resolved refs + still-missing names.
        let primary = ids.first().cloned().unwrap_or_default();
        println!(
            "{{\"id\":\"{}\",\"status\":\"{}\",\"refs\":{},\"missing\":{}}}",
            primary,
            status.as_str(),
            refs_json,
            missing_json
        );
    } else {
        for id in &ids {
            println!("{id}");
        }
        for name in &missing {
            eprintln!("nom: warning: unresolved use `{name}`");
        }
    }
    0
}

pub fn cmd_store_get(hash: &str, dict: &Path, json: bool) -> i32 {
    let dict_db = match open_dict(dict) {
        Some(d) => d,
        None => return 1,
    };
    let id = match resolve_prefix(&dict_db, hash) {
        Ok(id) => id,
        Err(msg) => {
            eprintln!("{msg}");
            return 1;
        }
    };
    let entry = match dict_db.get_entry(&id) {
        Ok(Some(e)) => e,
        Ok(None) => {
            eprintln!("nom: entry not found: {id}");
            return 1;
        }
        Err(e) => {
            eprintln!("nom: dict error: {e}");
            return 1;
        }
    };
    let meta = dict_db.get_meta(&id).unwrap_or_default();

    if json {
        // Minimal hand-rolled JSON to avoid pulling in a whole serializer
        // for a single command path. Strings are JSON-escaped.
        let body_json = entry
            .body_nom
            .as_deref()
            .map(|s| format!("\"{}\"", escape_json(s)))
            .unwrap_or_else(|| "null".to_string());
        let describe_json = entry
            .describe
            .as_deref()
            .map(|s| format!("\"{}\"", escape_json(s)))
            .unwrap_or_else(|| "null".to_string());
        let meta_json: Vec<String> = meta
            .iter()
            .map(|(k, v)| format!("{{\"key\":\"{}\",\"value\":\"{}\"}}", escape_json(k), escape_json(v)))
            .collect();
        println!(
            "{{\"id\":\"{}\",\"word\":\"{}\",\"kind\":\"{}\",\"language\":\"{}\",\"status\":\"{}\",\"describe\":{},\"body_nom\":{},\"meta\":[{}]}}",
            entry.id,
            escape_json(&entry.word),
            entry.kind.as_str(),
            escape_json(&entry.language),
            entry.status.as_str(),
            describe_json,
            body_json,
            meta_json.join(","),
        );
    } else {
        println!("id:       {}", entry.id);
        println!("word:     {}", entry.word);
        println!("kind:     {}", entry.kind.as_str());
        println!("language: {}", entry.language);
        println!("status:   {}", entry.status.as_str());
        if let Some(bk) = &entry.body_kind {
            println!("body_kind: {bk}");
        }
        if let Some(bb) = &entry.body_bytes {
            println!("body_bytes: {} bytes", bb.len());
        }
        if let Some(d) = &entry.describe {
            println!("describe: {d}");
        }
        if let Some(it) = &entry.contract.input_type {
            println!("input:    {it}");
        }
        if let Some(ot) = &entry.contract.output_type {
            println!("output:   {ot}");
        }
        if let Some(pre) = &entry.contract.pre {
            println!("pre:      {pre}");
        }
        if let Some(post) = &entry.contract.post {
            println!("post:     {post}");
        }
        if let Some(body) = &entry.body_nom {
            println!("--- body_nom ---");
            println!("{body}");
            println!("--- end body ---");
        }
        if !meta.is_empty() {
            println!("--- meta ---");
            for (k, v) in &meta {
                println!("  {k} = {v}");
            }
        }
    }
    0
}

pub fn cmd_store_closure(hash: &str, dict: &Path, json: bool) -> i32 {
    let dict_db = match open_dict(dict) {
        Some(d) => d,
        None => return 1,
    };
    let id = match resolve_prefix(&dict_db, hash) {
        Ok(id) => id,
        Err(msg) => {
            eprintln!("{msg}");
            return 1;
        }
    };
    let closure = match dict_db.closure(&id) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("nom: closure error: {e}");
            return 1;
        }
    };
    if json {
        println!("{}", json_array(&closure));
    } else {
        for h in &closure {
            println!("{h}");
        }
    }
    0
}

pub fn cmd_store_verify(hash: &str, dict: &Path, strict: bool) -> i32 {
    let dict_db = match open_dict(dict) {
        Some(d) => d,
        None => return 1,
    };
    let id = match resolve_prefix(&dict_db, hash) {
        Ok(id) => id,
        Err(msg) => {
            eprintln!("{msg}");
            return 1;
        }
    };
    let closure = match dict_db.closure(&id) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("nom: closure error: {e}");
            return 1;
        }
    };

    let mut partial = 0usize;
    let mut opaque = 0usize;
    let mut broken: Vec<(String, String)> = Vec::new();

    for node in &closure {
        match dict_db.get_entry(node) {
            Ok(Some(e)) => match e.status {
                EntryStatus::Partial => partial += 1,
                EntryStatus::Opaque => opaque += 1,
                EntryStatus::Complete => {}
            },
            _ => {}
        }
        let refs = dict_db.get_refs(node).unwrap_or_default();
        for r in refs {
            if dict_db.get_entry(&r).ok().flatten().is_none() {
                broken.push((node.clone(), r));
            }
        }
    }

    println!("total:   {}", closure.len());
    println!("partial: {partial}");
    println!("opaque:  {opaque}");
    println!("broken:  {}", broken.len());
    for (from, to) in &broken {
        println!("  broken: {from} -> {to}");
    }

    if !broken.is_empty() {
        return 2;
    }
    if strict && (partial > 0 || opaque > 0) {
        return 2;
    }
    0
}

/// `nom store stats [--json]`
///
/// Prints total entry count + §4.4.6 body_kind histogram. Quick
/// operator overview of the dict — how many entries, how many have
/// cached bitcode, how many are untagged (legacy pre-§4.4.6 rows),
/// how many are media.
pub fn cmd_store_stats(dict: &Path, json: bool) -> i32 {
    let dict_db = match open_dict(dict) {
        Some(d) => d,
        None => return 1,
    };
    let total = dict_db.count().unwrap_or(0);
    let concept_defs_count = dict_db.count_concept_defs().unwrap_or(0);
    let words_v2_count = dict_db.count_words_v2().unwrap_or(0);
    let required_axes_count = dict_db.count_required_axes().unwrap_or(0);
    let body_hist = match dict_db.body_kind_histogram() {
        Ok(h) => h,
        Err(e) => {
            eprintln!("nom: dict error: {e}");
            return 1;
        }
    };
    let status_hist = match dict_db.status_histogram() {
        Ok(h) => h,
        Err(e) => {
            eprintln!("nom: dict error: {e}");
            return 1;
        }
    };
    if json {
        let body_pairs: Vec<String> = body_hist
            .iter()
            .map(|(k, n)| format!("{{\"body_kind\":\"{}\",\"count\":{n}}}", escape_json(k)))
            .collect();
        let status_pairs: Vec<String> = status_hist
            .iter()
            .map(|(s, n)| format!("{{\"status\":\"{}\",\"count\":{n}}}", escape_json(s)))
            .collect();
        println!(
            "{{\"total\":{total},\"concept_defs\":{concept_defs_count},\"words_v2\":{words_v2_count},\"required_axes\":{required_axes_count},\"body_kind_histogram\":[{}],\"status_histogram\":[{}]}}",
            body_pairs.join(","),
            status_pairs.join(","),
        );
    } else {
        println!("total entries: {total}");
        println!("concept_defs (DB1):  {concept_defs_count}");
        println!("words_v2 (DB2):      {words_v2_count}");
        println!("required_axes (M7a): {required_axes_count}");
        println!();
        println!("body_kind histogram:");
        if body_hist.is_empty() {
            println!("  (empty)");
        } else {
            for (kind, count) in &body_hist {
                let pct = if total == 0 {
                    0.0
                } else {
                    100.0 * (*count as f64) / (total as f64)
                };
                println!("  {kind:<14} {count:>8}  ({pct:.1}%)");
            }
        }
        println!();
        println!("status histogram:");
        if status_hist.is_empty() {
            println!("  (empty)");
        } else {
            for (status, count) in &status_hist {
                let pct = if total == 0 {
                    0.0
                } else {
                    100.0 * (*count as f64) / (total as f64)
                };
                println!("  {status:<14} {count:>8}  ({pct:.1}%)");
            }
        }
    }
    0
}

/// `nom store list [--body-kind <k>] [--language <l>] [--status <s>] [--kind <k>] [--limit N] [--json]`
///
/// Multi-axis filter query over the v2 DIDS store. All filters are AND-composed;
/// omitting all filters returns the first `limit` entries ordered by id, which
/// is a useful operator overview of what's in the dict.
pub fn cmd_store_list(
    dict: &Path,
    body_kind: Option<&str>,
    language: Option<&str>,
    status: Option<&str>,
    kind: Option<&str>,
    limit: usize,
    json: bool,
) -> i32 {
    let dict_db = match open_dict(dict) {
        Some(d) => d,
        None => return 1,
    };

    // Validate body_kind against the §4.4.6 known-tag list.
    if let Some(bk) = body_kind {
        if !nom_types::body_kind::is_known(bk) {
            eprintln!(
                "nom: unknown body_kind: {bk}. Known: {}",
                nom_types::body_kind::ALL.join(", ")
            );
            return 1;
        }
    }

    // Parse status string into enum (unknown values → error with hint).
    let status_enum = if let Some(s) = status {
        let parsed = EntryStatus::from_str(s);
        // from_str falls back to Partial for unknown input; detect mismatch.
        if parsed.as_str() != s {
            eprintln!(
                "nom: unknown status: {s}. Known: complete, partial, opaque"
            );
            return 1;
        }
        Some(parsed)
    } else {
        None
    };

    // Parse kind string into enum (unknown values → Other, which is valid;
    // but warn if the string doesn't round-trip, indicating a typo).
    let kind_enum = if let Some(k) = kind {
        let parsed = EntryKind::from_str(k);
        if parsed.as_str() != k {
            eprintln!(
                "nom: unknown kind: {k}. Known: {}",
                EntryKind::ALL
                    .iter()
                    .map(|e| e.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            return 1;
        }
        Some(parsed)
    } else {
        None
    };

    let filter = EntryFilter {
        body_kind: body_kind.map(String::from),
        language: language.map(String::from),
        status: status_enum,
        kind: kind_enum,
        limit,
    };

    let entries = match dict_db.find_entries(&filter) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("nom: dict error: {e}");
            return 1;
        }
    };

    if entries.is_empty() {
        eprintln!("nom: no entries match (check --language/--status/--kind values)");
        return 0;
    }

    if json {
        for e in &entries {
            let bk = e
                .body_kind
                .as_deref()
                .map(|k| format!("\"{}\"", escape_json(k)))
                .unwrap_or_else(|| "null".into());
            println!(
                "{{\"id\":\"{}\",\"word\":\"{}\",\"kind\":\"{}\",\"language\":\"{}\",\"status\":\"{}\",\"body_kind\":{}}}",
                e.id,
                escape_json(&e.word),
                e.kind.as_str(),
                escape_json(&e.language),
                e.status.as_str(),
                bk,
            );
        }
    } else {
        println!(
            "{:<18} {:<20} {:<12} {:<12} {:<10} {}",
            "id (prefix)", "word", "kind", "language", "status", "body_kind"
        );
        for e in &entries {
            let id_pref = &e.id[..e.id.len().min(16)];
            println!(
                "{id_pref:<18} {:<20} {:<12} {:<12} {:<10} {}",
                truncate(&e.word, 20),
                e.kind.as_str(),
                truncate(&e.language, 12),
                e.status.as_str(),
                e.body_kind.as_deref().unwrap_or("-"),
            );
        }
        println!("{} entries", entries.len());
    }
    0
}

pub fn cmd_store_gc(dict: &Path, dry_run: bool) -> i32 {
    let dict_db = match open_dict(dict) {
        Some(d) => d,
        None => return 1,
    };

    let roots = match load_roots() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("nom: warning: could not read roots file: {e}");
            Vec::new()
        }
    };

    // Compute union of closures. Missing roots produce a diagnostic.
    let mut keep: std::collections::HashSet<String> = std::collections::HashSet::new();
    for root in &roots {
        let resolved = match resolve_prefix(&dict_db, root) {
            Ok(id) => id,
            Err(msg) => {
                eprintln!("nom: warning: gc root skipped: {msg}");
                continue;
            }
        };
        match dict_db.closure(&resolved) {
            Ok(c) => {
                for h in c {
                    keep.insert(h);
                }
            }
            Err(e) => {
                eprintln!("nom: warning: closure failed for {resolved}: {e}");
            }
        }
    }

    // Enumerate all ids.
    let mut stmt = match dict_db
        .connection()
        .prepare("SELECT id FROM entries ORDER BY id")
    {
        Ok(s) => s,
        Err(e) => {
            eprintln!("nom: gc query error: {e}");
            return 1;
        }
    };
    let all_ids: Vec<String> = match stmt
        .query_map([], |row| row.get::<_, String>(0))
        .and_then(|rows| rows.collect::<rusqlite::Result<Vec<_>>>())
    {
        Ok(v) => v,
        Err(e) => {
            eprintln!("nom: gc enumerate error: {e}");
            return 1;
        }
    };
    drop(stmt);

    let to_remove: Vec<String> = all_ids
        .iter()
        .filter(|id| !keep.contains(*id))
        .cloned()
        .collect();
    let kept = all_ids.len() - to_remove.len();

    if dry_run {
        for id in &to_remove {
            println!("would remove: {id}");
        }
        println!("would remove {} entries, keep {}", to_remove.len(), kept);
        return 0;
    }

    // FK cascade handles side tables.
    for id in &to_remove {
        if let Err(e) = dict_db
            .connection()
            .execute("DELETE FROM entries WHERE id = ?1", [id])
        {
            eprintln!("nom: gc delete error {id}: {e}");
            return 1;
        }
    }
    println!("removed {} entries, kept {}", to_remove.len(), kept);
    0
}

/// Return `Some(closure_ids)` if `arg` is a hash prefix that uniquely
/// matches a stored entry. Returns `None` when the arg looks like a
/// filesystem path (no unique id match), in which case the caller
/// should fall back to file-based build.
pub fn try_build_by_hash(arg: &str, dict: &Path) -> Option<Vec<String>> {
    let dict_db = open_dict(dict)?;
    // Only consider the positional arg as a hash if it's hex-ish and
    // reasonably long (≥ 8). Short hex prefixes like "ab" would ambigu-
    // ously shadow real files named "ab".
    if arg.len() < 8 || !arg.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let id = resolve_prefix(&dict_db, arg).ok()?;
    dict_db.closure(&id).ok()
}

/// Concatenate closure bodies in reverse BFS order so each entry's deps
/// appear above it. Missing bodies are skipped with a warning.
///
/// §4.4.6: each materialized chunk carries its `body_kind` tag in the
/// comment header so downstream tooling (and humans reading the
/// intermediate `.nom` artifact) can tell which entries are compiled
/// canonical artifacts vs. raw Nom source. A future build pass will
/// short-circuit `body_kind = "bc"` entries to link their cached
/// bitcode instead of re-transpiling; today this is informational only.
pub fn materialize_closure_body(dict: &Path, closure: &[String]) -> Option<String> {
    let dict_db = open_dict(dict)?;
    let mut parts: Vec<String> = Vec::new();
    let mut bc_cached = 0usize;
    for id in closure.iter().rev() {
        match dict_db.get_entry(id) {
            Ok(Some(e)) => {
                if let Some(body) = e.body_nom {
                    let kind_tag = e
                        .body_kind
                        .as_deref()
                        .map(|k| format!(", body_kind={k}"))
                        .unwrap_or_default();
                    if e.body_kind.as_deref() == Some(nom_types::body_kind::BC) {
                        bc_cached += 1;
                    }
                    parts.push(format!(
                        "# --- entry {id} ({}{kind_tag}) ---\n{body}",
                        e.word
                    ));
                } else {
                    eprintln!("nom: warning: no body_nom for {id}");
                }
            }
            _ => {
                eprintln!("nom: warning: entry missing: {id}");
            }
        }
    }
    if bc_cached > 0 {
        eprintln!(
            "nom: closure has {bc_cached} entries with cached bitcode (body_kind=bc) \
             — relinking path not yet wired; recompiling from source for now"
        );
    }
    Some(parts.join("\n\n"))
}
