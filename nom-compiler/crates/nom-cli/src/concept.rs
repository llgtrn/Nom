//! Handlers for `nom concept` subcommands — creating and managing named
//! domain concepts that group nomtu entries. Each concept name is itself
//! a valid Nom syntax token, addressable via `use <concept>@<hash>` in
//! .nom source.

use nom_dict::{Concept, EntryFilter, NomDict};
use nom_types::{Contract, Entry, EntryKind, EntryStatus};
use serde_json::json;
use std::path::Path;

pub fn cmd_concept_new(name: &str, describe: Option<&str>, dict: &Path) -> i32 {
    let d = match NomDict::open_in_place(dict) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("nom concept new: cannot open dict: {e}");
            return 1;
        }
    };
    let concept = Concept {
        id: Concept::id_for(name),
        name: name.trim().to_string(),
        describe: describe.map(str::to_string),
        created_at: chrono_now(),
        updated_at: None,
    };
    if let Err(e) = d.upsert_concept(&concept) {
        eprintln!("nom concept new: {e}");
        return 1;
    }
    // Also upsert an Entry with kind=Concept so the concept is first-class
    // addressable — LLMs can `list_nomtu --kind concept` or use
    // `use <name>@<hash>` in .nom source.
    let entry = Entry {
        id: concept.id.clone(),
        word: concept.name.clone(),
        variant: None,
        kind: EntryKind::Concept,
        language: "nom".to_string(),
        describe: concept.describe.clone(),
        concept: None,
        body: None,
        body_nom: None,
        contract: Contract::default(),
        status: EntryStatus::Complete,
        translation_score: None,
        is_canonical: true,
        deprecated_by: None,
        created_at: concept.created_at.clone(),
        updated_at: None,
        body_kind: None,
        body_bytes: None,
    };
    if let Err(e) = d.upsert_entry(&entry) {
        eprintln!("nom concept new: entry upsert failed: {e}");
        return 1;
    }
    println!("concept '{}' created (id {})", concept.name, &concept.id[..16]);
    0
}

pub fn cmd_concept_add(concept_name: &str, entry_spec: &str, dict: &Path) -> i32 {
    let d = match NomDict::open_in_place(dict) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("nom concept add: cannot open dict: {e}");
            return 1;
        }
    };
    let concept = match d.get_concept_by_name(concept_name) {
        Ok(Some(c)) => c,
        Ok(None) => {
            eprintln!("nom concept add: concept '{concept_name}' not found");
            return 1;
        }
        Err(e) => {
            eprintln!("nom concept add: {e}");
            return 1;
        }
    };
    // Resolve entry_spec: 64-hex exact id, or ≥8-char prefix.
    let entry_id = if entry_spec.len() == 64 && entry_spec.chars().all(|c| c.is_ascii_hexdigit()) {
        entry_spec.to_string()
    } else {
        match d.resolve_prefix(entry_spec) {
            Ok(id) => id,
            Err(e) => {
                eprintln!("nom concept add: {e}");
                return 1;
            }
        }
    };
    match d.add_concept_member(&concept.id, &entry_id) {
        Ok(true) => {
            println!("added {} to concept '{}'", &entry_id[..16], concept_name);
            0
        }
        Ok(false) => {
            println!("entry already in concept '{}' — no change", concept_name);
            0
        }
        Err(e) => {
            eprintln!("nom concept add: {e}");
            1
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn cmd_concept_add_by(
    concept_name: &str,
    language: Option<&str>,
    kind: Option<&str>,
    body_kind: Option<&str>,
    status: Option<&str>,
    describe_like: Option<&str>,
    limit: usize,
    dict: &Path,
) -> i32 {
    let d = match NomDict::open_in_place(dict) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("nom concept add-by: cannot open dict: {e}");
            return 1;
        }
    };
    let concept = match d.get_concept_by_name(concept_name) {
        Ok(Some(c)) => c,
        Ok(None) => {
            eprintln!("nom concept add-by: concept '{concept_name}' not found");
            return 1;
        }
        Err(e) => {
            eprintln!("nom concept add-by: {e}");
            return 1;
        }
    };

    // describe_like is exclusive with structural filters per spec.
    if describe_like.is_some()
        && (language.is_some() || kind.is_some() || body_kind.is_some() || status.is_some())
    {
        eprintln!(
            "nom concept add-by: --describe-like is exclusive with structural filters \
             (--language, --kind, --body-kind, --status); ignoring --describe-like"
        );
        // Proceed with structural filters only.
    }

    let added = if describe_like.is_some()
        && language.is_none()
        && kind.is_none()
        && body_kind.is_none()
        && status.is_none()
    {
        // describe_like only path.
        let q = describe_like.unwrap();
        let entries = match d.search_describe(q, limit) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("nom concept add-by: search failed: {e}");
                return 1;
            }
        };
        let mut count = 0usize;
        for e in &entries {
            match d.add_concept_member(&concept.id, &e.id) {
                Ok(true) => count += 1,
                Ok(false) => {}
                Err(e) => {
                    eprintln!("nom concept add-by: member insert failed: {e}");
                    return 1;
                }
            }
        }
        count
    } else {
        let filter = EntryFilter {
            language: language.map(str::to_string),
            kind: kind.map(EntryKind::from_str),
            body_kind: body_kind.map(str::to_string),
            status: status.map(EntryStatus::from_str),
            limit,
        };
        match d.add_concept_members_by_filter(&concept.id, &filter) {
            Ok(n) => n,
            Err(e) => {
                eprintln!("nom concept add-by: {e}");
                return 1;
            }
        }
    };

    println!("added {added} entries to concept '{concept_name}'");
    0
}

