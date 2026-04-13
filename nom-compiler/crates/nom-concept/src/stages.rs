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

use crate::lex::{Spanned, Tok};
use crate::KINDS;

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

/// Result of S2 — the `TokenStream` passed through plus a list of
/// top-level block boundaries. One entry per `the Kind Name …` declaration.
///
/// `end_tok` is approximate in this first real landing: it points at
/// the token right before the next `the` (or the end of the stream).
/// A4c-step2 will tighten it to the exact body-terminator dot.
#[derive(Debug, Clone)]
pub struct ClassifiedStream {
    pub toks: Vec<Spanned>,
    pub blocks: Vec<BlockBoundary>,
    pub source_len: usize,
}

/// One classified top-level block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockBoundary {
    /// One of `KINDS` (`function` / `module` / `concept` / `screen` /
    /// `data` / `event` / `media`).
    pub kind: String,
    /// The identifier that followed the kind noun.
    pub name: String,
    /// Index into `ClassifiedStream.toks` of the opening `Tok::The`.
    pub start_tok: usize,
    /// Exclusive end index. Approximate in A4c-step1 (next block start
    /// or end of stream). A4c-step2 narrows this to the closing dot.
    pub end_tok: usize,
    /// Byte position of the opening `the` token in the source.
    pub start_byte: usize,
}

/// S2 — Classify every top-level `the Kind Name …` block.
///
/// Read-only scan over the materialized token stream. Produces a
/// structured enumeration of block boundaries. Rejects:
///
/// - Stream starts with something other than `the`.
/// - A `the` at top-level isn't followed by a valid kind noun (from
///   `KINDS`). Unknown-kind words surface as `NOMX-S2-unknown-kind`.
/// - A `the Kind` isn't followed by an identifier `Word`. The ref-form
///   `the @Kind matching …` lives INSIDE a block body, not at top-level;
///   encountering one while expecting a top-level block means the
///   previous block didn't close properly (terminator `.` missing).
///
/// Composition: `the module Name composes …` is classified with
/// `kind = "module"` — S3 will disambiguate between module/composition
/// body shapes using the next token after the name.
pub fn stage2_kind_classify(stream: &TokenStream) -> Result<ClassifiedStream, StageFailure> {
    let toks = &stream.toks;
    let mut blocks = Vec::new();
    let mut i = 0usize;

    while i < toks.len() {
        // Scan forward until we find a top-level `the`. At this level
        // ANY token that isn't `the` is a violation (the previous block
        // didn't close, or leading garbage).
        match &toks[i].tok {
            Tok::The => {
                let start_tok = i;
                let start_byte = toks[i].pos;
                i += 1;

                // Next must be Tok::Kind(k).
                let kind = match toks.get(i) {
                    None => return Err(StageFailure::new(
                        StageId::KindClassify,
                        toks.last().map(|t| t.pos).unwrap_or(0),
                        "truncated-block-header",
                        "source ended after `the` without a kind noun",
                    )),
                    Some(Spanned { tok: Tok::Kind(k), .. }) => {
                        if !KINDS.contains(&k.as_str()) {
                            return Err(StageFailure::new(
                                StageId::KindClassify,
                                toks[i].pos,
                                "unknown-kind",
                                format!("`the {k}` is not a recognized kind noun"),
                            ));
                        }
                        k.clone()
                    }
                    Some(Spanned { tok: Tok::Word(w), pos }) => {
                        return Err(StageFailure::new(
                            StageId::KindClassify,
                            *pos,
                            "unknown-kind",
                            format!("`the {w}` — {w} is not in the closed kind set"),
                        ));
                    }
                    Some(other) => return Err(StageFailure::new(
                        StageId::KindClassify,
                        other.pos,
                        "expected-kind-noun",
                        "top-level `the` must be followed by a kind noun",
                    )),
                };
                i += 1;

                // Next must be Tok::Word(name).
                let name = match toks.get(i) {
                    None => return Err(StageFailure::new(
                        StageId::KindClassify,
                        toks.last().map(|t| t.pos).unwrap_or(0),
                        "truncated-block-header",
                        format!("source ended after `the {kind}` without a name"),
                    )),
                    Some(Spanned { tok: Tok::Word(w), .. }) => w.clone(),
                    Some(other) => return Err(StageFailure::new(
                        StageId::KindClassify,
                        other.pos,
                        "expected-block-name",
                        format!("`the {kind}` must be followed by an identifier"),
                    )),
                };
                i += 1;

                // Record. end_tok is finalized once we find the NEXT
                // `Tok::The` at top-level (or at end of stream) — we
                // push a preliminary value and patch it on the next
                // iteration's bookkeeping below.
                blocks.push(BlockBoundary {
                    kind,
                    name,
                    start_tok,
                    end_tok: toks.len(), // will be tightened below
                    start_byte,
                });

                // Advance past this block's body by skipping forward
                // until we hit another top-level `the` or EOF. For
                // A4c-step1 the heuristic is: skip until we see a
                // `Tok::The` that is preceded by a `Tok::Dot` (period).
                // That signals a new top-level block. Inline `the`
                // inside an entity ref is typically NOT preceded by a
                // dot — it sits after a keyword like `uses` or a
                // comma — so this is a conservative first pass.
                let mut prev_was_dot = false;
                while i < toks.len() {
                    match &toks[i].tok {
                        Tok::The if prev_was_dot => break,
                        Tok::Dot => {
                            prev_was_dot = true;
                            i += 1;
                        }
                        _ => {
                            prev_was_dot = false;
                            i += 1;
                        }
                    }
                }
                // Tighten end_tok of the just-pushed block to `i`.
                let last_idx = blocks.len() - 1;
                blocks[last_idx].end_tok = i;
            }
            other => {
                // Top-level position, non-`the` token: strictness-lane
                // violation. Editors surface this as "bare prose outside
                // an entity block".
                return Err(StageFailure::new(
                    StageId::KindClassify,
                    toks[i].pos,
                    "kindless-block",
                    format!(
                        "expected `the` at top level, found `{}` (stray tokens outside an entity/concept block)",
                        tok_shortname(other)
                    ),
                ));
            }
        }
    }

    Ok(ClassifiedStream {
        toks: toks.clone(),
        blocks,
        source_len: stream.source_len,
    })
}

