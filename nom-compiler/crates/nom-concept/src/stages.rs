//! W4-A4b — annotator-style staged-parser scaffold (doc 18).
//!
//! This module houses the named stage functions that will replace the
//! monolithic `parse_nom` / `parse_nomtu` over the course of sub-wedges
//! A4b + A4c. In this first landing (A4b), every stage is a stub with
//! its target signature pinned; the real bodies fill in incrementally.
//!
//! Pipeline (doc 18 §2):
//!
//! ```text
//! source_text
//!   │
//!   ▼
//!  [S1] tokenize      : &str             → TokenStream
//!   │
//!   ▼
//!  [S2] kind_classify : TokenStream      → ClassifiedStream
//!   │
//!   ▼
//!  [S3] shape_extract : ClassifiedStream → ShapedStream
//!   │
//!   ▼
//!  [S4] contract_bind : ShapedStream     → ContractedStream
//!   │
//!   ▼
//!  [S5] effect_bind   : ContractedStream → EffectedStream
//!   │
//!   ▼
//!  [S6] ref_resolve   : EffectedStream   → NomFile | NomtuFile
//! ```
//!
//! Each stage is a pure total function — it consumes every allotted
//! token and returns the next typed AST, or rejects with a structured
//! `ConceptError::StageFailure { stage, ... }`. No stage swallows
//! ambiguity.
//!
//! **Status 2026-04-14 (A4b):** scaffold only. Stage functions unwrap
//! into their todo markers; A4c lands the real bodies.

use crate::lex::Spanned;

/// Which stage of the annotator pipeline a failure came from.
///
/// Editors can colour squiggly diagnostics by stage id: lexical
/// failures (S1) get one treatment, structural failures (S2-S3) get
/// another, semantic failures (S4-S6) get a third.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StageId {
    /// S1 tokenize — convert `&str` into `Vec<Spanned>`.
    Tokenize,
    /// S2 kind_classify — mark every top-level block's kind + bounds.
    KindClassify,
    /// S3 shape_extract — extract signature + intent + body spans.
    ShapeExtract,
    /// S4 contract_bind — pull out `requires` / `ensures` clauses.
    ContractBind,
    /// S5 effect_bind — pull out `benefit` / `hazard` clauses.
    EffectBind,
    /// S6 ref_resolve — assemble the final `NomFile` / `NomtuFile`.
    RefResolve,
}

impl StageId {
    /// Human-readable stage name used in error messages. The editor
    /// diagnostic id has the form `NOMX-S<N>-<reason>` — this helper
    /// returns the `S<N>` prefix.
    pub fn code(&self) -> &'static str {
        match self {
            StageId::Tokenize     => "S1",
            StageId::KindClassify => "S2",
            StageId::ShapeExtract => "S3",
            StageId::ContractBind => "S4",
            StageId::EffectBind   => "S5",
            StageId::RefResolve   => "S6",
        }
    }

    /// Long-form stage name for authoring-tool UIs. Stable across versions.
    pub fn label(&self) -> &'static str {
        match self {
            StageId::Tokenize     => "tokenize",
            StageId::KindClassify => "kind_classify",
            StageId::ShapeExtract => "shape_extract",
            StageId::ContractBind => "contract_bind",
            StageId::EffectBind   => "effect_bind",
            StageId::RefResolve   => "ref_resolve",
        }
    }
}

/// Materialized output of stage S1.
///
/// This is the current A4b shape — A4c will extend with richer
/// per-stage typed outputs. The initial implementation delegates to
/// `lex::collect_all_tokens` shipped in A4a.
#[derive(Debug, Clone)]
pub struct TokenStream {
    pub toks: Vec<Spanned>,
    pub source_len: usize,
}

/// S1 — Materialize every token from the source.
///
/// Wraps `lex::collect_all_tokens` into the typed-stream contract.
/// Empty source yields a `TokenStream` with an empty `toks` vector.
/// Never fails today; the return type is `Result` for symmetry with
/// downstream stages that can reject.
pub fn stage1_tokenize(src: &str) -> Result<TokenStream, StageFailure> {
    Ok(TokenStream {
        toks: crate::lex::collect_all_tokens(src),
        source_len: src.len(),
    })
}

/// Stage-attributed failure variant.  Carries the stage id plus a
/// human-readable reason; callers (editor diagnostics, `nom parse`
/// CLI, LSP) can format this for surface rendering.
///
/// This is the future `ConceptError::StageFailure` payload; lives
/// here in the scaffold so we can land A4b without widening the
/// crate's top-level error enum prematurely. A4c will bubble the
/// variant into `ConceptError` and thread it through every stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageFailure {
    pub stage: StageId,
    /// Byte offset into the source where the stage gave up.
    pub position: usize,
    /// Short reason ID following the `NOMX-S<N>-<slug>` convention.
    pub reason: &'static str,
    /// Free-form human-readable detail for diagnostic rendering.
    pub detail: String,
}

impl StageFailure {
    /// Build a failure tagged with the given stage + reason.
    pub fn new(stage: StageId, position: usize, reason: &'static str, detail: impl Into<String>) -> Self {
        Self {
            stage,
            position,
            reason,
            detail: detail.into(),
        }
    }

    /// Diagnostic id — e.g. `"NOMX-S2-kindless-block"`.
    pub fn diag_id(&self) -> String {
        format!("NOMX-{}-{}", self.stage.code(), self.reason)
    }
}

// ── A4b stage function stubs ──────────────────────────────────────────────
//
// Each stub asserts its signature contract and panics on invocation.
// A4c replaces bodies with real stage logic. Until then the monolithic
// `parse_nom` / `parse_nomtu` remain the actual parsers.

