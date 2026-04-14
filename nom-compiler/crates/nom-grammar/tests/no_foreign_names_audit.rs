//! P6 — no-foreign-language-name audit.
//!
//! Claim: after importing baseline.sql into a fresh grammar.sqlite,
//! zero rows in any text column of any table contain any of the banned
//! foreign-language-programming-language names, matched on whole-word
//! boundaries (so "nim" does not falsely match inside "minimum").
//! The .claude memory file `feedback_no_foreign_language_in_db.md`
//! is the canonical banned list.

use std::path::PathBuf;

const BANNED: &[&str] = &[
    "rust",
    "python",
    "java",
    "erlang",
    "elixir",
    "coq",
    "ocaml",
    "haskell",
    "kotlin",
    "elm",
    "idris",
    "dafny",
    "pony",
    "smalltalk",
    "forth",
    "perl",
    "lua",
    "ruby",
    "swift",
    "scala",
    "clojure",
    "lisp",
    "javascript",
    "typescript",
    "golang",
    "fortran",
    "cobol",
    "pascal",
    "prolog",
    "scheme",
    "racket",
    "crystal",
    "nim",
    "zig",
    "julia",
];

/// Whole-word check: `banned` is contained in `haystack` only when it
/// appears as its own token — bordered by non-alphanumeric characters
/// or the string edges.
fn contains_whole_word(haystack: &str, banned: &str) -> bool {
    let hay = haystack.to_ascii_lowercase();
    let needle = banned.to_ascii_lowercase();
    let mut start = 0usize;
    while let Some(idx) = hay[start..].find(&needle) {
        let abs = start + idx;
        let before_ok = abs == 0
            || !hay[..abs]
                .chars()
                .last()
                .map(|c| c.is_ascii_alphanumeric())
                .unwrap_or(false);
        let after = abs + needle.len();
        let after_ok = after == hay.len()
            || !hay[after..]
                .chars()
                .next()
                .map(|c| c.is_ascii_alphanumeric())
                .unwrap_or(false);
        if before_ok && after_ok {
            return true;
        }
        start = abs + needle.len();
        if start >= hay.len() {
            break;
        }
    }
    false
}

#[test]
fn baseline_grammar_has_no_foreign_language_names_in_any_row() {
    let baseline_path: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("baseline.sql");
    let sql = std::fs::read_to_string(&baseline_path).expect("baseline.sql exists");

    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("grammar.sqlite");
    let conn = nom_grammar::init_at(&db).expect("init schema");
    conn.execute_batch(&sql).expect("import baseline.sql");

    let audits: &[(&str, &[&str])] = &[
        ("schema_meta", &["key", "value"]),
        (
            "keywords",
            &[
                "token",
                "role",
                "kind_scope",
                "source_ref",
                "shipped_commit",
                "notes",
            ],
        ),
        (
            "keyword_synonyms",
            &[
                "synonym",
                "canonical_keyword",
                "source_ref",
                "shipped_commit",
                "notes",
            ],
        ),
        (
            "kinds",
            &[
                "name",
                "description",
                "allowed_clauses",
                "allowed_refs",
                "shipped_commit",
                "notes",
            ],
        ),
        (
            "clause_shapes",
            &[
                "kind",
                "clause_name",
                "one_of_group",
                "grammar_shape",
                "source_ref",
                "notes",
            ],
        ),
        (
            "quality_names",
            &[
                "name",
                "axis",
                "metric_function",
                "cardinality",
                "required_at",
                "source_ref",
                "notes",
            ],
        ),
        (
            "patterns",
            &[
                "pattern_id",
                "intent",
                "nom_kinds",
                "nom_clauses",
                "typed_slot_refs",
                "example_shape",
                "hazards",
                "favors",
                "source_doc_refs",
            ],
        ),
    ];

    let mut hits: Vec<(String, String, String, String)> = Vec::new();
    for (table, columns) in audits {
        for col in *columns {
            let sql = format!("SELECT {col} FROM {table}", col = col, table = table);
            let mut stmt = conn.prepare(&sql).expect("prepare");
            let rows: Vec<String> = stmt
                .query_map([], |r| r.get::<_, Option<String>>(0))
                .expect("query")
                .filter_map(|r| r.ok().flatten())
                .collect();
            for val in rows {
                for banned in BANNED {
                    if contains_whole_word(&val, banned) {
                        hits.push((
                            table.to_string(),
                            col.to_string(),
                            banned.to_string(),
                            val.clone(),
                        ));
                    }
                }
            }
        }
    }

    assert!(
        hits.is_empty(),
        "baseline.sql rows contain banned foreign-language names (whole-word match): {hits:#?}"
    );
}

#[test]
fn whole_word_helper_rejects_substring_false_positives() {
    assert!(!contains_whole_word("minimum_cost", "nim"));
    assert!(!contains_whole_word("purify_data", "ruby"));
    assert!(!contains_whole_word("archived_notes", "ruby"));
    assert!(contains_whole_word("use ruby style", "ruby"));
    assert!(contains_whole_word("ruby", "ruby"));
    assert!(contains_whole_word("written_in_nim", "nim"));
}
