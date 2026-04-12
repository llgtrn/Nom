//! Handlers for `nom draft` subcommands — creating and managing named
//! collections of nomtu entries grouped by domain.

use nom_dict::{Draft, EntryFilter, NomDict};
use nom_types::{EntryKind, EntryStatus};
use serde_json::json;
use std::path::Path;

pub fn cmd_draft_new(name: &str, describe: Option<&str>, dict: &Path) -> i32 {
    let d = match NomDict::open_in_place(dict) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("nom draft new: cannot open dict: {e}");
            return 1;
        }
    };
    let draft = Draft {
        id: Draft::id_for(name),
        name: name.trim().to_string(),
        describe: describe.map(str::to_string),
        created_at: chrono_now(),
        updated_at: None,
    };
    if let Err(e) = d.upsert_draft(&draft) {
        eprintln!("nom draft new: {e}");
        return 1;
    }
    println!("draft '{}' created (id {})", draft.name, &draft.id[..16]);
    0
}

pub fn cmd_draft_add(draft_name: &str, entry_spec: &str, dict: &Path) -> i32 {
    let d = match NomDict::open_in_place(dict) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("nom draft add: cannot open dict: {e}");
            return 1;
        }
    };
    let draft = match d.get_draft_by_name(draft_name) {
        Ok(Some(dr)) => dr,
        Ok(None) => {
            eprintln!("nom draft add: draft '{draft_name}' not found");
            return 1;
        }
        Err(e) => {
            eprintln!("nom draft add: {e}");
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
                eprintln!("nom draft add: {e}");
                return 1;
            }
        }
    };
    match d.add_draft_member(&draft.id, &entry_id) {
        Ok(true) => {
            println!("added {} to draft '{}'", &entry_id[..16], draft_name);
            0
        }
        Ok(false) => {
            println!("entry already in draft '{}' — no change", draft_name);
            0
        }
        Err(e) => {
            eprintln!("nom draft add: {e}");
            1
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn cmd_draft_add_by(
    draft_name: &str,
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
            eprintln!("nom draft add-by: cannot open dict: {e}");
            return 1;
        }
    };
    let draft = match d.get_draft_by_name(draft_name) {
        Ok(Some(dr)) => dr,
        Ok(None) => {
            eprintln!("nom draft add-by: draft '{draft_name}' not found");
            return 1;
        }
        Err(e) => {
            eprintln!("nom draft add-by: {e}");
            return 1;
        }
    };

    // describe_like is exclusive with structural filters per spec.
    if describe_like.is_some()
        && (language.is_some() || kind.is_some() || body_kind.is_some() || status.is_some())
    {
        eprintln!(
            "nom draft add-by: --describe-like is exclusive with structural filters \
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
                eprintln!("nom draft add-by: search failed: {e}");
                return 1;
            }
        };
        let mut count = 0usize;
        for e in &entries {
            match d.add_draft_member(&draft.id, &e.id) {
                Ok(true) => count += 1,
                Ok(false) => {}
                Err(e) => {
                    eprintln!("nom draft add-by: member insert failed: {e}");
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
        match d.add_draft_members_by_filter(&draft.id, &filter) {
            Ok(n) => n,
            Err(e) => {
                eprintln!("nom draft add-by: {e}");
                return 1;
            }
        }
    };

    println!("added {added} entries to draft '{draft_name}'");
    0
}

pub fn cmd_draft_list(json: bool, dict: &Path) -> i32 {
    let d = match NomDict::open_in_place(dict) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("nom draft list: cannot open dict: {e}");
            return 1;
        }
    };
    let drafts = match d.list_drafts() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("nom draft list: {e}");
            return 1;
        }
    };

    if json {
        let mut rows: Vec<serde_json::Value> = Vec::with_capacity(drafts.len());
        for dr in &drafts {
            let count = d.count_draft_members(&dr.id).unwrap_or(0);
            rows.push(json!({
                "id": dr.id,
                "name": dr.name,
                "describe": dr.describe,
                "member_count": count,
                "created_at": dr.created_at,
            }));
        }
        println!("{}", serde_json::to_string_pretty(&rows).unwrap_or_default());
    } else {
        if drafts.is_empty() {
            println!("no drafts found");
            return 0;
        }
        println!("{:<30} {:>8}  {}", "name", "members", "describe");
        println!("{}", "-".repeat(72));
        for dr in &drafts {
            let count = d.count_draft_members(&dr.id).unwrap_or(0);
            println!(
                "{:<30} {:>8}  {}",
                dr.name,
                count,
                dr.describe.as_deref().unwrap_or("")
            );
        }
    }
    0
}

pub fn cmd_draft_show(name: &str, limit: usize, json: bool, dict: &Path) -> i32 {
    let d = match NomDict::open_in_place(dict) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("nom draft show: cannot open dict: {e}");
            return 1;
        }
    };
    let draft = match d.get_draft_by_name(name) {
        Ok(Some(dr)) => dr,
        Ok(None) => {
            eprintln!("nom draft show: draft '{name}' not found");
            return 1;
        }
        Err(e) => {
            eprintln!("nom draft show: {e}");
            return 1;
        }
    };
    let mut members = match d.get_draft_members(&draft.id) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("nom draft show: {e}");
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
            "draft": { "id": draft.id, "name": draft.name, "describe": draft.describe },
            "members": rows,
        });
        println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
    } else {
        println!("draft: {} ({})", draft.name, &draft.id[..16]);
        if let Some(desc) = &draft.describe {
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

pub fn cmd_draft_delete(name: &str, dict: &Path) -> i32 {
    let d = match NomDict::open_in_place(dict) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("nom draft delete: cannot open dict: {e}");
            return 1;
        }
    };
    match d.get_draft_by_name(name) {
        Ok(None) => {
            eprintln!("nom draft delete: draft '{name}' not found");
            return 1;
        }
        Err(e) => {
            eprintln!("nom draft delete: {e}");
            return 1;
        }
        Ok(Some(_)) => {}
    }
    if let Err(e) = d.delete_draft(name) {
        eprintln!("nom draft delete: {e}");
        return 1;
    }
    println!("draft '{name}' deleted");
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
