//! Pattern-catalog completion bar — every `patterns.example_shape` in
//! the canonical baseline parses through the DB-driven pipeline.
//!
//! Doc 09 records the catalog completion bar: every captured insight
//! must have a pattern row whose `example_shape` parses cleanly AND
//! whose `intent` uniquely matches the captured class of intents. This
//! file enforces the first half: load baseline.sql into a fresh
//! grammar.sqlite, fetch every patterns.example_shape, feed it through
//! `run_pipeline_with_grammar`, and report the pass/fail distribution.
//!
//! Three binding invariants:
//!   1. Baseline must seed at least 200 patterns (sanity)
//!   2. The pipeline never panics on any example_shape
//!   3. Row counts on grammar.sqlite are unchanged before/after the
//!      sweep — patterns must never trigger an INSERT
//!
//! One observational test prints the pass/fail distribution by stage
//! so authors can target the largest bars when tightening the catalog.

use nom_concept::stages::run_pipeline_with_grammar;
use std::panic::AssertUnwindSafe;

fn baseline_sql_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("nom-grammar")
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

fn fetch_example_shapes(conn: &rusqlite::Connection) -> Vec<(String, String)> {
    let mut stmt = conn
        .prepare("SELECT pattern_id, example_shape FROM patterns ORDER BY pattern_id")
        .expect("prepare");
    stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .expect("query")
        .map(|r| r.expect("row"))
        .collect()
}

#[test]
fn baseline_seeds_at_least_two_hundred_patterns() {
    let (_dir, conn) = open_baseline();
    let rows = fetch_example_shapes(&conn);
    assert!(
        rows.len() >= 200,
        "baseline must seed ≥200 patterns; got {}",
        rows.len()
    );
}

#[test]
fn pipeline_never_panics_on_any_example_shape() {
    let (_dir, conn) = open_baseline();
    let rows = fetch_example_shapes(&conn);
    for (id, shape) in &rows {
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            run_pipeline_with_grammar(shape, &conn)
        }));
        assert!(
            result.is_ok(),
            "parser panicked on pattern '{id}' example_shape: {shape:?}"
        );
    }
}

#[test]
fn parsing_example_shapes_does_not_grow_grammar() {
    let (_dir, conn) = open_baseline();
    let rows = fetch_example_shapes(&conn);
    let before = nom_grammar::counts(&conn).expect("counts before");
    for (_id, shape) in &rows {
        let _ = run_pipeline_with_grammar(shape, &conn);
    }
    let after = nom_grammar::counts(&conn).expect("counts after");
    assert_eq!(
        before, after,
        "grammar.sqlite row counts changed while parsing pattern example shapes"
    );
}

#[test]
fn every_pattern_intent_is_distinct() {
    // First-cut enforcement of the doc 09 'intent uniquely matches'
    // half of the catalog completion bar. Exact-string distinct is
    // the coarse layer; the fuzzy layer below tightens it.
    let (_dir, conn) = open_baseline();
    let mut stmt = conn
        .prepare(
            "SELECT intent, COUNT(*) AS n, GROUP_CONCAT(pattern_id) AS ids \
             FROM patterns GROUP BY intent HAVING n > 1 ORDER BY n DESC",
        )
        .expect("prepare");
    let dupes: Vec<(String, i64, String)> = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, String>(2)?)))
        .expect("query")
        .map(|r| r.expect("row"))
        .collect();
    assert!(
        dupes.is_empty(),
        "patterns with duplicate intent strings: {dupes:#?}"
    );
}

#[test]
fn every_pattern_intent_pair_jaccard_below_threshold() {
    // Fuzzy semantic-similarity tightening of the uniqueness bar:
    // closes the doc 09 open question that was previously queued for
    // the cycle that lands embedding-driven resolver re-rank.
    //
    // The bar uses a deterministic token-overlap (Jaccard) similarity
    // over normalized domain-word sets — no embedding model, no
    // network, no nondeterministic dependency. Two patterns whose
    // domain-word sets share more than half their union are almost
    // certainly redundant; the threshold 0.5 is set with ~2× margin
    // over the current catalog's observed maximum (0.273 on the
    // staged-release-channel vs approval-workflow-reviewer pair, which
    // share "route"/"ordered" but are semantically distinct).
    //
    // Future tightening: once embeddings ship, a parallel test with a
    // semantic-similarity backend can run alongside this one. The
    // Jaccard test stays as the deterministic floor.
    const THRESHOLD: f64 = 0.5;

    let (_dir, conn) = open_baseline();
    let mut stmt = conn
        .prepare("SELECT pattern_id, intent FROM patterns ORDER BY pattern_id")
        .expect("prepare");
    let rows: Vec<(String, String, std::collections::BTreeSet<String>)> = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .expect("query")
        .map(|r| {
            let (id, intent) = r.expect("row");
            let toks = nom_grammar::fuzzy_tokens(&intent);
            (id, intent, toks)
        })
        .collect();

    let mut hits: Vec<(f64, String, String, String, String)> = Vec::new();
    for i in 0..rows.len() {
        for j in (i + 1)..rows.len() {
            let (ref a_id, ref a_text, ref a_set) = rows[i];
            let (ref b_id, ref b_text, ref b_set) = rows[j];
            let jacc = nom_grammar::jaccard(a_set, b_set);
            if jacc >= THRESHOLD {
                hits.push((jacc, a_id.clone(), b_id.clone(), a_text.clone(), b_text.clone()));
            }
        }
    }

    assert!(
        hits.is_empty(),
        "patterns with intent Jaccard >= {THRESHOLD}: {hits:#?}"
    );
}

#[test]
fn pattern_example_shapes_dashboard() {
    // Pure observational: report pass/fail per stage so the catalog
    // completion bar can be tightened over time. Not a gate — many
    // example_shapes are AI-client templates with `<placeholder>`
    // markers that aren't valid Word tokens; those legitimately fail
    // S2 or later. The dashboard surfaces which patterns parse and
    // which don't, so authors can rewrite shapes without placeholders
    // when they want them to flow through the parser.
    let (_dir, conn) = open_baseline();
    let rows = fetch_example_shapes(&conn);

    let mut passed = 0usize;
    let mut failed_by_stage: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for (_id, shape) in &rows {
        match run_pipeline_with_grammar(shape, &conn) {
            Ok(_) => passed += 1,
            Err(err) => {
                *failed_by_stage.entry(err.stage.code().to_string()).or_insert(0) += 1;
            }
        }
    }

    println!(
        "pattern example_shape sweep: {}/{} parsed end-to-end; fails by stage: {:?}",
        passed,
        rows.len(),
        failed_by_stage
    );
}