/// S2 stub — will classify every top-level `the Kind Name is …` block.
pub fn stage2_kind_classify(_stream: &TokenStream) -> Result<(), StageFailure> {
    // A4c body lands here. For now we return an explicit "not yet wired" failure
    // rather than panicking — callers that dispatch on StageId will handle it.
    Err(StageFailure::new(
        StageId::KindClassify,
        0,
        "not-yet-wired",
        "stage2_kind_classify body lands in A4c",
    ))
}

/// S3 stub — signature + intent + body span extraction.
pub fn stage3_shape_extract(_stream: &TokenStream) -> Result<(), StageFailure> {
    Err(StageFailure::new(
        StageId::ShapeExtract,
        0,
        "not-yet-wired",
        "stage3_shape_extract body lands in A4c",
    ))
}

/// S4 stub — contract clause extraction.
pub fn stage4_contract_bind(_stream: &TokenStream) -> Result<(), StageFailure> {
    Err(StageFailure::new(
        StageId::ContractBind,
        0,
        "not-yet-wired",
        "stage4_contract_bind body lands in A4c",
    ))
}

/// S5 stub — effect clause extraction.
pub fn stage5_effect_bind(_stream: &TokenStream) -> Result<(), StageFailure> {
    Err(StageFailure::new(
        StageId::EffectBind,
        0,
        "not-yet-wired",
        "stage5_effect_bind body lands in A4c",
    ))
}

/// S6 stub — final typed-AST assembly.
pub fn stage6_ref_resolve(_stream: &TokenStream) -> Result<(), StageFailure> {
    Err(StageFailure::new(
        StageId::RefResolve,
        0,
        "not-yet-wired",
        "stage6_ref_resolve body lands in A4c",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// a4b01: StageId codes are unique and follow the NOMX-S<N> pattern.
    #[test]
    fn a4b01_stage_codes_are_unique() {
        let all = [
            StageId::Tokenize, StageId::KindClassify, StageId::ShapeExtract,
            StageId::ContractBind, StageId::EffectBind, StageId::RefResolve,
        ];
        let codes: Vec<&str> = all.iter().map(|s| s.code()).collect();
        assert_eq!(codes, ["S1", "S2", "S3", "S4", "S5", "S6"]);
        let labels: Vec<&str> = all.iter().map(|s| s.label()).collect();
        assert_eq!(
            labels,
            ["tokenize", "kind_classify", "shape_extract", "contract_bind", "effect_bind", "ref_resolve"]
        );
    }

    /// a4b02: stage1_tokenize on empty source returns a TokenStream with
    /// an empty toks vector and source_len = 0. Never fails.
    #[test]
    fn a4b02_stage1_empty_source() {
        let stream = stage1_tokenize("").expect("S1 must not fail");
        assert!(stream.toks.is_empty());
        assert_eq!(stream.source_len, 0);
    }

    /// a4b03: stage1_tokenize on a non-trivial source yields the same
    /// token sequence as collect_all_tokens and stores source_len.
    #[test]
    fn a4b03_stage1_matches_collect_all_tokens() {
        let src = r#"the function fetch_url is given a url, returns text.
  benefit cache_hit."#;
        let stream = stage1_tokenize(src).expect("S1 must succeed");
        let expected = crate::lex::collect_all_tokens(src);
        // Same count.
        assert_eq!(stream.toks.len(), expected.len());
        // Same Tok values (Spanned.pos is Copy and order must match).
        for (a, b) in stream.toks.iter().zip(expected.iter()) {
            assert_eq!(a.tok, b.tok);
            assert_eq!(a.pos, b.pos);
        }
        assert_eq!(stream.source_len, src.len());
    }

    /// a4b04: StageFailure carries a stable diag id in the NOMX-S<N>
    /// form. Editors depend on this shape for diagnostic routing.
    #[test]
    fn a4b04_stage_failure_diag_id_shape() {
        let f = StageFailure::new(
            StageId::KindClassify,
            42,
            "kindless-block",
            "bare prose outside an entity",
        );
        assert_eq!(f.diag_id(), "NOMX-S2-kindless-block");
        assert_eq!(f.stage, StageId::KindClassify);
        assert_eq!(f.position, 42);
    }

    /// a4b05: A4b stubs S2-S6 return structured not-yet-wired failures
    /// carrying the correct StageId. Locks that the scaffold is callable
    /// and surfaces the right stage tag until A4c wires real bodies.
    #[test]
    fn a4b05_stubs_return_not_yet_wired_per_stage() {
        let src = r#"the function f is given x, returns y.
  favor correctness."#;
        let stream = stage1_tokenize(src).expect("S1");

        let cases: &[(fn(&TokenStream) -> Result<(), StageFailure>, StageId, &str)] = &[
            (stage2_kind_classify, StageId::KindClassify, "S2"),
            (stage3_shape_extract, StageId::ShapeExtract, "S3"),
            (stage4_contract_bind, StageId::ContractBind, "S4"),
            (stage5_effect_bind,   StageId::EffectBind,   "S5"),
            (stage6_ref_resolve,   StageId::RefResolve,   "S6"),
        ];
        for (stage_fn, expected_stage, expected_code) in cases {
            let err = stage_fn(&stream).expect_err("stub must return Err");
            assert_eq!(err.stage, *expected_stage);
            assert_eq!(err.reason, "not-yet-wired");
            assert!(err.diag_id().starts_with(&format!("NOMX-{}", expected_code)));
        }
    }
}
