//! B9 — Translation corpus expansion (20 more paradigm translations).
//!
//! Each test encodes a distinct programming paradigm or language feature
//! expressed in the `.nomx natural` surface grammar.  All assertions are
//! purely structural (format detection + no-panic) so they remain stable
//! as the parser matures.

use nom_concept::stages::{NomxFormat, detect_format};

// ── helper ────────────────────────────────────────────────────────────────────

fn assert_natural(source: &str) {
    let fmt = detect_format(source);
    assert_eq!(
        fmt,
        Some(NomxFormat::Natural),
        "expected NomxFormat::Natural for source: {source:?}"
    );
}

// ── 1. Actor model ────────────────────────────────────────────────────────────

#[test]
fn test_natural_actor_model() {
    let source = "@nomx natural\ndefine spawn_actor that creates an isolated process communicating only through message passing";
    assert_natural(source);
}

// ── 2. CSP channels ───────────────────────────────────────────────────────────

#[test]
fn test_natural_csp_channels() {
    let source = "@nomx natural\ndefine send_on_channel that transmits a value and blocks until the receiver is ready";
    assert_natural(source);
}

// ── 3. Reactive streams ───────────────────────────────────────────────────────

#[test]
fn test_natural_reactive_streams() {
    let source = "@nomx natural\ndefine subscribe_to_stream that observes an asynchronous sequence applying back-pressure to producers";
    assert_natural(source);
}

// ── 4. Lens optics ────────────────────────────────────────────────────────────

#[test]
fn test_natural_lens_optics() {
    let source = "@nomx natural\ndefine focus_field that reads and updates a nested value without disturbing surrounding structure";
    assert_natural(source);
}

// ── 5. Free monad ─────────────────────────────────────────────────────────────

#[test]
fn test_natural_free_monad() {
    let source = "@nomx natural\ndefine build_program_tree that encodes effects as data deferring interpretation to the caller";
    assert_natural(source);
}

// ── 6. Cofree comonad ─────────────────────────────────────────────────────────

#[test]
fn test_natural_cofree_comonad() {
    let source = "@nomx natural\ndefine annotate_tree that labels every node with context derived from its surroundings";
    assert_natural(source);
}

// ── 7. Trampoline recursion ───────────────────────────────────────────────────

#[test]
fn test_natural_trampoline_recursion() {
    let source = "@nomx natural\ndefine bounce_step that returns either a final result or a suspended thunk to prevent stack overflow";
    assert_natural(source);
}

// ── 8. Zipper traversal ───────────────────────────────────────────────────────

#[test]
fn test_natural_zipper_traversal() {
    let source = "@nomx natural\ndefine navigate_zipper that moves focus through a data structure while tracking the path back to the root";
    assert_natural(source);
}

// ── 9. Phantom types ──────────────────────────────────────────────────────────

#[test]
fn test_natural_phantom_types() {
    let source = "@nomx natural\ndefine tag_with_state that attaches a compile-time marker carrying no runtime representation";
    assert_natural(source);
}

// ── 10. Rank-2 polymorphism ───────────────────────────────────────────────────

#[test]
fn test_natural_rank2_polymorphism() {
    let source = "@nomx natural\ndefine apply_universally that accepts a function polymorphic over all types at the call site";
    assert_natural(source);
}

// ── 11. Session types ─────────────────────────────────────────────────────────

#[test]
fn test_natural_session_types() {
    let source = "@nomx natural\ndefine open_session that enforces a typed protocol ensuring send and receive operations occur in sequence";
    assert_natural(source);
}

// ── 12. Affine types ──────────────────────────────────────────────────────────

#[test]
fn test_natural_affine_types() {
    let source = "@nomx natural\ndefine use_at_most_once that guarantees a resource is consumed no more than one time";
    assert_natural(source);
}

// ── 13. Ownership and borrowing ───────────────────────────────────────────────

#[test]
fn test_natural_ownership_borrowing() {
    let source = "@nomx natural\ndefine lend_reference that grants temporary read access without transferring ownership to the callee";
    assert_natural(source);
}

// ── 14. Gradual typing (distinct from b8 test_natural_gradual_typing) ─────────

#[test]
fn test_natural_gradual_typing_boundary() {
    let source = "@nomx natural\ndefine cross_type_boundary that inserts a runtime check where typed and untyped regions meet";
    assert_natural(source);
}

// ── 15. Refinement types (distinct from b8 test_natural_refinement_types) ─────

#[test]
fn test_natural_refinement_predicate() {
    let source = "@nomx natural\ndefine narrow_with_predicate that restricts a base type to values satisfying a compile-verified condition";
    assert_natural(source);
}

// ── 16. Capability-based security (distinct from b8) ─────────────────────────

#[test]
fn test_natural_capability_attenuation() {
    let source = "@nomx natural\ndefine attenuate_capability that wraps an authority token restricting it to a narrower set of operations";
    assert_natural(source);
}

// ── 17. Memoisation ───────────────────────────────────────────────────────────

#[test]
fn test_natural_memoisation() {
    let source = "@nomx natural\ndefine cache_result that stores the output of a pure computation and returns it on repeated identical inputs";
    assert_natural(source);
}

// ── 18. Dataflow programming ──────────────────────────────────────────────────

#[test]
fn test_natural_dataflow_programming() {
    let source = "@nomx natural\ndefine wire_nodes that connects computation cells so outputs automatically propagate to dependent inputs";
    assert_natural(source);
}

// ── 19. Logic programming ─────────────────────────────────────────────────────

#[test]
fn test_natural_logic_programming() {
    let source = "@nomx natural\ndefine query_relations that searches a set of facts finding bindings that satisfy all stated constraints";
    assert_natural(source);
}

// ── 20. Term rewriting ────────────────────────────────────────────────────────

#[test]
fn test_natural_term_rewriting() {
    let source = "@nomx natural\ndefine apply_rewrite_rule that replaces a matching sub-expression with its canonical equivalent form";
    assert_natural(source);
}