fn tok_shortname(t: &Tok) -> String {
    match t {
        Tok::Word(w) => format!("word `{w}`"),
        Tok::Kind(k) => format!("kind `{k}`"),
        Tok::Quoted(q) => format!("\"{q}\""),
        Tok::NumberLit(n) => format!("number `{n}`"),
        Tok::The => "the".into(),
        Tok::Is => "is".into(),
        Tok::Dot => ".".into(),
        _ => format!("{t:?}"),
    }
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

    /// a4b05: A4b stubs S3-S6 return structured not-yet-wired failures
    /// carrying the correct StageId. Locks the scaffold is callable
    /// and surfaces the right stage tag until later sub-wedges wire
    /// real bodies. (S2 was wired in A4c-step1, see a4c01-a4c05.)
    #[test]
    fn a4b05_stubs_return_not_yet_wired_per_stage() {
        let src = r#"the function f is given x, returns y.
  favor correctness."#;
        let stream = stage1_tokenize(src).expect("S1");

        let cases: &[(fn(&TokenStream) -> Result<(), StageFailure>, StageId, &str)] = &[
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

    // ── A4c-step1: S2 kind_classify body ──────────────────────────────────

    /// a4c01: classify a single-entity `.nomtu` source.
    #[test]
    fn a4c01_single_entity_classified() {
        let src = r#"the function fetch_url is given a url, returns text.
  benefit cache_hit."#;
        let stream = stage1_tokenize(src).expect("S1");
        let classified = stage2_kind_classify(&stream).expect("S2 must classify");
        assert_eq!(classified.blocks.len(), 1);
        assert_eq!(classified.blocks[0].kind, "function");
        assert_eq!(classified.blocks[0].name, "fetch_url");
    }

    /// a4c02: classify a two-entity `.nomtu` source.
    #[test]
    fn a4c02_two_entities_classified() {
        let src = r#"the function fetch_url is given a url, returns text.
  benefit cache_hit.

the function read_file is given a path, returns bytes.
  benefit fast_path."#;
        let stream = stage1_tokenize(src).expect("S1");
        let classified = stage2_kind_classify(&stream).expect("S2 must classify");
        assert_eq!(classified.blocks.len(), 2);
        assert_eq!(classified.blocks[0].name, "fetch_url");
        assert_eq!(classified.blocks[1].name, "read_file");
        // Second block's start_tok comes AFTER the first block's end_tok.
        assert!(classified.blocks[0].end_tok <= classified.blocks[1].start_tok);
    }

    /// a4c03: classify a `.nom` concept source with `intended to …`.
    #[test]
    fn a4c03_concept_classified() {
        let src = r#"the concept auth_system is
  intended to authenticate users via jwt.
  uses the @Function matching "token verify" with at-least 0.85 confidence.
  favor correctness."#;
        let stream = stage1_tokenize(src).expect("S1");
        let classified = stage2_kind_classify(&stream).expect("S2 must classify");
        assert_eq!(classified.blocks.len(), 1);
        assert_eq!(classified.blocks[0].kind, "concept");
        assert_eq!(classified.blocks[0].name, "auth_system");
    }

    /// a4c04: bare prose at top level fails S2 with kindless-block.
    #[test]
    fn a4c04_bare_prose_fails_kindless_block() {
        let src = r#"some random prose here.
the function f is given x, returns y.
  favor correctness."#;
        let stream = stage1_tokenize(src).expect("S1");
        let err = stage2_kind_classify(&stream).expect_err("S2 must reject");
        assert_eq!(err.stage, StageId::KindClassify);
        assert_eq!(err.reason, "kindless-block");
        assert!(err.diag_id().contains("S2-kindless-block"));
    }

    /// a4c05: `the <notakind> name` rejects with unknown-kind.
    #[test]
    fn a4c05_unknown_kind_rejected() {
        let src = r#"the banana fetch_url is given x, returns y.
  favor correctness."#;
        let stream = stage1_tokenize(src).expect("S1");
        let err = stage2_kind_classify(&stream).expect_err("S2 must reject");
        assert_eq!(err.stage, StageId::KindClassify);
        assert_eq!(err.reason, "unknown-kind");
    }
}
