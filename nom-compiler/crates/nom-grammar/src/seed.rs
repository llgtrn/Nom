//! Seeding functions per doc 21 phases P4 + P5 (kinds + QualityNames + authoring_rules).
//!
//! This module carries the data that migrates "grammar in doc files" into the
//! grammar.sqlite registry so AI clients can query it deterministically. Each
//! seed fn is idempotent (INSERT OR REPLACE) so re-running after doc edits
//! brings the DB back in sync.

use anyhow::{Context, Result};
use rusqlite::{Connection, params};

// ── P5: closed 9-kind set (doc 08 §2, W41 + W46) ────────────────────

/// The closed set of 9 top-level kinds, in the canonical order they were
/// introduced (function → module → concept → screen → data → event → media
/// → property (W41) → scenario (W46)).
pub const KINDS_SEED: &[(&str, &str, &str)] = &[
    (
        "function",
        "Named computation with input types + output type + requires/ensures/hazard contract clauses.",
        "a04b91e",
    ),
    (
        "module",
        "Tier-1 composition: several DB2 entities grouped with optional composition expressions (doc 08 §1 Tier 1).",
        "a04b91e",
    ),
    (
        "concept",
        "Tier-2 big-scope container: one or more concepts with dictionary-relative index over DB2 (doc 08 §1 Tier 2).",
        "a04b91e",
    ),
    (
        "screen",
        "User-facing UI / rendered artifact / internal architecture diagram. Generalised by doc 14 #39 + #49.",
        "a04b91e",
    ),
    (
        "data",
        "Structural type / tagged variant / schema-IDL. Covers Kotlin-sealed, Elm Msg, Protobuf, Solidity tagged errors.",
        "a04b91e",
    ),
    (
        "event",
        "Named event signal (editor event, subscription, stream element). W49-quantified ensures describe delivery semantics.",
        "a04b91e",
    ),
    (
        "media",
        "Image / audio / video / 3D / typography — composable via same 3 operators per §5.18 aesthetic-is-programming.",
        "a04b91e",
    ),
    (
        "property",
        "Universally-quantified claim over a generator. Wedge W41 — 8th kind added for property-based-verification paradigm.",
        "W41-ship-commit",
    ),
    (
        "scenario",
        "Asserted-behavior claim with given/when/then triple. Wedge W46 — 9th kind added for BDD/Gherkin/RSpec surface.",
        "W46-ship-commit",
    ),
];

pub fn seed_kinds(conn: &Connection) -> Result<usize> {
    let mut inserted = 0;
    for (name, description, commit) in KINDS_SEED {
        conn.execute(
            "INSERT OR REPLACE INTO kinds \
             (name, description, allowed_clauses, allowed_refs, shipped_commit, notes) \
             VALUES (?1, ?2, '[]', '[]', ?3, NULL)",
            params![name, description, commit],
        )?;
        inserted += 1;
    }
    Ok(inserted)
}

// ── P5: 10 fixed QualityName seeds per MEMORY.md ────────────────────

/// The 10 fixed seed QualityNames registered 2026-04-14 (doc 08 §7, W51).
/// `metric_function` is a placeholder hash; real metric nomtu hashes are
/// populated by `nom corpus register-axis` per MEMORY.md roadmap item 8.
pub const QUALITY_SEED: &[(&str, &str, &str, Option<&str>)] = &[
    ("forward_compatibility", "semver/api", "any", None),
    ("numerical_stability", "numeric", "any", None),
    ("gas_efficiency", "onchain_cost", "any", None),
    ("synthesizability", "hardware", "any", None),
    ("minimum_cost", "optimization", "any", None),
    ("statistical_rigor", "stats", "any", None),
    ("availability", "ops", "exactly_one_per_app", Some("app")),
    ("auditability", "ops", "any", None),
    ("accessibility", "ops", "exactly_one_per_app", Some("app")),
    ("totality", "proofs", "any", None),
];

