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

use nom_dict::{Dict, EntryFilter, find_entries};
use nom_types::{EntryKind, EntryStatus};
use sha2::Digest;

use super::{escape_json, json_array, load_roots, open_dict, resolve_prefix, truncate};

// ── Private helpers (commands-only) ──────────────────────────────────

// ── Public CLI entry points ──────────────────────────────────────────

fn sha256_hex(bytes: &[u8]) -> String {
    let mut sh = sha2::Sha256::new();
    sh.update(bytes);
    format!("{:x}", sh.finalize())
}

fn canonical_decl_hash<T: serde::Serialize>(value: &T) -> Result<String, String> {
    let bytes = serde_json::to_vec(value).map_err(|e| format!("canonical serialize: {e}"))?;
    Ok(sha256_hex(&bytes))
}

fn repo_id_for_source(file: &Path) -> String {
    file.parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("default")
        .to_string()
}

fn composition_members_json(comp: &nom_concept::CompositionDecl) -> Option<String> {
    let hashes: Vec<String> = comp
        .composes
        .iter()
        .filter_map(|r| r.hash.clone())
        .collect();
    if hashes.is_empty() {
        let words: Vec<String> = comp.composes.iter().map(|r| r.word.clone()).collect();
        serde_json::to_string(&words).ok()
    } else {
        serde_json::to_string(&hashes).ok()
    }
}

