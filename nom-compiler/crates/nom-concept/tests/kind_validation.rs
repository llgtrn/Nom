//! Phase B2 — S2 kind validation against grammar.sqlite.kinds.
//!
//! Proves the strict invariant: an empty kinds table forces every
//! source to fail; a populated table accepts only the kinds it lists.

use nom_concept::stages::{
    run_pipeline_with_grammar, stage1_tokenize, stage2_kind_classify_with_grammar,
};

const SRC_FUNCTION: &str = r#"the function greet is intended to print a greeting.
"#;

const SRC_PROPERTY: &str = r#"the property addition_is_commutative is
  intended to assert that natural-number addition is commutative.
  generator pairs of natural numbers from 0 to 100.
"#;

fn fresh_grammar() -> (tempfile::TempDir, rusqlite::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("grammar.sqlite");
    let conn = nom_grammar::init_at(&db).unwrap();
    (dir, conn)
}

fn insert_kind(conn: &rusqlite::Connection, name: &str) {
    conn.execute(
        "INSERT INTO kinds (name, description, allowed_clauses, allowed_refs, shipped_commit, notes) \
         VALUES (?1, '', '[]', '[]', 'phaseB2-test', NULL)",
        [name],
    )
    .unwrap();
}

#[test]
fn empty_kinds_table_rejects_every_block() {
    let (_dir, conn) = fresh_grammar();
    let s1 = stage1_tokenize(SRC_FUNCTION).unwrap();
    let result = stage2_kind_classify_with_grammar(&s1, &conn);
    let err = result.expect_err("empty kinds must reject");
    assert_eq!(err.reason, "empty-registry");
}

#[test]
fn populated_kinds_accept_listed_kind() {
    let (_dir, conn) = fresh_grammar();
    insert_kind(&conn, "function");
    let s1 = stage1_tokenize(SRC_FUNCTION).unwrap();
    let classified = stage2_kind_classify_with_grammar(&s1, &conn)
        .expect("function kind row present → S2 must accept");
    assert_eq!(classified.blocks.len(), 1);
    assert_eq!(classified.blocks[0].kind, "function");
    assert_eq!(classified.blocks[0].name, "greet");
}

#[test]
fn populated_kinds_reject_kind_not_in_registry() {
    let (_dir, conn) = fresh_grammar();
    insert_kind(&conn, "function"); // present
    // property NOT inserted

    let s1 = stage1_tokenize(SRC_PROPERTY).unwrap();
    let result = stage2_kind_classify_with_grammar(&s1, &conn);
    let err = result.expect_err("property kind not in registry → reject");
    assert_eq!(err.reason, "unknown-kind");
    assert!(
        err.detail.contains("property"),
        "diagnostic must name the offending kind: {}",
        err.detail
    );
}

#[test]
fn empty_source_with_empty_kinds_does_not_fail() {
    // An empty source has no blocks, so the empty-registry guard does
    // not fire. This preserves the existing "empty source = clean
    // no-op" invariant from W4-A6.
    let (_dir, conn) = fresh_grammar();
    let s1 = stage1_tokenize("").unwrap();
    let classified = stage2_kind_classify_with_grammar(&s1, &conn)
        .expect("empty source must not invoke kind validation");
    assert!(classified.blocks.is_empty());
}

#[test]
fn run_pipeline_with_grammar_full_path_with_populated_registry() {
    let (_dir, conn) = fresh_grammar();
    insert_kind(&conn, "function");
    let result = run_pipeline_with_grammar(SRC_FUNCTION, &conn);
    // Even with kinds populated, downstream stages may still reject for
    // other reasons (missing intent, etc.). What we're verifying here
    // is that the S2 kind check doesn't block this minimal example.
    // If it returns Ok, the pipeline is fully grammar-aware. If it
    // returns Err, the failure must come from S3/S4/S5/S6, not S2.
    if let Err(e) = result {
        assert_ne!(
            e.reason, "empty-registry",
            "S2 must not fire empty-registry once kinds row is present"
        );
        assert_ne!(
            e.reason, "unknown-kind",
            "S2 must not fire unknown-kind once 'function' row is present"
        );
    }
}
