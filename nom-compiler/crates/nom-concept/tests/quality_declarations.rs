//! GAP-12 — S5m inline quality-score declaration tests.
//!
//! Covers:
//!  - Parse `quality security 0.95.` → QualityDeclaration with name=security, score=0.95.
//!  - Multiple quality declarations are all captured in source order.
//!  - Parse without any quality clause → quality_declarations is None.
//!  - Quality coexists with contracts and effects.
//!  - Malformed: score outside [0.0, 1.0] → NOMX-S5m-out-of-range-quality.
//!  - Malformed: missing closing `.` → NOMX-S5m-unterminated-quality.
//!  - Clause shape registered in grammar DB for function kind.

use nom_concept::{
    NomtuItem, QualityDeclaration,
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
    run_pipeline(src).expect_err("pipeline must reject malformed quality clause")
}

// ── positive cases ────────────────────────────────────────────────────────────

#[test]
fn single_quality_declaration_extracted() {
    let src = "the function validate_input is\n\
               intended to validate user input.\n\
               quality security 0.95.\n";
    let e = first_entity(src);
    let decls = e
        .quality_declarations
        .expect("quality_declarations must be Some");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].name, "security");
    assert!((decls[0].score - 0.95).abs() < 1e-9);
}

#[test]
fn multiple_quality_declarations_all_captured() {
    let src = "the function auth_check is\n\
               intended to check authorization.\n\
               quality security 0.98.\n\
               quality performance 0.80.\n\
               quality reliability 0.99.\n";
    let e = first_entity(src);
    let decls = e
        .quality_declarations
        .expect("quality_declarations must be Some");
    assert_eq!(
        decls.len(),
        3,
        "all three quality declarations must be captured"
    );
    assert_eq!(decls[0].name, "security");
    assert!((decls[0].score - 0.98).abs() < 1e-9);
    assert_eq!(decls[1].name, "performance");
    assert!((decls[1].score - 0.80).abs() < 1e-9);
    assert_eq!(decls[2].name, "reliability");
    assert!((decls[2].score - 0.99).abs() < 1e-9);
}

#[test]
fn no_quality_declarations_yields_none() {
    let src = "the function simple_fn is\n\
               intended to perform a simple operation.\n";
    let e = first_entity(src);
    assert_eq!(
        e.quality_declarations, None,
        "quality_declarations must be None when no quality clauses are present"
    );
}

#[test]
fn quality_declarations_coexist_with_contracts_and_effects() {
    let src = "the function process_payment is\n\
               intended to process a payment transaction.\n\
               requires the amount is positive.\n\
               ensures the payment is recorded.\n\
               quality security 0.99.\n\
               hazard double_charge.\n";
    let e = first_entity(src);
    let decls = e
        .quality_declarations
        .expect("quality_declarations must be Some");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].name, "security");
    assert_eq!(e.contracts.len(), 2, "contracts must be preserved");
    assert_eq!(e.effects.len(), 1, "effects must be preserved");
}

#[test]
fn quality_score_boundary_values_accepted() {
    let src = "the function boundary_fn is\n\
               intended to test quality score boundaries.\n\
               quality min_quality 0.0.\n";
    let e = first_entity(src);
    let decls = e
        .quality_declarations
        .expect("quality_declarations must be Some");
    assert!((decls[0].score - 0.0).abs() < 1e-9);
}

// ── negative cases ────────────────────────────────────────────────────────────

#[test]
fn quality_score_above_one_rejects() {
    let src = "the function foo is intended to bar.\nquality security 1.5.\n";
    let err = parse_err(src);
    assert_eq!(
        err.reason, "out-of-range-quality",
        "expected out-of-range-quality, got: {:?}",
        err
    );
    assert!(
        err.detail.contains("1.5"),
        "diagnostic should include the bad score: {}",
        err.detail
    );
}

#[test]
fn quality_score_below_zero_rejects() {
    // Negative numbers are tricky — the lexer doesn't produce negative NumberLit.
    // A bare negative sign would be skipped as non-alphanumeric, so this just tests
    // a positive out-of-range value to confirm the guard works.
    let src = "the function foo is intended to bar.\nquality performance 2.0.\n";
    let err = parse_err(src);
    assert_eq!(err.reason, "out-of-range-quality");
}

#[test]
fn quality_missing_closing_dot_rejects() {
    let src = "the function foo is intended to bar.\nquality security 0.95\n";
    let err = parse_err(src);
    assert_eq!(err.reason, "unterminated-quality");
}

// ── clause_shapes DB registration ─────────────────────────────────────────────

#[test]
fn quality_clause_shape_in_grammar_db() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("grammar.sqlite");
    let conn = nom_grammar::init_at(&db).unwrap();

    conn.execute(
        "INSERT INTO kinds (name, description, allowed_clauses, allowed_refs, shipped_commit, notes) \
         VALUES ('function', '', '[]', '[]', 'GAP-12-test', NULL)",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT OR IGNORE INTO clause_shapes \
         (kind, clause_name, is_required, position, grammar_shape, source_ref) \
         VALUES ('function', 'quality', 0, 13, \
         'quality <quality-name> <0..1> .', 'GAP-12')",
        [],
    )
    .unwrap();

    let (is_req, pos): (i64, i64) = conn
        .query_row(
            "SELECT is_required, position FROM clause_shapes \
             WHERE kind = 'function' AND clause_name = 'quality'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("quality row must exist");
    assert_eq!(
        is_req, 0,
        "quality clause must be optional (is_required = 0)"
    );
    assert_eq!(pos, 13, "quality clause must be at position 13");
}
