//! Rust ↔ Nom parity checks for self-host canonical strings.
//!
//! The self-host `.nom` scaffolds claim to mirror canonical tags
//! emitted by the Rust implementation. This test closes the loop:
//! the expected values live as `pub const` in
//! [`nom_types::self_host_tags`]. Each .nom scaffold must contain
//! `return "<const_value>"` as a literal for its corresponding helper.
//!
//! Drift fails CI from either side:
//!   - Rust-side: change `nom_types::self_host_tags::DEFAULT_ON_FAIL`
//!     and this test catches the `.nom` file still holding the old
//!     string.
//!   - Nom-side: change `planner.nom::default_on_fail()` and this
//!     test catches it not matching the live const.
//!
//! Lightweight by design: reads .nom as text, searches for
//! `return "<expected>"` patterns. No dependency on nom-llvm.

use nom_types::self_host_tags as tags;
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

fn assert_returns_literal(src: &str, value: &str, where_from: &str) {
    let needle = format!("return \"{value}\"");
    assert!(
        src.contains(&needle),
        "self-host {where_from} must contain `{needle}` (canonical tag parity with nom_types::self_host_tags)"
    );
}

#[test]
fn planner_canonical_tags_match_rust_consts() {
    let src = self_host_file("planner.nom");
    assert_returns_literal(&src, tags::DEFAULT_ON_FAIL, "planner.nom::default_on_fail");
    assert_returns_literal(&src, tags::EDGE_KIND_CALLS, "planner.nom::edge_kind_calls");
    assert_returns_literal(&src, tags::EDGE_KIND_DEPENDS_ON, "planner.nom::edge_kind_depends_on");
    assert_returns_literal(&src, tags::EDGE_KIND_CONSTRAINS, "planner.nom::edge_kind_constrains");
}

#[test]
fn codegen_canonical_tags_match_rust_consts() {
    let src = self_host_file("codegen.nom");
    assert_returns_literal(&src, tags::RUST_TY_I64, "codegen.nom::rust_ty_i64");
    assert_returns_literal(&src, tags::RUST_TY_STRING, "codegen.nom::rust_ty_string");
    assert_returns_literal(&src, tags::RUST_TY_BOOL, "codegen.nom::rust_ty_bool");
    assert_returns_literal(&src, tags::DEFAULT_ENTRY_SYMBOL, "codegen.nom::default_entry_symbol");
}

#[test]
fn verifier_canonical_tags_match_rust_consts() {
    let src = self_host_file("verifier.nom");
    assert_returns_literal(&src, tags::EFFECT_PURE, "verifier.nom::effect_pure");
    assert_returns_literal(&src, tags::EFFECT_READS, "verifier.nom::effect_reads");
    assert_returns_literal(&src, tags::EFFECT_WRITES, "verifier.nom::effect_writes");
    assert_returns_literal(&src, tags::EFFECT_PANICS, "verifier.nom::effect_panics");
}

#[test]
fn parser_classifier_tags_match_rust_consts() {
    let src = self_host_file("parser.nom");
    assert_returns_literal(&src, tags::CLASSIFIER_NOM, "parser.nom::nom_classifier");
    assert_returns_literal(&src, tags::CLASSIFIER_FLOW, "parser.nom::flow_classifier");
    assert_returns_literal(&src, tags::CLASSIFIER_STORE, "parser.nom::store_classifier");
    assert_returns_literal(&src, tags::CLASSIFIER_GRAPH, "parser.nom::graph_classifier");
    assert_returns_literal(&src, tags::CLASSIFIER_SYSTEM, "parser.nom::system_classifier");
    assert_returns_literal(&src, tags::CLASSIFIER_AGENT, "parser.nom::agent_classifier");
    assert_returns_literal(&src, tags::CLASSIFIER_TEST, "parser.nom::test_classifier");
    assert_returns_literal(&src, tags::CLASSIFIER_GATE, "parser.nom::gate_classifier");
    assert_returns_literal(&src, tags::CLASSIFIER_POOL, "parser.nom::pool_classifier");
    assert_returns_literal(&src, tags::CLASSIFIER_VIEW, "parser.nom::view_classifier");
}

