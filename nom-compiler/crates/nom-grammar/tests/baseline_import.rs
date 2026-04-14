//! Phase D — end-to-end test that the canonical baseline.sql data
//! file imports cleanly against a freshly initialized grammar.sqlite
//! and produces the expected row counts. Confirms the grammar is
//! genuinely DB-driven (data ships as SQL, not Rust const arrays).

use std::path::PathBuf;

#[test]
fn baseline_sql_imports_into_fresh_db() {
    let baseline_path: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("baseline.sql");
    let baseline_sql = std::fs::read_to_string(&baseline_path).expect("baseline.sql exists");

    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("grammar.sqlite");
    let conn = nom_grammar::init_at(&db).expect("init schema");

    // Apply baseline.sql via execute_batch (same path the CLI uses).
    conn.execute_batch(&baseline_sql)
        .expect("baseline import must succeed");

    let counts = nom_grammar::counts(&conn).expect("count");
    println!("after baseline import: {counts:?}");

    // Concrete canonical numbers — drift catches regressions.
    assert_eq!(counts.kinds, 9, "9 closed kinds in baseline");
    assert_eq!(counts.quality_names, 20, "10 founding + 10 corpus-driven qualities");
    assert!(counts.keywords >= 40, "≥40 reserved tokens, got {}", counts.keywords);
    assert!(
        counts.clause_shapes >= 40,
        "≥40 per-kind clause rows, got {}",
        counts.clause_shapes
    );
    // keyword_synonyms carries corpus-driven rewrites; patterns carries
    // the canonical authoring shapes extracted from doc 14.
    assert_eq!(counts.patterns, 148, "148 canonical patterns");
    assert_eq!(counts.keyword_synonyms, 7, "7 corpus-driven synonyms");
}

#[test]
fn baseline_sql_is_idempotent() {
    let baseline_path: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("baseline.sql");
    let baseline_sql = std::fs::read_to_string(&baseline_path).expect("baseline.sql exists");

    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("grammar.sqlite");
    let conn = nom_grammar::init_at(&db).expect("init schema");

    conn.execute_batch(&baseline_sql).expect("first import");
    let counts1 = nom_grammar::counts(&conn).expect("count1");
    conn.execute_batch(&baseline_sql).expect("second import");
    let counts2 = nom_grammar::counts(&conn).expect("count2");

    assert_eq!(counts1, counts2, "INSERT OR IGNORE → idempotent");
}

#[test]
fn baseline_sql_has_no_foreign_language_names() {
    // Read baseline.sql as text and assert none of the banned
    // foreign-language-programming-language names appear in it.
    // This is a shallow audit; the full P6 proof in Phase E will run
    // SQL queries against every text column.
    let baseline_path: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("baseline.sql");
    let sql = std::fs::read_to_string(&baseline_path).expect("baseline.sql exists");
    let lower = sql.to_lowercase();
    // Sample of the most-likely-offender names from the no-foreign-names memory.
    // The full list lives in C:\Users\trngh\.claude\projects\...\memory\feedback_no_foreign_language_in_db.md
    for banned in [
        " rust ", " python ", " java ", " erlang ", " elixir ", " coq ",
        " ocaml ", " haskell ", " kotlin ", " elm ", " idris ", " dafny ",
        " pony ", " smalltalk ", " forth ", " perl ", " lua ", " ruby ",
        " swift ", " scala ", " clojure ", " lisp ",
    ] {
        assert!(
            !lower.contains(banned),
            "baseline.sql contains banned foreign-language name {banned:?}"
        );
    }
}
