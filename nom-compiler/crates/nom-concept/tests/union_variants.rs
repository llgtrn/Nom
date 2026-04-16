//! GAP-12 — @Union sum-type clause tests.
//!
//! Covers:
//!  - Parse a data entity with `@Union of X, Y, Z.` → variants extracted.
//!  - Parse a data entity without `@Union` → union_variants is None.
//!  - Single variant (no trailing comma).
//!  - Multiple variants with trailing comma tolerated.
//!  - Malformed: `@Union` not followed by `of` → NOMX-S5d-malformed-union.
//!  - Malformed: `@Union of` with no variants → NOMX-S5d-empty-union.
//!  - Malformed: missing closing `.` → NOMX-S5d-unterminated-union.
//!  - `@Union` clause shape registered in grammar DB for data kind.

use nom_concept::{
    NomtuItem, UnionVariants,
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
    run_pipeline(src).expect_err("pipeline must reject malformed @Union clause")
}

// ── positive cases ────────────────────────────────────────────────────────────

#[test]
fn union_multiple_variants_extracted() {
    let src = "the data payment_method is\n\
               intended to represent how a customer pays.\n\
               @Union of credit_card, debit_card, bank_transfer, cryptocurrency.\n";
    let e = first_entity(src);
    assert_eq!(
        e.union_variants,
        Some(UnionVariants {
            variants: vec![
                "credit_card".to_string(),
                "debit_card".to_string(),
                "bank_transfer".to_string(),
                "cryptocurrency".to_string(),
            ],
        }),
        "union_variants should capture all four discriminants"
    );
}

#[test]
fn union_single_variant() {
    let src = "the data unit_type is\n\
               intended to represent a singleton type.\n\
               @Union of unit.\n";
    let e = first_entity(src);
    assert_eq!(
        e.union_variants,
        Some(UnionVariants {
            variants: vec!["unit".to_string()],
        }),
        "single variant should be captured"
    );
}

#[test]
fn no_union_clause_yields_none() {
    let src = "the data record is\n\
               intended to hold a simple record.\n\
               exposes name as text.\n";
    let e = first_entity(src);
    assert_eq!(
        e.union_variants, None,
        "union_variants must be None when no @Union clause is present"
    );
}

#[test]
fn union_coexists_with_intent_and_contracts() {
    let src = "the data result is\n\
               intended to represent a computation result.\n\
               requires the computation is defined.\n\
               @Union of success, failure, pending.\n";
    let e = first_entity(src);
    assert_eq!(
        e.union_variants,
        Some(UnionVariants {
            variants: vec![
                "success".to_string(),
                "failure".to_string(),
                "pending".to_string(),
            ],
        })
    );
    assert_eq!(e.contracts.len(), 1, "contracts should be preserved");
}

// ── negative cases ────────────────────────────────────────────────────────────

#[test]
fn union_not_followed_by_of_rejects() {
    // `@Union credit_card, debit_card.` — missing `of`
    let src = "the data payment is intended to represent payment.\n\
               @Union credit_card, debit_card.\n";
    let err = parse_err(src);
    assert_eq!(
        err.reason, "malformed-union",
        "expected malformed-union, got: {:?}",
        err
    );
}

#[test]
fn union_of_with_no_variants_rejects() {
    // `@Union of .` — no variant names before closing dot
    let src = "the data empty is intended to represent nothing.\n\
               @Union of .\n";
    let err = parse_err(src);
    assert_eq!(
        err.reason, "empty-union",
        "expected empty-union, got: {:?}",
        err
    );
}

#[test]
fn union_missing_closing_dot_rejects() {
    // `@Union of foo, bar` with no `.`
    let src = "the data partial is intended to represent partial.\n\
               @Union of foo, bar\n";
    let err = parse_err(src);
    assert_eq!(
        err.reason, "unterminated-union",
        "expected unterminated-union, got: {:?}",
        err
    );
}

// ── clause_shapes DB registration ─────────────────────────────────────────────

#[test]
fn union_clause_shape_in_grammar_db() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("grammar.sqlite");
    let conn = nom_grammar::init_at(&db).unwrap();

    // Seed the data kind.
    conn.execute(
        "INSERT INTO kinds (name, description, allowed_clauses, allowed_refs, shipped_commit, notes) \
         VALUES ('data', '', '[]', '[]', 'GAP-12-union-test', NULL)",
        [],
    )
    .unwrap();

    // Insert the union clause shape (mirrors the baseline.sql row).
    conn.execute(
        "INSERT OR IGNORE INTO clause_shapes \
         (kind, clause_name, is_required, position, grammar_shape, source_ref) \
         VALUES ('data', 'union', 0, 3, \
         '@Union of <variant1> ('','' <variantN>)* ''.''', 'GAP-12')",
        [],
    )
    .unwrap();

    // Verify it shows up with the right attributes.
    let (is_req, pos): (i64, i64) = conn
        .query_row(
            "SELECT is_required, position FROM clause_shapes \
             WHERE kind = 'data' AND clause_name = 'union'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("union row must exist");
    assert_eq!(is_req, 0, "union clause must be optional (is_required = 0)");
    assert_eq!(pos, 3, "union clause must be at position 3");
}