/// CLASSIFIERS_ALL in the Rust module must contain every individual
/// constant. Guards against adding one without the other.
#[test]
fn classifiers_all_covers_each_individual_const() {
    use tags::CLASSIFIERS_ALL;
    for c in &[
        tags::CLASSIFIER_NOM,
        tags::CLASSIFIER_FLOW,
        tags::CLASSIFIER_STORE,
        tags::CLASSIFIER_GRAPH,
        tags::CLASSIFIER_SYSTEM,
        tags::CLASSIFIER_AGENT,
        tags::CLASSIFIER_TEST,
        tags::CLASSIFIER_GATE,
        tags::CLASSIFIER_POOL,
        tags::CLASSIFIER_VIEW,
    ] {
        assert!(
            CLASSIFIERS_ALL.contains(c),
            "CLASSIFIERS_ALL missing {c}"
        );
    }
    assert_eq!(CLASSIFIERS_ALL.len(), 10);
}

#[test]
fn edge_kinds_all_covers_each_individual_const() {
    for c in &[
        tags::EDGE_KIND_CALLS,
        tags::EDGE_KIND_DEPENDS_ON,
        tags::EDGE_KIND_CONSTRAINS,
    ] {
        assert!(tags::EDGE_KINDS_ALL.contains(c), "EDGE_KINDS_ALL missing {c}");
    }
    assert_eq!(tags::EDGE_KINDS_ALL.len(), 3);
}

#[test]
fn effects_all_covers_each_individual_const() {
    for c in &[
        tags::EFFECT_PURE,
        tags::EFFECT_READS,
        tags::EFFECT_WRITES,
        tags::EFFECT_PANICS,
    ] {
        assert!(tags::EFFECTS_ALL.contains(c), "EFFECTS_ALL missing {c}");
    }
    assert_eq!(tags::EFFECTS_ALL.len(), 4);
}

#[test]
fn rust_tys_all_covers_each_individual_const() {
    for c in &[tags::RUST_TY_I64, tags::RUST_TY_STRING, tags::RUST_TY_BOOL] {
        assert!(tags::RUST_TYS_ALL.contains(c), "RUST_TYS_ALL missing {c}");
    }
    assert_eq!(tags::RUST_TYS_ALL.len(), 3);
}

#[test]
fn decl_kinds_all_covers_each_individual_const() {
    for c in &[
        tags::DECL_KIND_FN,
        tags::DECL_KIND_STRUCT,
        tags::DECL_KIND_ENUM,
    ] {
        assert!(tags::DECL_KINDS_ALL.contains(c), "DECL_KINDS_ALL missing {c}");
    }
    assert_eq!(tags::DECL_KINDS_ALL.len(), 3);
}

#[test]
fn prim_types_all_covers_each_individual_const() {
    for c in &[
        tags::PRIM_TYPE_INTEGER,
        tags::PRIM_TYPE_TEXT,
        tags::PRIM_TYPE_BOOL,
    ] {
        assert!(tags::PRIM_TYPES_ALL.contains(c), "PRIM_TYPES_ALL missing {c}");
    }
    assert_eq!(tags::PRIM_TYPES_ALL.len(), 3);
}

/// Cross-check: codegen's RUST_TYS_ALL must be the same length as
/// PRIM_TYPES_ALL — each Nom primitive must lower to exactly one
/// Rust primitive. A mismatch means lower_type() has a gap or an
/// extra arm.
#[test]
fn prim_types_and_rust_tys_are_same_cardinality() {
    assert_eq!(
        tags::PRIM_TYPES_ALL.len(),
        tags::RUST_TYS_ALL.len(),
        "Nom primitive count must match Rust-target primitive count — lower_type() bijection broken"
    );
}

#[test]
fn ast_decl_kind_tags_match_rust_consts() {
    let src = self_host_file("ast.nom");
    assert_returns_literal(&src, tags::DECL_KIND_FN, "ast.nom::decl_kind_fn");
    assert_returns_literal(&src, tags::DECL_KIND_STRUCT, "ast.nom::decl_kind_struct");
    assert_returns_literal(&src, tags::DECL_KIND_ENUM, "ast.nom::decl_kind_enum");
    assert_returns_literal(&src, tags::PRIM_TYPE_INTEGER, "ast.nom::prim_type_integer");
    assert_returns_literal(&src, tags::PRIM_TYPE_TEXT, "ast.nom::prim_type_text");
    assert_returns_literal(&src, tags::PRIM_TYPE_BOOL, "ast.nom::prim_type_bool");
}
