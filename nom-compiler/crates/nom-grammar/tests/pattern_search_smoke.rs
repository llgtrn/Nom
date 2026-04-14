//! Pattern search backend smoke — run a handful of known-good queries
//! against the live baseline and assert that the expected pattern_id
//! appears in the top-K results above a meaningful threshold.
//!
//! The point of this test is NOT to characterize the full ranking; it's
//! to lock in the deterministic Jaccard backend so a future change to
//! `fuzzy_tokens`, `FUZZY_STOPWORDS`, or `jaccard` that would silently
//! drift the CLI `pattern-search` results catches a regression in CI.
//!
//! Each query is a free-form intent prose an author might type into
//! `nom grammar pattern-search`; the assertion is that the catalog
//! returns the obvious match in the top three at score ≥ 0.20.

use std::path::PathBuf;

fn baseline_sql_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("baseline.sql")
}

fn open_baseline() -> (tempfile::TempDir, rusqlite::Connection) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("grammar.sqlite");
    let conn = nom_grammar::init_at(&db).expect("init");
    let sql = std::fs::read_to_string(baseline_sql_path()).expect("baseline.sql exists");
    conn.execute_batch(&sql).expect("import baseline");
    (dir, conn)
}

/// Assert that `expected_id` appears in the top-3 matches of `query`
/// via `nom_grammar::search_patterns` (the canonical backend the CLI
/// also calls). Threshold 0.15 is set with empirical headroom for
/// the morphology friction Jaccard hits without stemming (results vs
/// result; supervise vs supervised). The catalog's natural overlap
/// among unrelated patterns is ~0.005 mean / ~0.27 max, so 0.15 still
/// rejects unrelated noise comfortably.
fn assert_top3_contains(conn: &rusqlite::Connection, query: &str, expected_id: &str) {
    let hits = nom_grammar::search_patterns(conn, query, 0.15, 3).expect("search");
    let ids: Vec<&str> = hits.iter().map(|m| m.pattern_id.as_str()).collect();
    assert!(
        ids.contains(&expected_id),
        "query {query:?} expected {expected_id} in top-3, got {ids:?} (full: {hits:#?})"
    );
}

#[test]
fn cache_pure_function_results_finds_cache_memoization() {
    let (_dir, conn) = open_baseline();
    assert_top3_contains(
        &conn,
        "cache pure function results to skip recomputation",
        "cache-memoization",
    );
}

#[test]
fn supervise_child_processes_with_restart_finds_supervised_process_tree() {
    let (_dir, conn) = open_baseline();
    assert_top3_contains(
        &conn,
        "supervise child processes with restart policies",
        "supervised-process-tree",
    );
}

#[test]
fn retry_a_transient_failure_finds_retry_policy() {
    let (_dir, conn) = open_baseline();
    assert_top3_contains(
        &conn,
        "retry a transient failure with backoff",
        "retry-policy",
    );
}

#[test]
fn validate_form_fields_finds_form_validation_pattern() {
    let (_dir, conn) = open_baseline();
    assert_top3_contains(
        &conn,
        "validate form fields against declared constraints before submission",
        "form-validation-client-side",
    );
}

#[test]
fn empty_after_stopword_filter_returns_no_matches() {
    let (_dir, conn) = open_baseline();
    let hits = nom_grammar::search_patterns(&conn, "the of a to", 0.10, 10).expect("search");
    assert!(
        hits.is_empty(),
        "all-stopword query should return no matches; got {hits:#?}"
    );
}
