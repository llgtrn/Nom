//! P2 — Determinism proof (Phase E of the blueprint).
//!
//! Claim: same input + same DB state → same PipelineOutput, 100 runs
//! in a row. Any HashMap iteration order, random tiebreaker, or
//! clock-dependent behavior would surface as a divergent run.
//!
//! Scoring: the parser currently returns structured data (not a
//! pre-hashed AST), so determinism is observed via Debug repr
//! equality across the 100 iterations. If a `Hash` impl ships
//! later, this test can upgrade to hash-equality.

use nom_concept::stages::run_pipeline_with_grammar;

fn open_baseline() -> (tempfile::TempDir, rusqlite::Connection) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("grammar.sqlite");
    let conn = nom_grammar::init_at(&db).expect("init");
    let baseline = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("nom-grammar")
        .join("data")
        .join("baseline.sql");
    let sql = std::fs::read_to_string(&baseline).expect("baseline.sql exists");
    conn.execute_batch(&sql).expect("import baseline");
    (dir, conn)
}

/// Curated input set. Real empirical curation would draw from the
/// archived translation corpus; this first cut exercises both
/// success and typical-failure shapes to bound both code paths.
const INPUTS: &[&str] = &[
    "",
    "\n\n",
    "the function greet is intended to say hello.\n",
    "the concept agent_demo is intended to demo.\n",
    "garbage %%%%%%%%%%%%%\n",
    "the function f requires x. ensures y. favor availability.\n",
];

#[test]
fn pipeline_is_deterministic_across_100_runs() {
    let (_dir, conn) = open_baseline();

    for input in INPUTS {
        let first = format!("{:?}", run_pipeline_with_grammar(input, &conn));
        for i in 1..100 {
            let nth = format!("{:?}", run_pipeline_with_grammar(input, &conn));
            assert_eq!(
                first, nth,
                "divergent run #{i} on input {input:?}:\nfirst = {first}\nnth   = {nth}"
            );
        }
    }
}
