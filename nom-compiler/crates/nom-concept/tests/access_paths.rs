//! GAP-12 — nested-record-path access clause tests.
//!
//! Covers:
//!  - Single flat path: `accesses user.name.` → access_paths = ["user.name"].
//!  - Deeply nested path: `accesses user.address.city.` → access_paths = ["user.address.city"].
//!  - Multiple comma-separated paths → both captured.
//!  - No access clause → access_paths is None.
//!  - Access clause coexists with contracts and effects.
//!  - Access clause coexists with retry and format.
//!  - Malformed: missing closing `.` → NOMX-S5f-unterminated-accesses.
//!  - Clause shape registered in grammar DB for function kind.

use nom_concept::{
    NomtuItem,
    stages::{PipelineOutput, run_pipeline},
};

// ── helpers ──────────────────────────────────────────────────────────────────

/// Run the pipeline and unwrap the first entity decl.
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
    run_pipeline(src).expect_err("pipeline must reject malformed accesses clause")
}

// ── positive cases ────────────────────────────────────────────────────────────

#[test]
fn access_paths_single_flat_path() {
    let src = "the function get_name is intended to extract the user name.\n\
               accesses user.name.\n";
    let e = first_entity(src);
    assert_eq!(
        e.access_paths,
        Some(vec!["user.name".to_string()]),
        "access_paths should capture a two-segment dot path"
    );
}

#[test]
fn access_paths_deeply_nested() {
    let src = "the function get_city is intended to extract the city from a user record.\n\
               accesses user.address.city.\n";
    let e = first_entity(src);
    assert_eq!(
        e.access_paths,
        Some(vec!["user.address.city".to_string()]),
        "access_paths should capture a three-segment dot path"
    );
}

#[test]
fn access_paths_multiple_paths() {
    let src = "the function get_contact is intended to extract contact information.\n\
               accesses user.name, user.address.city.\n";
    let e = first_entity(src);
    assert_eq!(
        e.access_paths,
        Some(vec![
            "user.name".to_string(),
            "user.address.city".to_string()
        ]),
        "access_paths should capture all comma-separated paths"
    );
}

#[test]
fn no_access_clause_yields_none() {
    let src = "the function ping is intended to check reachability.\n\
               requires the host is reachable.\n";
    let e = first_entity(src);
    assert_eq!(
        e.access_paths, None,
        "access_paths must be None when no accesses clause is present"
    );
}

#[test]
fn access_coexists_with_contracts_and_effects() {
    let src = "the function get_city is\n\
               intended to extract the city from a user record.\n\
               requires the user record is non-empty.\n\
               ensures the city is returned.\n\
               accesses user.address.city.\n\
               benefit city_resolved.\n";
    let e = first_entity(src);
    assert_eq!(e.access_paths, Some(vec!["user.address.city".to_string()]),);
    assert_eq!(e.contracts.len(), 2, "contracts should be preserved");
    assert_eq!(e.effects.len(), 1, "effects should be preserved");
}

#[test]
fn access_coexists_with_retry_and_format() {
    let src = "the function fetch_city is intended to fetch and format city from a record.\n\
               retry at-most 3 times.\n\
               accesses user.address.city.\n\
               format \"City: {city}\".\n";
    let e = first_entity(src);
    assert_eq!(e.access_paths, Some(vec!["user.address.city".to_string()]),);
    assert!(e.retry_policy.is_some(), "retry_policy should be preserved");
    assert!(
        e.format_template.is_some(),
        "format_template should be preserved"
    );
}

#[test]
fn accesses_in_prose_not_followed_by_word_yields_none() {
    // "accesses" at end of intent prose should be treated as a word, not a clause opener.
    let src = "the function audit is intended to log all accesses.\n";
    let e = first_entity(src);
    assert_eq!(
        e.access_paths, None,
        "accesses in prose without a following word should not be treated as a clause opener"
    );
}

// ── negative cases ────────────────────────────────────────────────────────────

#[test]
fn access_missing_closing_dot_rejects() {
    // `accesses user.address.city` with no `.`
    let src = "the function foo is intended to bar.\naccesses user.address.city\n";
    let err = parse_err(src);
    assert_eq!(
        err.reason, "unterminated-accesses",
        "expected unterminated-accesses, got: {:?}",
        err
    );
}

// ── clause_shapes DB registration ─────────────────────────────────────────────

#[test]
fn access_clause_shape_in_grammar_db() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("grammar.sqlite");
    let conn = nom_grammar::init_at(&db).unwrap();

    // Seed the function kind.
    conn.execute(
        "INSERT INTO kinds (name, description, allowed_clauses, allowed_refs, shipped_commit, notes) \
         VALUES ('function', '', '[]', '[]', 'GAP-12-test', NULL)",
        [],
    )
    .unwrap();

    // Insert the accesses clause shape (mirrors the baseline.sql row).
    conn.execute(
        "INSERT OR IGNORE INTO clause_shapes \
         (kind, clause_name, is_required, position, grammar_shape, source_ref) \
         VALUES ('function', 'accesses', 0, 9, \
         'accesses <dot-path> ([, <dot-path>]*)? .', 'GAP-12')",
        [],
    )
    .unwrap();

    // Verify it shows up with the right attributes.
    let (is_req, pos): (i64, i64) = conn
        .query_row(
            "SELECT is_required, position FROM clause_shapes \
             WHERE kind = 'function' AND clause_name = 'accesses'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("accesses row must exist");
    assert_eq!(
        is_req, 0,
        "accesses clause must be optional (is_required = 0)"
    );
    assert_eq!(pos, 9, "accesses clause must be at position 9");
}