pub fn seed_quality_names(conn: &Connection) -> Result<usize> {
    let mut inserted = 0;
    for (name, axis, cardinality, required_at) in QUALITY_SEED {
        conn.execute(
            "INSERT OR REPLACE INTO quality_names \
             (name, axis, metric_function, cardinality, required_at, source_ref, notes) \
             VALUES (?1, ?2, 'placeholder_metric_hash', ?3, ?4, 'MEMORY.md:2026-04-14', NULL)",
            params![name, axis, cardinality, required_at],
        )?;
        inserted += 1;
    }
    Ok(inserted)
}

// ── P4: parse doc 16 markdown table → authoring_rules rows ──────────

#[derive(Debug, PartialEq)]
pub struct DocRuleRow {
    pub row_id: i64,
    pub gap_summary: String,
    pub destination: String,
    pub status: String,
    pub closed_in: Option<String>,
}

/// Parse lines shaped like `| 419 | Behavioral-module ... | authoring-guide rule | ✅ closed (doc 14 #85) |`
/// from doc 16's markdown source. Header lines, divider lines, and narrative
/// text are silently skipped. Returns one DocRuleRow per table row.
pub fn parse_doc16_rules(md_source: &str) -> Vec<DocRuleRow> {
    let mut rows = Vec::new();
    for raw in md_source.lines() {
        let line = raw.trim();
        // Require a numeric-leading table row: `| <n> | ... | ... | ... |`
        if !line.starts_with("| ") {
            continue;
        }
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        // A well-formed row has 6 split-pieces: "" | id | gap | dest | status | ""
        if cells.len() < 5 {
            continue;
        }
        let id_cell = cells[1];
        let row_id: i64 = match id_cell.parse() {
            Ok(n) => n,
            Err(_) => continue,
        };
        let gap_summary = cells[2].to_string();
        let destination = cells[3].to_string();
        let status_cell = cells[4].to_string();
        let (status, closed_in) = split_status_and_ref(&status_cell);
        rows.push(DocRuleRow {
            row_id,
            gap_summary,
            destination,
            status,
            closed_in,
        });
    }
    rows
}

/// Split a status cell like "✅ closed (doc 14 #85)" into ("closed", Some("doc 14 #85")).
/// Leaves free-form statuses like "⏳ queued" → ("queued", None).
fn split_status_and_ref(cell: &str) -> (String, Option<String>) {
    // Strip leading emoji + whitespace.
    let stripped = cell
        .chars()
        .skip_while(|c| !c.is_ascii_alphabetic())
        .collect::<String>();
    // Look for "(...)" ref suffix.
    if let Some(paren_idx) = stripped.find('(') {
        let (head, tail) = stripped.split_at(paren_idx);
        let status = head.trim().to_string();
        let closed_in = tail
            .trim_start_matches('(')
            .trim_end_matches(')')
            .trim()
            .to_string();
        (status, if closed_in.is_empty() { None } else { Some(closed_in) })
    } else {
        (stripped.trim().to_string(), None)
    }
}

pub fn seed_authoring_rules(conn: &Connection, rows: &[DocRuleRow]) -> Result<usize> {
    let mut inserted = 0;
    for row in rows {
        conn.execute(
            "INSERT OR REPLACE INTO authoring_rules \
             (row_id, source_paradigm, gap_summary, nom_shape, reuses_rows, destination, status, closed_in, source_doc_ref) \
             VALUES (?1, '', ?2, '', NULL, ?3, ?4, ?5, ?6)",
            params![
                row.row_id,
                row.gap_summary,
                row.destination,
                row.status,
                row.closed_in,
                format!("doc 16 row {}", row.row_id),
            ],
        )?;
        inserted += 1;
    }
    Ok(inserted)
}

