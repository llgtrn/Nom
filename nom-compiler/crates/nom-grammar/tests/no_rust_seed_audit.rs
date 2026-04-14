//! P7 — no-Rust-bundled-data audit.
//!
//! Claim: zero const arrays in nom-grammar/src/ carry grammar data.
//! Only the schema SQL (SCHEMA_SQL) and version stamp
//! (SCHEMA_VERSION) are allowed as Rust constants. Any other
//! `pub const X: &[...]` array of tuples or structs in this crate
//! is a regression — a reintroduction of bundled seed data — and
//! CI fails on it.

use std::fs;
use std::path::PathBuf;

#[test]
fn nom_grammar_src_has_no_seed_const_arrays() {
    let src_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offenses: Vec<(String, String)> = Vec::new();

    let entries = fs::read_dir(&src_dir).expect("read src dir");
    for entry in entries {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        let content = fs::read_to_string(&path).expect("read rust file");
        let file_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("<unknown>")
            .to_string();

        for (lineno, line) in content.lines().enumerate() {
            let trimmed = line.trim_start();
            if !trimmed.starts_with("pub const ") && !trimmed.starts_with("const ") {
                continue;
            }

            // Allowed constants — the schema SQL text and the version
            // number. Anything else that is a slice or array of
            // non-trivial elements is a red flag.
            if trimmed.contains("SCHEMA_SQL") || trimmed.contains("SCHEMA_VERSION") {
                continue;
            }

            // Heuristic: does this const declare a slice or array
            // (contains `&[` or `[`) holding tuples or structs? That's
            // the shape of a seed table.
            let looks_like_collection =
                trimmed.contains("&[(") || trimmed.contains(": &[") || trimmed.contains(": [");
            if !looks_like_collection {
                continue;
            }

            // Primitive slice types are still allowed (e.g. &[u8]).
            // A seed array carries tuples (&str, &str, …) or structs.
            if trimmed.contains(": &[u8]")
                || trimmed.contains(": &[&str]") && !trimmed.contains("(")
            {
                continue;
            }

            offenses.push((
                file_name.clone(),
                format!("line {}: {}", lineno + 1, trimmed),
            ));
        }
    }

    assert!(
        offenses.is_empty(),
        "nom-grammar/src/ has reintroduced bundled seed data; grammar rows must live in the DB, not Rust source:\n{offenses:#?}"
    );
}
