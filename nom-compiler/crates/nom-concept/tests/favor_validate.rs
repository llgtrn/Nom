//! Phase B4 — S5b favor-clause validator against grammar.sqlite.quality_names.

use nom_concept::stages::{
    run_pipeline_with_grammar, stage1_tokenize, stage2_kind_classify,
    stage3_shape_extract, stage4_contract_bind, stage5_effect_bind,
    stage5b_favor_validate,
};

const SRC_FAVOR: &str = r#"the function login_user is intended to verify a user's credentials.
ensures a session token is returned on success.
favor auditability.
"#;

const SRC_NO_FAVOR: &str = r#"the function login_user is intended to verify a user's credentials.
ensures a session token is returned on success.
"#;

fn baseline_to_s5(src: &str) -> nom_concept::stages::EffectedStream {
    let s1 = stage1_tokenize(src).unwrap();
    let s2 = stage2_kind_classify(&s1).unwrap();
    let s3 = stage3_shape_extract(&s2).unwrap();
    let s4 = stage4_contract_bind(&s3).unwrap();
    stage5_effect_bind(&s4).unwrap()
}

fn fresh_grammar() -> (tempfile::TempDir, rusqlite::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("grammar.sqlite");
    let conn = nom_grammar::init_at(&db).unwrap();
    (dir, conn)
}

fn insert_quality(conn: &rusqlite::Connection, name: &str) {
    conn.execute(
        "INSERT INTO quality_names (name, axis, cardinality, source_ref) \
         VALUES (?1, 'ops', 'any', 'B4-test')",
        [name],
    )
    .unwrap();
}

#[test]
fn no_favor_in_source_skips_validation_entirely() {
    // Empty quality_names table is fine when source has no favor clause.
    let (_dir, conn) = fresh_grammar();
    let s5 = baseline_to_s5(SRC_NO_FAVOR);
    let s5b = stage5b_favor_validate(&s5, &conn).expect("no favor → no check");
    // Output equals input (validator does not mutate)
    assert_eq!(s5b.blocks.len(), s5.blocks.len());
}

#[test]
fn favor_present_with_empty_quality_registry_rejects() {
    let (_dir, conn) = fresh_grammar();
    let s5 = baseline_to_s5(SRC_FAVOR);
    let result = stage5b_favor_validate(&s5, &conn);
    let err = result.expect_err("favor with empty quality_names → reject");
    assert_eq!(err.reason, "empty-quality-registry");
}

#[test]
fn favor_present_with_unregistered_name_rejects() {
    let (_dir, conn) = fresh_grammar();
    insert_quality(&conn, "totality"); // some other name; auditability NOT inserted
    let s5 = baseline_to_s5(SRC_FAVOR);
    let result = stage5b_favor_validate(&s5, &conn);
    let err = result.expect_err("favor name not in registry → reject");
    assert_eq!(err.reason, "unknown-quality-name");
    assert!(
        err.detail.contains("auditability"),
        "diagnostic must name the offending favor: {}",
        err.detail
    );
}

#[test]
fn favor_present_with_registered_name_passes() {
    let (_dir, conn) = fresh_grammar();
    insert_quality(&conn, "auditability");
    let s5 = baseline_to_s5(SRC_FAVOR);
    let s5b = stage5b_favor_validate(&s5, &conn).expect("registered name → ok");
    assert_eq!(s5b.blocks.len(), s5.blocks.len());
}

#[test]
fn full_pipeline_with_grammar_passes_when_all_registries_populated() {
    // End-to-end: kinds, clause_shapes, quality_names all populated;
    // run_pipeline_with_grammar should not reject in any of S1/S2/S3/S5b.
    let (_dir, conn) = fresh_grammar();
    conn.execute(
        "INSERT INTO kinds (name, description, allowed_clauses, allowed_refs, shipped_commit, notes) \
         VALUES ('function', '', '[]', '[]', 'B4-test', NULL)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO clause_shapes (kind, clause_name, is_required, position, grammar_shape, source_ref) \
         VALUES ('function', 'intended', 1, 1, '...', 'B4-test')",
        [],
    )
    .unwrap();
    insert_quality(&conn, "auditability");

    let result = run_pipeline_with_grammar(SRC_FAVOR, &conn);
    if let Err(e) = result {
        assert_ne!(e.reason, "empty-quality-registry");
        assert_ne!(e.reason, "unknown-quality-name");
        assert_ne!(e.reason, "empty-registry");
        assert_ne!(e.reason, "empty-clause-shapes-for-kind");
        // Other reasons (e.g. ref-resolve issues from S6) are allowed —
        // we're only verifying the new B4 guard does not block this case.
    }
}
