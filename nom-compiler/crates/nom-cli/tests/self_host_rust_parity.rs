//! Rust ↔ Nom parity checks for self-host canonical strings.
//!
//! The self-host `.nom` scaffolds claim to mirror canonical tags
//! emitted by the Rust implementation (e.g. `default_on_fail` returns
//! `"abort"` in both languages). Nothing structural enforces this
//! today — the match is by convention + comment.
//!
//! This test locks the convention into CI: it greps the .nom source
//! for the expected string literals. Any drift (either side changes
//! the tag without updating the other) fails here, not at fixpoint
//! time.
//!
//! Lightweight by design: reads .nom as text, searches for
//! `return "<expected>"` patterns. Doesn't depend on nom-llvm or
//! nom-planner crates, so this runs on Windows too.

use std::path::PathBuf;

fn self_host_file(name: &str) -> String {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("stdlib/self_host")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Assert that `src` contains `return "<value>"` somewhere — the
/// return-literal pattern used by every canonical-tag helper in the
/// self-host scaffolds. Exact-match on the literal (no escaping
/// considered; all current tags are ASCII identifiers).
fn assert_returns_literal(src: &str, value: &str, where_from: &str) {
    let needle = format!("return \"{value}\"");
    assert!(
        src.contains(&needle),
        "self-host {where_from} must contain `{needle}` (canonical tag parity with Rust)"
    );
}

#[test]
fn planner_canonical_tags_match_rust_strings() {
    let src = self_host_file("planner.nom");
    // nom_planner::default_on_fail() -> "abort"
    assert_returns_literal(&src, "abort", "planner.nom::default_on_fail");
    // Edge-kind strings used by the Rust PlanEdge.kind serialization.
    assert_returns_literal(&src, "calls", "planner.nom::edge_kind_calls");
    assert_returns_literal(&src, "depends_on", "planner.nom::edge_kind_depends_on");
    assert_returns_literal(&src, "constrains", "planner.nom::edge_kind_constrains");
}

#[test]
fn codegen_canonical_tags_match_rust_strings() {
    let src = self_host_file("codegen.nom");
    // Rust primitive type names codegen lowers to.
    assert_returns_literal(&src, "i64", "codegen.nom::rust_ty_i64");
    assert_returns_literal(&src, "String", "codegen.nom::rust_ty_string");
    assert_returns_literal(&src, "bool", "codegen.nom::rust_ty_bool");
    // Canonical entry symbol that the runtime invokes.
    assert_returns_literal(&src, "nom_main", "codegen.nom::default_entry_symbol");
}

#[test]
fn verifier_canonical_tags_match_rust_strings() {
    let src = self_host_file("verifier.nom");
    // Effect tag set — matches nom_verifier::Effect variants.
    assert_returns_literal(&src, "pure", "verifier.nom::effect_pure");
    assert_returns_literal(&src, "reads", "verifier.nom::effect_reads");
    assert_returns_literal(&src, "writes", "verifier.nom::effect_writes");
    assert_returns_literal(&src, "panics", "verifier.nom::effect_panics");
}

#[test]
fn parser_classifier_tags_match_rust_strings() {
    let src = self_host_file("parser.nom");
    // Classifier keywords the Rust lexer emits.
    assert_returns_literal(&src, "nom", "parser.nom::nom_classifier");
    assert_returns_literal(&src, "flow", "parser.nom::flow_classifier");
    assert_returns_literal(&src, "store", "parser.nom::store_classifier");
    assert_returns_literal(&src, "graph", "parser.nom::graph_classifier");
}

#[test]
fn ast_decl_kind_tags_match_rust_strings() {
    let src = self_host_file("ast.nom");
    // Decl-kind tags codegen + planner consume.
    assert_returns_literal(&src, "fn", "ast.nom::decl_kind_fn");
    assert_returns_literal(&src, "struct", "ast.nom::decl_kind_struct");
    assert_returns_literal(&src, "enum", "ast.nom::decl_kind_enum");
    // Primitive type names shared with codegen's lower_type().
    assert_returns_literal(&src, "integer", "ast.nom::prim_type_integer");
    assert_returns_literal(&src, "text", "ast.nom::prim_type_text");
    assert_returns_literal(&src, "bool", "ast.nom::prim_type_bool");
}