pub fn cmd_concept_list(json: bool, dict: &Path) -> i32 {
    let d = match NomDict::open_in_place(dict) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("nom concept list: cannot open dict: {e}");
            return 1;
        }
    };
    let concepts = match d.list_concepts() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("nom concept list: {e}");
            return 1;
        }
    };

    if json {
        let mut rows: Vec<serde_json::Value> = Vec::with_capacity(concepts.len());
        for c in &concepts {
            let count = d.count_concept_members(&c.id).unwrap_or(0);
            rows.push(json!({
                "id": c.id,
                "name": c.name,
                "describe": c.describe,
                "member_count": count,
                "created_at": c.created_at,
            }));
        }
        println!("{}", serde_json::to_string_pretty(&rows).unwrap_or_default());
    } else {
        if concepts.is_empty() {
            println!("no concepts found");
            return 0;
        }
        println!("{:<30} {:>8}  {}", "name", "members", "describe");
        println!("{}", "-".repeat(72));
        for c in &concepts {
            let count = d.count_concept_members(&c.id).unwrap_or(0);
            println!(
                "{:<30} {:>8}  {}",
                c.name,
                count,
                c.describe.as_deref().unwrap_or("")
            );
        }
    }
    0
}

pub fn cmd_concept_show(name: &str, limit: usize, json: bool, dict: &Path) -> i32 {
    let d = match NomDict::open_in_place(dict) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("nom concept show: cannot open dict: {e}");
            return 1;
        }
    };
    let concept = match d.get_concept_by_name(name) {
        Ok(Some(c)) => c,
        Ok(None) => {
            eprintln!("nom concept show: concept '{name}' not found");
            return 1;
        }
        Err(e) => {
            eprintln!("nom concept show: {e}");
            return 1;
        }
    };
    let mut members = match d.get_concept_members(&concept.id) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("nom concept show: {e}");
            return 1;
        }
    };
    members.truncate(limit);

    if json {
        let rows: Vec<serde_json::Value> = members
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
        let out = json!({
            "concept": { "id": concept.id, "name": concept.name, "describe": concept.describe },
            "members": rows,
        });
        println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
    } else {
        println!("concept: {} ({})", concept.name, &concept.id[..16]);
        if let Some(desc) = &concept.describe {
            println!("  describe: {desc}");
        }
        println!("  showing {} member(s):", members.len());
        println!();
        for e in &members {
            println!(
                "  {} {:>8}  {}  {}",
                &e.id[..16],
                e.word,
                e.kind.as_str(),
                e.describe.as_deref().unwrap_or("")
            );
        }
    }
    0
}

pub fn cmd_concept_delete(name: &str, dict: &Path) -> i32 {
    let d = match NomDict::open_in_place(dict) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("nom concept delete: cannot open dict: {e}");
            return 1;
        }
    };
    match d.get_concept_by_name(name) {
        Ok(None) => {
            eprintln!("nom concept delete: concept '{name}' not found");
            return 1;
        }
        Err(e) => {
            eprintln!("nom concept delete: {e}");
            return 1;
        }
        Ok(Some(_)) => {}
    }
    if let Err(e) = d.delete_concept(name) {
        eprintln!("nom concept delete: {e}");
        return 1;
    }
    println!("concept '{name}' deleted");
    0
}

/// Minimal UTC-like timestamp for created_at (no chrono dep needed).
fn chrono_now() -> String {
    // Use SQLite's datetime('now') substitute via a fixed-format string.
    // In production the DB DEFAULT handles this; we pass a fallback.
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Convert epoch seconds to a naive UTC datetime string.
    // Days/months math is simple enough to avoid a dep.
    let s = secs;
    let days_since_epoch = s / 86400;
    let time_of_day = s % 86400;
    let h = time_of_day / 3600;
    let m = (time_of_day % 3600) / 60;
    let sec = time_of_day % 60;
    // Gregorian calendar from epoch (Jan 1 1970).
    let (y, mo, d) = days_to_ymd(days_since_epoch);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{sec:02}Z")
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    let mut year = 1970u64;
    loop {
        let leap = is_leap(year);
        let days_in_year = if leap { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }
    let leap = is_leap(year);
    let month_days: [u64; 12] = [
        31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31,
    ];
    let mut month = 1u64;
    for &md in &month_days {
        if days < md {
            break;
        }
        days -= md;
        month += 1;
    }
    (year, month, days + 1)
}

fn is_leap(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}
