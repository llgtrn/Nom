//! End-to-end smoke test: seed an on-disk grammar.sqlite and read every row back.

use std::path::PathBuf;

#[test]
fn seed_real_disk_db_and_read_back() {
    let db_path: PathBuf = std::env::temp_dir().join("nom_grammar_smoke.sqlite");
    let _ = std::fs::remove_file(&db_path);

    let conn = nom_grammar::init_at(&db_path).expect("init");
    assert!(db_path.exists());

    let counts = nom_grammar::seed::seed_all(&conn).expect("seed");
    println!("\nSEEDED: {counts:?}");
    // Ground-truth counts surfaced by the real on-disk seed pipeline.
    // Earlier in-memory tests under-asserted on these numbers; this test
    // pins them exactly so future drift is caught.
    assert_eq!(counts.kinds, 9);
    assert_eq!(counts.quality_names, 10);
    assert_eq!(counts.keywords, 50);
    assert_eq!(counts.clause_shapes, 43);
    assert!(
        counts.patterns >= 13,
        "expected at-least 13 native pattern rows, got {}",
        counts.patterns
    );

    drop(conn);
    let conn2 = nom_grammar::open_readonly(&db_path).expect("reopen");
    let runtime_counts = nom_grammar::counts(&conn2).expect("count");
    println!("REOPENED: {runtime_counts:?}");
    assert_eq!(runtime_counts.patterns, counts.patterns as u64);

    let mut stmt = conn2
        .prepare("SELECT pattern_id FROM patterns ORDER BY pattern_id")
        .expect("prepare");
    let pids: Vec<String> = stmt
        .query_map([], |r| r.get(0))
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("collect");
    println!("\nPATTERNS ON DISK ({}):", pids.len());
    for p in &pids {
        println!("  {p}");
    }
}
