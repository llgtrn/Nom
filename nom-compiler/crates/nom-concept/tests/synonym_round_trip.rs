//! P5 proof — synonym round-trip at the full pipeline level.
//!
//! Adding a row to grammar.sqlite.keyword_synonyms changes the parser's
//! behaviour without recompiling. Removing the row reverts the
//! behaviour. No code change in between. This is the operational proof
//! that the parser is genuinely DB-driven, not falling back to
//! hardcoded rules.

use nom_concept::stages::{run_pipeline, run_pipeline_with_grammar, stage1_tokenize, stage1_tokenize_with_synonyms};

const SOURCE_WITH_SYNONYM: &str = r#"the function login_user is intended to verify a user's credentials and issue a session.
expects credentials are non-empty.
ensures a session token is returned on success.
"#;

const SOURCE_CANONICAL: &str = r#"the function login_user is intended to verify a user's credentials and issue a session.
requires credentials are non-empty.
ensures a session token is returned on success.
"#;

#[test]
fn synonym_round_trip_at_pipeline_level() {
    use rusqlite::Connection;

    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("grammar.sqlite");
    let conn = nom_grammar::init_at(&db).expect("init");

    // Step 1: with no synonym row, the surface "expects" remains a Tok::Word
    // and downstream stages see no Requires keyword for that line. The
    // pipeline either parses (treating it as plain prose inside the body)
    // or rejects in a stage downstream from S1; either way, the canonical
    // tokenize+grammar version produces the SAME outcome as the
    // synonym-less tokenize since there's no row to apply.
    let raw_no_grammar = stage1_tokenize(SOURCE_WITH_SYNONYM).expect("S1 raw");
    let raw_with_empty_grammar = stage1_tokenize_with_synonyms(SOURCE_WITH_SYNONYM, &conn)
        .expect("S1 with empty grammar");
    assert_eq!(
        raw_no_grammar.toks.len(),
        raw_with_empty_grammar.toks.len(),
        "empty synonym table must not change the token count"
    );

    // Step 2: insert "expects" → "requires" row.
    conn.execute(
        "INSERT INTO keyword_synonyms (synonym, canonical_keyword, source_ref, shipped_commit, notes) \
         VALUES ('expects', 'requires', 'P5-test', 'test', NULL)",
        [],
    )
    .expect("insert synonym");

    // Step 3: the synonym-aware tokenize now produces a token stream
    // EQUAL to the one produced from the canonical-source variant.
    let canonical_tokens = stage1_tokenize(SOURCE_CANONICAL).expect("canonical S1");
    let rewritten_tokens = stage1_tokenize_with_synonyms(SOURCE_WITH_SYNONYM, &conn)
        .expect("synonym-rewritten S1");

    let canon_kinds: Vec<String> = canonical_tokens
        .toks
        .iter()
        .map(|t| format!("{:?}", t.tok))
        .collect();
    let rewrite_kinds: Vec<String> = rewritten_tokens
        .toks
        .iter()
        .map(|t| format!("{:?}", t.tok))
        .collect();
    assert_eq!(
        canon_kinds, rewrite_kinds,
        "with the synonym row present, the surface 'expects' must produce \
         the same token stream as the canonical 'requires'"
    );

    // Step 4: drop the row → behaviour reverts to step 1.
    conn.execute("DELETE FROM keyword_synonyms WHERE synonym = 'expects'", [])
        .expect("delete row");
    let after_delete = stage1_tokenize_with_synonyms(SOURCE_WITH_SYNONYM, &conn)
        .expect("S1 after delete");
    let after_delete_kinds: Vec<String> = after_delete
        .toks
        .iter()
        .map(|t| format!("{:?}", t.tok))
        .collect();
    let raw_kinds: Vec<String> = raw_no_grammar
        .toks
        .iter()
        .map(|t| format!("{:?}", t.tok))
        .collect();
    assert_eq!(
        after_delete_kinds, raw_kinds,
        "removing the synonym row must restore pre-insert behaviour"
    );
}

#[test]
fn synonym_aware_pipeline_matches_baseline_when_grammar_populated() {
    // The grammar-aware pipeline produces the SAME PipelineOutput as the
    // baseline pipeline once the grammar tables carry the rules
    // downstream stages need. With ONLY the kinds rows populated (S2's
    // requirement), the pipeline succeeds end-to-end on a canonical
    // source that the baseline also accepts.
    //
    // Note: an EMPTY grammar.sqlite would correctly fail S2 with
    // NOMX-S2-empty-registry — that is the strict invariant from the
    // blueprint; the grammar-aware pipeline is NOT a "pure extension"
    // in that sense. Rather, it is a strict gate that requires the DB
    // to be populated before parsing.
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("grammar.sqlite");
    let conn = nom_grammar::init_at(&db).expect("init");

    // Populate only the kinds row that S2 needs for SOURCE_CANONICAL.
    conn.execute(
        "INSERT INTO kinds (name, description, allowed_clauses, allowed_refs, shipped_commit, notes) \
         VALUES ('function', '', '[]', '[]', 'B2-test', NULL)",
        [],
    )
    .expect("seed kind");

    let baseline = run_pipeline(SOURCE_CANONICAL).expect("baseline");
    let with_grammar = run_pipeline_with_grammar(SOURCE_CANONICAL, &conn).expect("with grammar");

    // Compare via debug formatting (no PartialEq on PipelineOutput today).
    assert_eq!(format!("{baseline:?}"), format!("{with_grammar:?}"));
}

#[test]
fn empty_grammar_correctly_rejects_in_strict_mode() {
    // The strict invariant from the blueprint: an empty grammar.sqlite
    // forces the pipeline to fail at S2's empty-registry check. This
    // proves the grammar-aware pipeline is genuinely DB-driven, not
    // falling back to hardcoded rules.
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("grammar.sqlite");
    let conn = nom_grammar::init_at(&db).expect("init");

    let result = run_pipeline_with_grammar(SOURCE_CANONICAL, &conn);
    let err = result.expect_err("empty grammar must reject");
    assert_eq!(err.reason, "empty-registry");
}

#[test]
fn synonym_must_lex_to_single_canonical_token_or_pipeline_rejects() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("grammar.sqlite");
    let conn = nom_grammar::init_at(&db).expect("init");

    // Insert a malformed synonym whose canonical text lexes to multiple tokens.
    conn.execute(
        "INSERT INTO keyword_synonyms (synonym, canonical_keyword, source_ref, shipped_commit, notes) \
         VALUES ('expects', 'requires the', 'malformed-test', 'test', NULL)",
        [],
    )
    .expect("insert");

    let result = stage1_tokenize_with_synonyms(SOURCE_WITH_SYNONYM, &conn);
    let err = result.expect_err("multi-token canonical must reject");
    assert_eq!(err.reason, "multitoken-synonym");
}