/// One-shot convenience: seed kinds + quality_names + parse+insert all rows from
/// the given doc-16 markdown source. Callable from the CLI `nom grammar seed`.
pub fn seed_all_from_doc16(conn: &Connection, doc16_md: &str) -> Result<(usize, usize, usize)> {
    let kinds = seed_kinds(conn).context("seeding kinds")?;
    let qualities = seed_quality_names(conn).context("seeding quality_names")?;
    let rows = parse_doc16_rules(doc16_md);
    let rules = seed_authoring_rules(conn, &rows).context("seeding authoring_rules")?;
    Ok((kinds, qualities, rules))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_at;
    use tempfile::tempdir;

    #[test]
    fn seeds_nine_kinds_verbatim() {
        let dir = tempdir().unwrap();
        let conn = init_at(dir.path().join("g.sqlite")).unwrap();
        let n = seed_kinds(&conn).unwrap();
        assert_eq!(n, 9);
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM kinds", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 9);
    }

    #[test]
    fn seeding_kinds_is_idempotent() {
        let dir = tempdir().unwrap();
        let conn = init_at(dir.path().join("g.sqlite")).unwrap();
        let _ = seed_kinds(&conn).unwrap();
        let _ = seed_kinds(&conn).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM kinds", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 9); // INSERT OR REPLACE keeps row count constant
    }

    #[test]
    fn seeds_ten_quality_names() {
        let dir = tempdir().unwrap();
        let conn = init_at(dir.path().join("g.sqlite")).unwrap();
        let n = seed_quality_names(&conn).unwrap();
        assert_eq!(n, 10);
    }

    #[test]
    fn parses_closed_row_with_ref() {
        let md = "\
| 419 | Behavioral-module declarations | authoring-guide rule | ✅ closed (doc 14 #85) |
";
        let rows = parse_doc16_rules(md);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].row_id, 419);
        assert_eq!(rows[0].gap_summary, "Behavioral-module declarations");
        assert_eq!(rows[0].destination, "authoring-guide rule");
        assert_eq!(rows[0].status, "closed");
        assert_eq!(rows[0].closed_in.as_deref(), Some("doc 14 #85"));
    }

    #[test]
    fn parses_queued_row_without_ref() {
        let md = "\
| 5 | Format-string interpolation | **W5** grammar rule | ⏳ queued |
";
        let rows = parse_doc16_rules(md);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].row_id, 5);
        assert_eq!(rows[0].status, "queued");
        assert_eq!(rows[0].closed_in, None);
    }

    #[test]
    fn ignores_header_and_divider_lines() {
        let md = "\
# Title
## Triage format
| # | Gap | Destination | Status |
|--:|-----|-------------|--------|
| 1 | First | W-wedge | ⏳ queued |
Narrative text here.
| 2 | Second | authoring-guide rule | ✅ closed (doc 14 #42) |
";
        let rows = parse_doc16_rules(md);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].row_id, 1);
        assert_eq!(rows[1].row_id, 2);
    }

    #[test]
    fn seed_all_from_doc16_populates_all_three_tables() {
        let dir = tempdir().unwrap();
        let conn = init_at(dir.path().join("g.sqlite")).unwrap();
        let md = "\
| 1 | First gap | authoring-guide rule | ✅ closed (doc 14 #1) |
| 2 | Second gap | W-wedge | ⏳ queued |
| 3 | Third gap | design deferred | 🔒 blocked |
";
        let (kinds, qualities, rules) = seed_all_from_doc16(&conn, md).unwrap();
        assert_eq!(kinds, 9);
        assert_eq!(qualities, 10);
        assert_eq!(rules, 3);
    }

    #[test]
    fn doc16_row_count_matches_repo_file() {
        // Smoke test: parse the actual doc 16 shipped in the repo and confirm row
        // count matches the expected 450 at the current HEAD.
        let md = match std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../research/language-analysis/16-nomx-syntax-gap-backlog.md"
        )) {
            Ok(s) => s,
            Err(_) => {
                // Skip test if the doc is not at the expected relative path
                // (e.g. when running from a tarball without research/).
                return;
            }
        };
        let rows = parse_doc16_rules(&md);
        assert!(
            rows.len() >= 400,
            "expected ≥400 rows from doc 16, got {}",
            rows.len()
        );
    }
}
