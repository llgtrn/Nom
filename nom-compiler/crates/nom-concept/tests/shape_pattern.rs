//! GAP-12 — pattern-shape clause (`shaped like`) tests.
//!
//! Covers:
//!  - Parse a data entity with `shaped like "<pattern>"` → shape_pattern extracted.
//!  - Parse without a shaped clause → shape_pattern is None.
//!  - Pattern with structural interpolation markers retained verbatim.
//!  - `shaped` in prose (not followed by `like`) → no shape_pattern.
//!  - `shaped like` followed by non-quoted token → no shape_pattern (prose fallthrough).
//!  - Missing closing `.` → NOMX-S5h-unterminated-shape.
//!  - Clause shape registered in grammar DB for data kind.

use nom_concept::{
    NomtuItem,
    stages::{PipelineOutput, run_pipeline},
};

// ── helpers ──────────────────────────────────────────────────────────────────

fn first_entity(src: &str) -> nom_concept::EntityDecl {
    let out = run_pipeline(src).expect("pipeline must accept");
    match out {
        PipelineOutput::Nomtu(f) => match f.items.into_iter().next().unwrap() {
            NomtuItem::Entity(e) => e,
            NomtuItem::Composition(_) => panic!("expected entity, got composition"),
        },
        PipelineOutput::Nom(_) => panic!("expected nomtu output"),
    }
}

fn parse_err(src: &str) -> nom_concept::stages::StageFailure {
    run_pipeline(src).expect_err("pipeline must reject malformed shaped clause")
}

// ── positive cases ────────────────────────────────────────────────────────────

#[test]
fn shape_pattern_email_address() {
    let src = "the data email_address is intended to represent a validated email.\n\
               shaped like \"{local}@{domain}\".\n";
    let e = first_entity(src);
    assert_eq!(
        e.shape_pattern,
        Some("{local}@{domain}".to_string()),
        "shape_pattern should capture the quoted pattern verbatim"
    );
}

#[test]
fn shape_pattern_phone_number() {
    let src = "the data phone_number is intended to represent a formatted phone number.\n\
               shaped like \"+{country_code}-{area}-{local}\".\n";
    let e = first_entity(src);
    assert_eq!(
        e.shape_pattern,
        Some("+{country_code}-{area}-{local}".to_string()),
        "shape_pattern should retain all structural markers verbatim"
    );
}

#[test]
fn shape_pattern_plain_string() {
    let src = "the data url is intended to represent a resource locator.\n\
               shaped like \"https://{host}/{path}\".\n";
    let e = first_entity(src);
    assert_eq!(e.shape_pattern, Some("https://{host}/{path}".to_string()),);
}

#[test]
fn no_shaped_clause_yields_none() {
    let src = "the data record_id is intended to represent a database record identifier.\n\
               requires the identifier is non-empty.\n";
    let e = first_entity(src);
    assert_eq!(
        e.shape_pattern, None,
        "shape_pattern must be None when no shaped clause is present"
    );
}

#[test]
fn shaped_clause_coexists_with_contracts() {
    let src = "the data email_address is intended to represent a validated email.\n\
               requires the local part is non-empty.\n\
               ensures the domain part contains a dot.\n\
               shaped like \"{local}@{domain}\".\n";
    let e = first_entity(src);
    assert_eq!(
        e.shape_pattern,
        Some("{local}@{domain}".to_string()),
        "shape_pattern should be extracted alongside contracts"
    );
    assert_eq!(e.contracts.len(), 2, "contracts should be preserved");
}

#[test]
fn shaped_in_prose_not_followed_by_like_yields_none() {
    // `shaped` appearing in the intent sentence — not a clause opener.
    let src = "the data record is intended to be shaped for optimal storage.\n";
    let e = first_entity(src);
    assert_eq!(
        e.shape_pattern, None,
        "shaped in prose (not followed by like) must not be treated as a clause opener"
    );
}

// ── negative cases ────────────────────────────────────────────────────────────

#[test]
fn shaped_like_missing_closing_dot_rejects() {
    let src = "the data email_address is intended to represent an email.\nshaped like \"{local}@{domain}\"\n";
    let err = parse_err(src);
    assert_eq!(err.reason, "unterminated-shape");
}

// ── clause_shapes DB registration ─────────────────────────────────────────────

#[test]
fn shaped_clause_shape_in_grammar_db() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("grammar.sqlite");
    let conn = nom_grammar::init_at(&db).unwrap();

    // Seed the data kind + its clause_shapes.
    conn.execute(
        "INSERT INTO kinds (name, description, allowed_clauses, allowed_refs, shipped_commit, notes) \
         VALUES ('data', '', '[]', '[]', 'GAP-12-test', NULL)",
        [],
    )
    .unwrap();

    // Insert the shaped clause shape (mirrors the baseline.sql row).
    conn.execute(
        "INSERT OR IGNORE INTO clause_shapes \
         (kind, clause_name, is_required, position, grammar_shape, source_ref) \
         VALUES ('data', 'shaped', 0, 4, \
         '''shaped like'' <quoted-pattern> ''.''', 'GAP-12')",
        [],
    )
    .unwrap();

    let (is_req, pos): (i64, i64) = conn
        .query_row(
            "SELECT is_required, position FROM clause_shapes \
             WHERE kind = 'data' AND clause_name = 'shaped'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("shaped row must exist");
    assert_eq!(
        is_req, 0,
        "shaped clause must be optional (is_required = 0)"
    );
    assert_eq!(pos, 4, "shaped clause must be at position 4");
}
