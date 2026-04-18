//! B8 — Translation corpus expansion (16 paradigm translations).
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

// ── 1. Lazy evaluation ────────────────────────────────────────────────────────

#[test]
fn test_natural_lazy_evaluation() {
    let source =
        "@nomx natural\ndefine defer_work that produces a thunk holding unevaluated computation";
    assert_natural(source);
}

// ── 2. Tail-call optimisation ─────────────────────────────────────────────────

#[test]
fn test_natural_tail_call_optimisation() {
    let source = "@nomx natural\ndefine loop_accumulate that calls itself in tail position without growing the stack";
    assert_natural(source);
}

// ── 3. Pattern matching ───────────────────────────────────────────────────────

#[test]
fn test_natural_pattern_matching() {
    let source = "@nomx natural\ndefine classify_shape that inspects structure and dispatches to the matching branch";
    assert_natural(source);
}

// ── 4. Algebraic types ────────────────────────────────────────────────────────

#[test]
fn test_natural_algebraic_types() {
    let source =
        "@nomx natural\ndefine build_result that yields either a success value or a failure reason";
    assert_natural(source);
}

// ── 5. Monadic bind ───────────────────────────────────────────────────────────

#[test]
fn test_natural_monadic_bind() {
    let source = "@nomx natural\ndefine chain_optional that sequences two computations each of which may produce nothing";
    assert_natural(source);
}

// ── 6. Type inference ─────────────────────────────────────────────────────────

#[test]
fn test_natural_type_inference() {
    let source = "@nomx natural\ndefine infer_type that derives the precise type from available context without annotation";
    assert_natural(source);
}

// ── 7. Gradual typing ─────────────────────────────────────────────────────────

#[test]
fn test_natural_gradual_typing() {
    let source = "@nomx natural\ndefine accept_dynamic that allows untyped values to flow through where static guarantees are absent";
    assert_natural(source);
}

// ── 8. Duck typing ────────────────────────────────────────────────────────────

#[test]
fn test_natural_duck_typing() {
    let source = "@nomx natural\ndefine invoke_method that calls any value exposing the required capability regardless of declared origin";
    assert_natural(source);
}

// ── 9. Continuation passing ───────────────────────────────────────────────────

#[test]
fn test_natural_continuation_passing() {
    let source = "@nomx natural\ndefine pass_continuation that receives a callback representing the rest of the computation";
    assert_natural(source);
}

// ── 10. Structural subtyping ──────────────────────────────────────────────────

#[test]
fn test_natural_structural_subtyping() {
    let source = "@nomx natural\ndefine check_compatibility that accepts any value whose shape contains the required fields";
    assert_natural(source);
}

// ── 11. Refinement types ──────────────────────────────────────────────────────

#[test]
fn test_natural_refinement_types() {
    let source = "@nomx natural\ndefine validate_positive that restricts integers to values strictly above zero";
    assert_natural(source);
}

// ── 12. Dependent types ───────────────────────────────────────────────────────

#[test]
fn test_natural_dependent_types() {
    let source = "@nomx natural\ndefine size_safe_access that takes an index proven smaller than the collection length";
    assert_natural(source);
}

// ── 13. Linear types ──────────────────────────────────────────────────────────

#[test]
fn test_natural_linear_types() {
    let source = "@nomx natural\ndefine consume_once that takes ownership and ensures the value is used exactly one time";
    assert_natural(source);
}

// ── 14. Effect handlers ───────────────────────────────────────────────────────

#[test]
fn test_natural_effect_handlers() {
    let source = "@nomx natural\ndefine handle_output that intercepts print effects and redirects them to a buffer";
    assert_natural(source);
}

// ── 15. Row polymorphism ──────────────────────────────────────────────────────

#[test]
fn test_natural_row_polymorphism() {
    let source = "@nomx natural\ndefine extend_record that adds a field to any record regardless of other fields present";
    assert_natural(source);
}

// ── 16. Capability-based security ────────────────────────────────────────────

#[test]
fn test_natural_capability_based_security() {
    let source = "@nomx natural\ndefine grant_capability that passes an unforgeable token authorising a specific operation";
    assert_natural(source);
}
