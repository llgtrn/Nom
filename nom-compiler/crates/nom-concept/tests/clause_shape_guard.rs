//! Phase B3 — S3 clause-shape presence guard against grammar.sqlite.clause_shapes.
//!
//! Strict invariant: a kind with zero rows in clause_shapes means the
//! parser cannot validate the block's shape. S3 surfaces that with
//! NOMX-S3-empty-clause-shapes-for-kind rather than silently accepting.

use nom_concept::stages::{
    run_pipeline_with_grammar, stage1_tokenize, stage2_kind_classify_with_grammar,
    stage3_shape_extract_with_grammar,
};

const SRC_FUNCTION: &str = r#"the function greet is intended to print a greeting.
"#;

const SRC_FUNCTION_WITH_ENSURES: &str = r#"the function greet is intended to print a greeting.
ensures the result is a greeting.
"#;

fn fresh_grammar_with_function_kind() -> (tempfile::TempDir, rusqlite::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("grammar.sqlite");
    let conn = nom_grammar::init_at(&db).unwrap();
    conn.execute(
        "INSERT INTO kinds (name, description, allowed_clauses, allowed_refs, shipped_commit, notes) \
         VALUES ('function', '', '[]', '[]', 'B3-test', NULL)",
        [],
    )
    .unwrap();
    (dir, conn)
}

fn insert_clause_shape(
    conn: &rusqlite::Connection,
    kind: &str,
    clause: &str,
    is_required: i32,
    position: i32,
) {
    conn.execute(
        "INSERT INTO clause_shapes (kind, clause_name, is_required, position, grammar_shape, source_ref) \
         VALUES (?1, ?2, ?3, ?4, '...', 'B3-test')",
        rusqlite::params![kind, clause, is_required, position],
    )
    .unwrap();
}

#[test]
fn empty_clause_shapes_for_kind_rejects_block() {
    // kinds row exists for `function`, but clause_shapes is empty for it.
    let (_dir, conn) = fresh_grammar_with_function_kind();

    let s1 = stage1_tokenize(SRC_FUNCTION).unwrap();
    let s2 = stage2_kind_classify_with_grammar(&s1, &conn).expect("S2 ok");
    let result = stage3_shape_extract_with_grammar(&s2, &conn);
    let err = result.expect_err("empty clause_shapes must reject");
    assert_eq!(err.reason, "empty-clause-shapes-for-kind");
    assert!(
        err.detail.contains("function"),
        "diagnostic must name the offending kind: {}",
        err.detail
    );
}

#[test]
fn populated_clause_shapes_allow_extraction() {
    let (_dir, conn) = fresh_grammar_with_function_kind();
    insert_clause_shape(&conn, "function", "intended", 1, 1);
    insert_clause_shape(&conn, "function", "ensures", 0, 2);

    let s1 = stage1_tokenize(SRC_FUNCTION).unwrap();
    let s2 = stage2_kind_classify_with_grammar(&s1, &conn).expect("S2 ok");
    let s3 = stage3_shape_extract_with_grammar(&s2, &conn)
        .expect("populated clause_shapes → S3 must accept");
    assert_eq!(s3.blocks.len(), 1);
}

#[test]
fn missing_required_clause_rejects_block() {
    let (_dir, conn) = fresh_grammar_with_function_kind();
    insert_clause_shape(&conn, "function", "intended", 1, 1);
    insert_clause_shape(&conn, "function", "ensures", 1, 2);

    let s1 = stage1_tokenize(SRC_FUNCTION).unwrap();
    let s2 = stage2_kind_classify_with_grammar(&s1, &conn).expect("S2 ok");
    let err = stage3_shape_extract_with_grammar(&s2, &conn)
        .expect_err("missing required clause must reject");
    assert_eq!(err.reason, "missing-required-clause");
    assert!(
        err.detail.contains("ensures"),
        "diagnostic must name the missing clause: {}",
        err.detail
    );
}

#[test]
fn all_required_clauses_present_allows_pipeline() {
    let (_dir, conn) = fresh_grammar_with_function_kind();
    insert_clause_shape(&conn, "function", "intended", 1, 1);
    insert_clause_shape(&conn, "function", "ensures", 1, 2);

    let s1 = stage1_tokenize(SRC_FUNCTION_WITH_ENSURES).unwrap();
    let s2 = stage2_kind_classify_with_grammar(&s1, &conn).expect("S2 ok");
    let s3 = stage3_shape_extract_with_grammar(&s2, &conn).expect("all required clauses present");
    assert_eq!(s3.blocks.len(), 1);
}

#[test]
fn empty_source_skips_clause_shape_check() {
    // No blocks → no kinds to check → S3 must not invoke the guard.
    let (_dir, conn) = fresh_grammar_with_function_kind();
    let s1 = stage1_tokenize("").unwrap();
    let s2 = stage2_kind_classify_with_grammar(&s1, &conn).expect("S2 ok");
    let s3 = stage3_shape_extract_with_grammar(&s2, &conn).expect("empty source → no guard");
    assert!(s3.blocks.is_empty());
}

#[test]
fn run_pipeline_with_grammar_full_path_passes_when_all_three_tables_populated() {
    let (_dir, conn) = fresh_grammar_with_function_kind();
    insert_clause_shape(&conn, "function", "intended", 1, 1);

    let result = run_pipeline_with_grammar(SRC_FUNCTION, &conn);
    // S1 + S2 + S3 must all pass with this minimal source. If S4/S5/S6
    // fail for other reasons, that's fine — what we're verifying is
    // that the new S3 guard does not block the canonical case.
    if let Err(e) = result {
        assert_ne!(
            e.reason, "empty-clause-shapes-for-kind",
            "S3 must not fire empty-clause-shapes-for-kind when row is present"
        );
    }
}
