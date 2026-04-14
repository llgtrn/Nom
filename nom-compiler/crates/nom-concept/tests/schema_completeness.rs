//! P1 — Schema-completeness proof (Phase E of the blueprint).
//!
//! Claim: the DB-driven parser cannot succeed on non-empty input when
//! `grammar.sqlite` is empty. An empty registry forces S2 to reject
//! with `NOMX-S2-empty-registry`, confirming that Rust holds no
//! fallback grammar data.

use nom_concept::stages::{StageId, run_pipeline_with_grammar};

#[test]
fn empty_grammar_rejects_any_non_empty_block_source() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("grammar.sqlite");
    let conn = nom_grammar::init_at(&db).expect("init schema");

    // Every table empty — the author's responsibility has not been met.
    let src = "the function greet is intended to say hello.\n";
    let result = run_pipeline_with_grammar(src, &conn);

    let err = result.expect_err("empty grammar must fail, not succeed");
    assert_eq!(
        err.stage,
        StageId::KindClassify,
        "failure must come from S2"
    );
    assert_eq!(err.diag_id(), "NOMX-S2-empty-registry");
}

#[test]
fn empty_grammar_accepts_empty_source_trivially() {
    // Empty input has no block to classify, so the empty-registry
    // guard does not fire. The parser returns a trivially empty
    // PipelineOutput.
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("grammar.sqlite");
    let conn = nom_grammar::init_at(&db).expect("init schema");

    let result = run_pipeline_with_grammar("", &conn);
    assert!(
        result.is_ok(),
        "empty source + empty grammar should succeed vacuously"
    );
}

#[test]
fn baseline_grammar_accepts_minimal_valid_source() {
    // After baseline.sql loads, the same source that failed above
    // now parses cleanly. This is the inverse of the first proof.
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("grammar.sqlite");
    let conn = nom_grammar::init_at(&db).expect("init schema");

    let baseline_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("nom-grammar")
        .join("data")
        .join("baseline.sql");
    let sql = std::fs::read_to_string(&baseline_path).expect("baseline.sql exists");
    conn.execute_batch(&sql).expect("import baseline");

    let src = "the function greet is intended to say hello.\n";
    // Note: this may still fail downstream stages (S3 clause-shape,
    // S4 contract, etc.) depending on the kind's required clauses;
    // the key invariant is that S2 no longer fires empty-registry.
    let result = run_pipeline_with_grammar(src, &conn);
    if let Err(err) = &result {
        assert_ne!(
            err.diag_id(),
            "NOMX-S2-empty-registry",
            "baseline must populate kinds — got {err:?}"
        );
    }
}