pub fn cmd_store_add(file: &Path, dict: &Path, json: bool) -> i32 {
    let source = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("nom: cannot read {}: {e}", file.display());
            return 1;
        }
    };

    let pipeline_out = match nom_concept::stages::run_pipeline(&source) {
        Ok(out) => out,
        Err(e) => {
            eprintln!("nom: pipeline error: {} at offset {}", e.reason, e.position);
            return 1;
        }
    };

    // Use TryOpen to handle split-DB appropriately.
    let dict_db = match nom_dict::Dict::try_open_from_nomdict_path(dict) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("nom: dict error: {e}");
            return 1;
        }
    };

    let mut ids: Vec<String> = Vec::new();
    let repo_id = repo_id_for_source(file);
    let src_path = file.display().to_string();
    let src_hash = sha256_hex(source.as_bytes());

    match pipeline_out {
        nom_concept::stages::PipelineOutput::Nom(nom_file) => {
            for concept in &nom_file.concepts {
                let row = nom_dict::ConceptRow {
                    name: concept.name.clone(),
                    repo_id: repo_id.clone(),
                    intent: concept.intent.clone(),
                    index_into_db2: serde_json::to_string(&concept.index).unwrap_or_default(),
                    exposes: serde_json::to_string(&concept.exposes).unwrap_or_default(),
                    acceptance: serde_json::to_string(&concept.acceptance).unwrap_or_default(),
                    objectives: serde_json::to_string(&concept.objectives).unwrap_or_default(),
                    src_path: src_path.clone(),
                    src_hash: src_hash.clone(),
                    body_hash: None,
                };
                if let Err(e) = nom_dict::upsert_concept_def(&dict_db, &row) {
                    eprintln!("nom: failed to upsert concept {}: {e}", concept.name);
                    return 1;
                }
                ids.push(concept.name.clone());
            }
        }
        nom_concept::stages::PipelineOutput::Nomtu(nomtu_file) => {
            for item in &nomtu_file.items {
                match item {
                    nom_concept::NomtuItem::Entity(decl) => {
                        let id = match canonical_decl_hash(decl) {
                            Ok(h) => h,
                            Err(e) => {
                                eprintln!("nom: hash entity error for {}: {e}", decl.word);
                                return 1;
                            }
                        };
                        let row = nom_dict::EntityRow {
                            hash: id.clone(),
                            word: decl.word.clone(),
                            kind: decl.kind.clone(),
                            signature: Some(decl.signature.clone()),
                            contracts: Some(
                                serde_json::to_string(&decl.contracts).unwrap_or_default(),
                            ),
                            body_kind: None,
                            body_size: Some(source.len() as i64),
                            origin_ref: Some(src_path.clone()),
                            bench_ids: None,
                            authored_in: Some(src_path.clone()),
                            composed_of: None,
                        };
                        if let Err(e) = nom_dict::upsert_entity(&dict_db, &row) {
                            eprintln!("nom: upsert entity error for {}: {e}", decl.word);
                            return 1;
                        }
                        ids.push(id.clone());
                    }
                    nom_concept::NomtuItem::Composition(comp) => {
                        let id = match canonical_decl_hash(comp) {
                            Ok(h) => h,
                            Err(e) => {
                                eprintln!("nom: hash composition error for {}: {e}", comp.word);
                                return 1;
                            }
                        };
                        let row = nom_dict::EntityRow {
                            hash: id.clone(),
                            word: comp.word.clone(),
                            kind: "module".to_string(),
                            signature: comp.glue.clone(),
                            contracts: Some(
                                serde_json::to_string(&comp.contracts).unwrap_or_default(),
                            ),
                            body_kind: None,
                            body_size: Some(source.len() as i64),
                            origin_ref: Some(src_path.clone()),
                            bench_ids: None,
                            authored_in: Some(src_path.clone()),
                            composed_of: composition_members_json(comp),
                        };
                        if let Err(e) = nom_dict::upsert_entity(&dict_db, &row) {
                            eprintln!("nom: upsert composition error for {}: {e}", comp.word);
                            return 1;
                        }
                        ids.push(id.clone());
                    }
                }
            }
        }
    }

    let status_str = "Complete";
    if json {
        let refs_json = "[]";
        let missing_json = "[]";
        // Emit the primary id + resolved refs + still-missing names.
        let primary = ids.first().cloned().unwrap_or_default();
        println!(
            "{{\"id\":\"{}\",\"status\":\"{}\",\"refs\":{},\"missing\":{}}}",
            primary, status_str, refs_json, missing_json
        );
    } else {
        for id in &ids {
            println!("{id}");
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::{cmd_store_add, repo_id_for_source};
    use std::path::{Path, PathBuf};

    use nom_concept::stages::{PipelineOutput, run_pipeline};
    use nom_dict::Dict;
    use nom_dict::dict::find_concept_def;

    fn temp_dir(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!("nom-store-cmd-{tag}-{pid}-{nanos}"));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn repo_id_for_source_uses_parent_dir_name() {
        let path = Path::new(r"C:\work\sample_repo\concept.nomx");
        assert_eq!(repo_id_for_source(path), "sample_repo");
    }

    #[test]
    fn cmd_store_add_concept_uses_parent_repo_id() {
        let root = temp_dir("concept-add");
        let repo = root.join("sample_repo");
        std::fs::create_dir_all(&repo).expect("create repo dir");

        let src = repo.join("routing.nomx");
        std::fs::write(
            &src,
            "the concept routing is\n  intended to route incoming requests.\n  favor correctness.\n",
        )
        .expect("write source");

        let dict_path = root.join("nomdict.db");
        let code = cmd_store_add(&src, &dict_path, false);
        assert_eq!(code, 0, "cmd_store_add should succeed");

        let dict = Dict::try_open_from_nomdict_path(&dict_path).expect("open split dict");
        let row = find_concept_def(&dict, "routing")
            .expect("concept query")
            .expect("routing concept row");
        assert_eq!(row.repo_id, "sample_repo");
        assert_eq!(row.src_path, src.display().to_string());
    }

    #[test]
    fn canonical_decl_hash_matches_pipeline_json_for_entity() {
        let src = "the function fetch_url is given a url, returns text.\n";
        let out = run_pipeline(src).expect("pipeline");
        let entity = match out {
            PipelineOutput::Nomtu(file) => match file.items.into_iter().next().expect("one item") {
                nom_concept::NomtuItem::Entity(entity) => entity,
                _ => panic!("expected entity"),
            },
            _ => panic!("expected nomtu"),
        };

        let expected = super::sha256_hex(&serde_json::to_vec(&entity).expect("serialize entity"));
        let actual = super::canonical_decl_hash(&entity).expect("canonical hash");
        assert_eq!(actual, expected);
    }
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
            .map(|(k, v)| {
                format!(
                    "{{\"key\":\"{}\",\"value\":\"{}\"}}",
                    escape_json(k),
                    escape_json(v)
                )
            })
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
    let entities_count = dict_db.count_entities().unwrap_or(0);
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
            "{{\"total\":{total},\"concept_defs\":{concept_defs_count},\"entities\":{entities_count},\"required_axes\":{required_axes_count},\"body_kind_histogram\":[{}],\"status_histogram\":[{}]}}",
            body_pairs.join(","),
            status_pairs.join(","),
        );
    } else {
        println!("total entries: {total}");
        println!("concept_defs (DB1):  {concept_defs_count}");
        println!("entities (DB2):      {entities_count}");
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
            eprintln!("nom: unknown status: {s}. Known: complete, partial, opaque");
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

    // Try Dict first (new split-DB path); fall back to NomDict for backward compatibility.
    let entries = if let Ok(d) = Dict::try_open_from_nomdict_path(dict) {
        match find_entries(&d, &filter) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("nom: dict error: {e}");
                return 1;
            }
        }
    } else {
        let dict_db = match open_dict(dict) {
            Some(d) => d,
            None => return 1,
        };
        match dict_db.find_entries(&filter) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("nom: dict error: {e}");
                return 1;
            }
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
