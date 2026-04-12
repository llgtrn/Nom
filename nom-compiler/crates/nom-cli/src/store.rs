//! `nom store` subcommands — v2 content-addressed dictionary CLI.
//!
//! Wires nom-parser → nom-types::canonical → nom-dict → nom-resolver::v2
//! so a user can ingest a `.nom` file, retrieve an entry by hash prefix,
//! walk the closure from a root, verify reachability, and GC to roots.
//!
//! Tasks A/B landed the storage layer; this module is the single CLI
//! surface that consumes them. `body_nom` is stored as the human-readable
//! pre-rewrite source per the Task B hazard report; `entry_refs` is
//! populated from the resolver output (missing refs → diagnostics +
//! Partial status, not a blocking error).
//!
//! Paths are UTF-8 safe and handle both forward and back slashes so the
//! CLI works unchanged on Windows and POSIX.

use std::path::{Path, PathBuf};

use nom_ast::{Classifier, Declaration, SourceFile, Statement};
use nom_dict::{EntryFilter, NomDict};
use nom_parser::parse_source;
use nom_resolver::v2::{ResolutionTable, resolve_use_statements};
use nom_types::{
    Contract, Entry, EntryKind, EntryStatus, canonical::entry_id,
};
use sha2::{Digest, Sha256};

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

    let mut ids: Vec<String> = Vec::new();
    let mut refs: Vec<String> = Vec::new();

    for decl in &sf.declarations {
        let contract = contract_from_decl(decl);
        let id = entry_id(decl, &contract);
        let word = decl.name.name.clone();
        let kind = kind_from_classifier(decl.classifier);
        let describe = describe_from_decl(decl);

        // Re-upsert is idempotent; side tables get the latest merge-union.
        let already_present = dict_db
            .get_entry(&id)
            .ok()
            .flatten()
            .is_some();

        let body_nom = if already_present {
            // Body is immutable once stored — preserve existing bytes.
            None
        } else {
            Some(source.clone())
        };

        let entry = Entry {
            id: id.clone(),
            word,
            variant: None,
            kind,
            language: "nom".to_string(),
            describe,
            concept: None,
            body: None,
            body_nom,
            body_bytes: None,
            body_kind: Some(nom_types::body_kind::NOM_SOURCE.to_owned()),
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
    let hist = match dict_db.body_kind_histogram() {
        Ok(h) => h,
        Err(e) => {
            eprintln!("nom: dict error: {e}");
            return 1;
        }
    };
    if json {
        let pairs: Vec<String> = hist
            .iter()
            .map(|(k, n)| format!("{{\"body_kind\":\"{}\",\"count\":{n}}}", escape_json(k)))
            .collect();
        println!(
            "{{\"total\":{total},\"body_kind_histogram\":[{}]}}",
            pairs.join(",")
        );
    } else {
        println!("total entries: {total}");
        println!("body_kind histogram:");
        if hist.is_empty() {
            println!("  (empty)");
        } else {
            for (kind, count) in &hist {
                let pct = if total == 0 {
                    0.0
                } else {
                    100.0 * (*count as f64) / (total as f64)
                };
                println!("  {kind:<14} {count:>8}  ({pct:.1}%)");
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

/// `nom store add-media <file> [--dict <path>] [--json]`
///
/// Ingest a media file, persist its canonical bytes' SHA-256 hash as a
/// v2 `entries` row tagged with the matching §4.4.6 `body_kind`, and
/// print the resulting id.
///
/// The canonical bytes themselves are NOT stored in `Entry.body` (which
/// is `Option<String>`, a legacy text column). A future schema migration
/// will add a BLOB column; for now `body = None` and the hash (= `id`)
/// is the persistent record.
pub fn cmd_store_add_media(path: &Path, dict: &Path, json: bool) -> i32 {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("nom: cannot read {}: {e}", path.display());
            return 1;
        }
    };

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    // Dispatch to the shared ingest helper (same table as `nom media import`).
    let summary = match crate::media::ingest_by_extension(&bytes, &ext) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("nom: {e}");
            return 1;
        }
    };

    // SHA-256 the canonical bytes → hex id (same shape as Phase-4 hashes).
    let id = {
        let mut hasher = Sha256::new();
        hasher.update(&summary.canonical_bytes);
        format!("{:x}", hasher.finalize())
    };

    // Derive word = file stem, stripped to [a-z0-9].
    let word = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("media")
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>();
    let word = if word.is_empty() { "media".to_string() } else { word };

    // variant = the extension (lets multiple encodings of same word coexist).
    let variant = Some(ext.clone());

    let canonical_bytes_len = summary.canonical_bytes.len();

    let entry = Entry {
        id: id.clone(),
        word: word.clone(),
        variant,
        kind: EntryKind::MediaUnit,
        language: "media".to_string(),
        describe: Some(summary.describe.clone()),
        concept: None,
        body: None,
        body_nom: None,
        body_bytes: Some(summary.canonical_bytes),
        body_kind: Some(summary.body_kind_tag.to_owned()),
        contract: Contract::default(),
        status: EntryStatus::Complete,
        translation_score: None,
        is_canonical: true,
        deprecated_by: None,
        created_at: chrono_like_now(),
        updated_at: None,
    };

    let dict_db = match open_dict(dict) {
        Some(d) => d,
        None => return 1,
    };

    if let Err(e) = dict_db.upsert_entry(&entry) {
        eprintln!("nom: upsert error for {id}: {e}");
        return 1;
    }

    if json {
        println!(
            "{{\"id\":\"{id}\",\"body_kind\":\"{}\",\"canonical_bytes\":{canonical_bytes_len},\"word\":\"{word}\",\"variant\":\"{ext}\"}}",
            summary.body_kind_tag
        );
    } else {
        println!("id:              {id}");
        println!("body_kind:       {}", summary.body_kind_tag);
        println!("canonical_bytes: {canonical_bytes_len}");
        println!("word:            {word}");
        println!("variant:         {ext}");
        println!("describe:        {}", summary.describe);
    }
    0
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_owned()
    } else {
        format!("{}…", &s[..n.saturating_sub(1)])
    }
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

// ── Helpers ───────────────────────────────────────────────────────────

fn open_dict(dict: &Path) -> Option<NomDict> {
    let root = if dict.extension().is_some_and(|e| e == "db") {
        // dict points to a .db file; NomDict::open expects the directory
        // that contains `data/nomdict.db`. Pick an ancestor that already
        // has a `data/` child, falling back to cwd for compatibility
        // with the legacy `--dict nomdict.db` convention.
        let parent = dict.parent().unwrap_or_else(|| Path::new("."));
        if parent.file_name().and_then(|n| n.to_str()) == Some("data") {
            parent.parent().unwrap_or(Path::new(".")).to_path_buf()
        } else {
            parent.to_path_buf()
        }
    } else {
        dict.to_path_buf()
    };
    match NomDict::open(&root) {
        Ok(d) => Some(d),
        Err(e) => {
            eprintln!("nom: cannot open nomdict at {}: {e}", root.display());
            None
        }
    }
}

/// Resolve a hash prefix against the dict. Returns the full 64-char id
/// on a unique match; an error message otherwise.
fn resolve_prefix(dict: &NomDict, hash: &str) -> Result<String, String> {
    if hash.len() < 8 {
        return Err(format!(
            "nom: hash prefix too short (need ≥ 8 hex chars): {hash}"
        ));
    }
    if !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("nom: not a hex string: {hash}"));
    }
    // Full id? get_entry fast path.
    if hash.len() == 64 {
        return match dict.get_entry(hash) {
            Ok(Some(e)) => Ok(e.id),
            Ok(None) => Err(format!("nom: no entry with id {hash}")),
            Err(e) => Err(format!("nom: dict error: {e}")),
        };
    }
    let pattern = format!("{hash}%");
    let mut stmt = dict
        .connection()
        .prepare_cached("SELECT id FROM entries WHERE id LIKE ?1 ORDER BY id")
        .map_err(|e| format!("nom: dict error: {e}"))?;
    let ids: Vec<String> = stmt
        .query_map([pattern], |row| row.get::<_, String>(0))
        .map_err(|e| format!("nom: dict error: {e}"))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| format!("nom: dict error: {e}"))?;
    match ids.len() {
        0 => Err(format!("nom: no entry matching prefix {hash}")),
        1 => Ok(ids.into_iter().next().unwrap()),
        _ => {
            let mut msg = format!("nom: hash prefix {hash} is ambiguous ({} candidates):", ids.len());
            for id in &ids {
                msg.push_str(&format!("\n  {id}"));
            }
            Err(msg)
        }
    }
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

fn kind_from_classifier(c: Classifier) -> EntryKind {
    match c {
        Classifier::Nom => EntryKind::Function,
        Classifier::Flow => EntryKind::Function,
        Classifier::Test => EntryKind::TestCase,
        Classifier::Store => EntryKind::Schema,
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

fn chrono_like_now() -> String {
    // Keep dependencies minimal: use a coarse UTC timestamp so newly
    // upserted rows don't leave `created_at` empty. The dict's own
    // `datetime('now')` DEFAULT handles downstream re-upserts.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("epoch-{secs}")
}

fn load_roots() -> std::io::Result<Vec<String>> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from);
    let path = match home {
        Some(h) => h.join(".nom").join("roots.txt"),
        None => return Ok(Vec::new()),
    };
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(&path)?;
    Ok(content
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect())
}

fn json_array(items: &[String]) -> String {
    let escaped: Vec<String> = items
        .iter()
        .map(|s| format!("\"{}\"", escape_json(s)))
        .collect();
    format!("[{}]", escaped.join(","))
}

fn escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}
