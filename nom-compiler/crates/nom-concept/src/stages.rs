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
use crate::{
    CompositionDecl, ConceptDecl, ContractClause, EffectClause, EffectValence, EntityDecl,
    NomFile, NomtuFile, NomtuItem, KINDS,
};

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

/// Output of S3 — each block gains an extracted `intended to …`
/// phrase. This is the first shape field the pipeline fills in;
/// later A4c increments add signature + body span details.
#[derive(Debug, Clone)]
pub struct ShapedStream {
    pub toks: Vec<Spanned>,
    pub blocks: Vec<ShapedBlock>,
    pub source_len: usize,
}

/// A classified block with its intent phrase extracted (doc 17 §I6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapedBlock {
    pub kind: String,
    pub name: String,
    pub start_tok: usize,
    pub end_tok: usize,
    pub start_byte: usize,
    /// The `intended to …` sentence, with leading/trailing whitespace
    /// trimmed. Every block MUST have exactly one — S3 rejects blocks
    /// that omit it with `NOMX-S3-missing-intent`.
    pub intent: String,
}

/// S3 — Extract the `intended to …` phrase from each classified block.
///
/// Walks `classified.toks[block.start_tok..block.end_tok]` for every
/// block and finds the first `Tok::Intended → Tok::To → … → Tok::Dot`
/// sentence. The prose between `to` and the terminator becomes the
/// block's `intent` slot.
///
/// Rejects:
///
/// - `NOMX-S3-missing-intent` — block body has no `intended to …`
///   sentence at all (strictness invariant ct14).
/// - `NOMX-S3-intended-not-followed-by-to` — `intended` appeared but
///   wasn't immediately followed by `to`. The strict two-word opener
///   keeps the intent lane unambiguous.
/// - `NOMX-S3-unterminated-intent` — source ends before a `.` closes
///   the intent sentence.
///
/// Note: this S3 increment extracts ONLY the intent phrase. Signature
/// (`given X, returns Y` for entities) and body span extraction come
/// in a later A4c step.
pub fn stage3_shape_extract(classified: &ClassifiedStream) -> Result<ShapedStream, StageFailure> {
    let toks = &classified.toks;
    let mut blocks = Vec::with_capacity(classified.blocks.len());

    for b in &classified.blocks {
        let body_slice = &toks[b.start_tok..b.end_tok];
        // Find `Intended` token index (relative to body_slice).
        let intended_idx = body_slice.iter().position(|s| matches!(s.tok, Tok::Intended));
        let intended_rel = match intended_idx {
            Some(i) => i,
            None => {
                return Err(StageFailure::new(
                    StageId::ShapeExtract,
                    b.start_byte,
                    "missing-intent",
                    format!(
                        "block `the {} {}` has no `intended to …` sentence",
                        b.kind, b.name
                    ),
                ));
            }
        };
        // Next token must be `To`.
        let after_intended = intended_rel + 1;
        match body_slice.get(after_intended) {
            Some(Spanned { tok: Tok::To, .. }) => {}
            Some(other) => {
                return Err(StageFailure::new(
                    StageId::ShapeExtract,
                    other.pos,
                    "intended-not-followed-by-to",
                    format!(
                        "`intended` must be immediately followed by `to` (block `{}`)",
                        b.name
                    ),
                ));
            }
            None => {
                return Err(StageFailure::new(
                    StageId::ShapeExtract,
                    body_slice.last().map(|s| s.pos).unwrap_or(b.start_byte),
                    "unterminated-intent",
                    format!("`intended` at end of block `{}` with no following `to`", b.name),
                ));
            }
        }

        // Collect tokens after `to` until we hit a terminator.
        // Terminator: Tok::Dot at top-level within the block.
        let prose_start = after_intended + 1;
        let dot_rel = body_slice[prose_start..]
            .iter()
            .position(|s| matches!(s.tok, Tok::Dot));
        let dot_idx = match dot_rel {
            Some(n) => prose_start + n,
            None => {
                return Err(StageFailure::new(
                    StageId::ShapeExtract,
                    body_slice
                        .last()
                        .map(|s| s.pos)
                        .unwrap_or(b.start_byte),
                    "unterminated-intent",
                    format!("`intended to …` has no closing `.` in block `{}`", b.name),
                ));
            }
        };

        let intent = body_slice[prose_start..dot_idx]
            .iter()
            .map(|s| tok_prose_repr(&s.tok))
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string();

        blocks.push(ShapedBlock {
            kind: b.kind.clone(),
            name: b.name.clone(),
            start_tok: b.start_tok,
            end_tok: b.end_tok,
            start_byte: b.start_byte,
            intent,
        });
    }

    Ok(ShapedStream {
        toks: toks.clone(),
        blocks,
        source_len: classified.source_len,
    })
}

/// Render a token back into its source-level prose word. Best-effort —
/// preserves the content of Word/Quoted/Kind/NumberLit and collapses
/// other tokens to their lowercase keyword spelling.
fn tok_prose_repr(t: &Tok) -> String {
    match t {
        Tok::Word(w) => w.clone(),
        Tok::Quoted(q) => format!("\"{q}\""),
        Tok::Kind(k) => k.clone(),
        Tok::NumberLit(n) => format!("{n}"),
        Tok::The => "the".into(),
        Tok::Is => "is".into(),
        Tok::Matching => "matching".into(),
        Tok::With => "with".into(),
        Tok::AtLeast => "at-least".into(),
        Tok::Composes => "composes".into(),
        Tok::Then => "then".into(),
        Tok::To => "to".into(),
        Tok::Intended => "intended".into(),
        Tok::This => "this".into(),
        Tok::Uses => "uses".into(),
        Tok::Extends => "extends".into(),
        Tok::Favor => "favor".into(),
        Tok::Comma => ",".into(),
        _ => String::new(),
    }
}

/// Output of S4 — each shaped block gains zero-or-more typed
/// contract clauses (requires/ensures) pulled from its body.
#[derive(Debug, Clone)]
pub struct ContractedStream {
    pub toks: Vec<Spanned>,
    pub blocks: Vec<ContractedBlock>,
    pub source_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractedBlock {
    pub kind: String,
    pub name: String,
    pub start_tok: usize,
    pub end_tok: usize,
    pub start_byte: usize,
    pub intent: String,
    /// Contract clauses in source order. Empty for blocks that declare none.
    pub contracts: Vec<ContractClause>,
}

/// S4 — Pull `requires <prose>.` / `ensures <prose>.` out of each
/// shaped block's body.
///
/// Read-only walk over `shaped.toks[block.start_tok..block.end_tok]`.
/// Every `Tok::Requires` or `Tok::Ensures` must be followed by prose
/// tokens terminated by a `Tok::Dot`; the collected prose (trimmed)
/// becomes a `ContractClause::Requires` or `ContractClause::Ensures`.
///
/// Rejects:
///
/// - `NOMX-S4-unterminated-contract` — contract verb seen but no `.`
///   closes the clause before the block ends.
/// - `NOMX-S4-empty-contract` — `requires .` or `ensures .` with no
///   prose between verb and dot.
///
/// Note this increment does NOT yet extract effect clauses
/// (`benefit`/`hazard`); those land in A4c-step4 as S5.
pub fn stage4_contract_bind(shaped: &ShapedStream) -> Result<ContractedStream, StageFailure> {
    let toks = &shaped.toks;
    let mut blocks = Vec::with_capacity(shaped.blocks.len());

    for b in &shaped.blocks {
        let body_slice = &toks[b.start_tok..b.end_tok];
        let mut contracts = Vec::new();
        let mut i = 0usize;

        while i < body_slice.len() {
            let is_requires = matches!(body_slice[i].tok, Tok::Requires);
            let is_ensures  = matches!(body_slice[i].tok, Tok::Ensures);
            if !is_requires && !is_ensures {
                i += 1;
                continue;
            }
            let verb_pos = body_slice[i].pos;
            let prose_start = i + 1;
            // Find next Tok::Dot, but fail strict if we cross another
            // top-level clause keyword first (another Requires/Ensures,
            // Favor, Benefit, Hazard, Uses, Exposes). That indicates
            // the author wrote an unclosed contract.
            let verb_name = if is_requires { "requires" } else { "ensures" };
            let mut dot_idx_opt: Option<usize> = None;
            for j in prose_start..body_slice.len() {
                match &body_slice[j].tok {
                    Tok::Dot => {
                        dot_idx_opt = Some(j);
                        break;
                    }
                    Tok::Requires | Tok::Ensures | Tok::Favor | Tok::Benefit
                    | Tok::Hazard | Tok::Uses | Tok::Exposes => {
                        return Err(StageFailure::new(
                            StageId::ContractBind,
                            body_slice[j].pos,
                            "unterminated-contract",
                            format!(
                                "`{verb_name}` clause in block `{}` crosses into another clause at `{:?}` without a closing `.`",
                                b.name, body_slice[j].tok
                            ),
                        ));
                    }
                    _ => {}
                }
            }
            let dot_idx = match dot_idx_opt {
                Some(n) => n,
                None => {
                    return Err(StageFailure::new(
                        StageId::ContractBind,
                        verb_pos,
                        "unterminated-contract",
                        format!(
                            "`{verb_name}` clause in block `{}` has no closing `.`",
                            b.name
                        ),
                    ));
                }
            };
            let prose = body_slice[prose_start..dot_idx]
                .iter()
                .map(|s| tok_prose_repr(&s.tok))
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();
            if prose.is_empty() {
                return Err(StageFailure::new(
                    StageId::ContractBind,
                    verb_pos,
                    "empty-contract",
                    format!(
                        "`{}` clause in block `{}` has no predicate prose",
                        if is_requires { "requires" } else { "ensures" },
                        b.name
                    ),
                ));
            }
            contracts.push(if is_requires {
                ContractClause::Requires(prose)
            } else {
                ContractClause::Ensures(prose)
            });
            i = dot_idx + 1;
        }

        blocks.push(ContractedBlock {
            kind: b.kind.clone(),
            name: b.name.clone(),
            start_tok: b.start_tok,
            end_tok: b.end_tok,
            start_byte: b.start_byte,
            intent: b.intent.clone(),
            contracts,
        });
    }

    Ok(ContractedStream {
        toks: toks.clone(),
        blocks,
        source_len: shaped.source_len,
    })
}

/// Output of S5 — each contracted block gains zero-or-more typed
/// effect clauses (benefit / hazard, plus `boon` / `bane` synonyms).
#[derive(Debug, Clone)]
pub struct EffectedStream {
    pub toks: Vec<Spanned>,
    pub blocks: Vec<EffectedBlock>,
    pub source_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectedBlock {
    pub kind: String,
    pub name: String,
    pub start_tok: usize,
    pub end_tok: usize,
    pub start_byte: usize,
    pub intent: String,
    pub contracts: Vec<ContractClause>,
    /// Effect clauses (valence + comma-separated effect names).
    pub effects: Vec<EffectClause>,
}

/// S5 — Pull `benefit` / `hazard` effect clauses out of each
/// contracted block's body.
///
/// Walks the body span for every `Tok::Benefit` or `Tok::Hazard` token
/// (`boon` → `Tok::Benefit`, `bane` → `Tok::Hazard` per lexer synonym
/// table at lib.rs:409-412). Collects comma-separated `Tok::Word(name)`
/// entries until a closing `Tok::Dot`. Each effect name must be a
/// simple `Word` — quoted strings or numbers reject.
///
/// Rejects:
///
/// - `NOMX-S5-unterminated-effect` — effect verb hits another clause
///   keyword before a closing `.` (same cross-clause guard as S4).
/// - `NOMX-S5-empty-effect` — `benefit .` with no names.
/// - `NOMX-S5-non-word-effect-name` — name token is not a `Word`.
pub fn stage5_effect_bind(contracted: &ContractedStream) -> Result<EffectedStream, StageFailure> {
    let toks = &contracted.toks;
    let mut blocks = Vec::with_capacity(contracted.blocks.len());

    for b in &contracted.blocks {
        let body_slice = &toks[b.start_tok..b.end_tok];
        let mut effects = Vec::new();
        let mut i = 0usize;

        while i < body_slice.len() {
            let valence = match body_slice[i].tok {
                Tok::Benefit => EffectValence::Benefit,
                Tok::Hazard => EffectValence::Hazard,
                _ => {
                    i += 1;
                    continue;
                }
            };
            let verb_pos = body_slice[i].pos;
            let verb_name = match valence {
                EffectValence::Benefit => "benefit",
                EffectValence::Hazard => "hazard",
            };
            let mut names: Vec<String> = Vec::new();
            let mut j = i + 1;
            let mut saw_dot = false;
            while j < body_slice.len() {
                match &body_slice[j].tok {
                    Tok::Dot => {
                        saw_dot = true;
                        break;
                    }
                    Tok::Comma => {
                        j += 1;
                    }
                    Tok::Word(w) => {
                        names.push(w.clone());
                        j += 1;
                    }
                    Tok::Requires | Tok::Ensures | Tok::Favor | Tok::Benefit
                    | Tok::Hazard | Tok::Uses | Tok::Exposes => {
                        return Err(StageFailure::new(
                            StageId::EffectBind,
                            body_slice[j].pos,
                            "unterminated-effect",
                            format!(
                                "`{verb_name}` clause in block `{}` crosses into another clause at `{:?}` without a closing `.`",
                                b.name, body_slice[j].tok
                            ),
                        ));
                    }
                    other => {
                        return Err(StageFailure::new(
                            StageId::EffectBind,
                            body_slice[j].pos,
                            "non-word-effect-name",
                            format!(
                                "`{verb_name}` in block `{}` expects comma-separated Word names; saw `{:?}`",
                                b.name, other
                            ),
                        ));
                    }
                }
            }
            if !saw_dot {
                return Err(StageFailure::new(
                    StageId::EffectBind,
                    verb_pos,
                    "unterminated-effect",
                    format!("`{verb_name}` clause in block `{}` has no closing `.`", b.name),
                ));
            }
            if names.is_empty() {
                return Err(StageFailure::new(
                    StageId::EffectBind,
                    verb_pos,
                    "empty-effect",
                    format!("`{verb_name}` clause in block `{}` has no effect names", b.name),
                ));
            }
            effects.push(EffectClause {
                valence,
                effects: names,
            });
            i = j + 1; // past the dot
        }

        blocks.push(EffectedBlock {
            kind: b.kind.clone(),
            name: b.name.clone(),
            start_tok: b.start_tok,
            end_tok: b.end_tok,
            start_byte: b.start_byte,
            intent: b.intent.clone(),
            contracts: b.contracts.clone(),
            effects,
        });
    }

    Ok(EffectedStream {
        toks: toks.clone(),
        blocks,
        source_len: contracted.source_len,
    })
}

/// End-of-pipeline typed AST. One of `NomFile` or `NomtuFile`
/// depending on the first block's kind — `concept` blocks flow into
/// `NomFile`; `function` / `module` / `composition` / `data` / `screen` /
/// `event` / `media` blocks flow into `NomtuFile`.
#[derive(Debug, Clone)]
pub enum PipelineOutput {
    Nom(NomFile),
    Nomtu(NomtuFile),
}

/// S6 — Assemble the final typed AST from the staged outputs.
///
/// This first-landing body is SKELETAL: it populates every field the
/// stages have observed (intent / contracts / effects) but leaves
/// ref-carrying fields (`index` on concepts, `composes` on
/// compositions) empty. Later cycles wire the real ref-resolution
/// path that walks each block's remaining ref_spans into `EntityRef`
/// values.
///
/// For callers that need the full AST today, `parse_nom` / `parse_nomtu`
/// remain the production path. S6 is a pipeline output demonstrator
/// + a target shape for the upcoming full migration.
///
/// Dispatch: if ANY block's kind is `concept`, the output is a
/// `NomFile` containing those concepts only (mixed sources are an
/// authoring anti-pattern per doc 08). Otherwise the output is a
/// `NomtuFile` with `function` blocks → `NomtuItem::Entity`,
/// `composition`-style blocks not yet disambiguated from modules →
/// `NomtuItem::Composition` only when the kind is exactly `module`
/// (matches the current parser's heuristic). Bare `screen` / `data` /
/// `event` / `media` kinds become Entity decls with empty signature
/// — sufficient for the skeletal pass.
pub fn stage6_ref_resolve(effected: &EffectedStream) -> Result<PipelineOutput, StageFailure> {
    let has_concept = effected.blocks.iter().any(|b| b.kind == "concept");
    let has_non_concept = effected.blocks.iter().any(|b| b.kind != "concept");
    if has_concept && has_non_concept {
        let first = effected
            .blocks
            .iter()
            .find(|b| b.kind != "concept")
            .expect("has_non_concept");
        return Err(StageFailure::new(
            StageId::RefResolve,
            first.start_byte,
            "mixed-concept-and-entity",
            format!(
                "block `{}` (kind `{}`) mixed with a concept block — pick one per file per doc 08",
                first.name, first.kind
            ),
        ));
    }

    if has_concept {
        let concepts = effected
            .blocks
            .iter()
            .map(|b| ConceptDecl {
                name: b.name.clone(),
                intent: b.intent.clone(),
                index: Vec::new(), // ref-resolution deferred
                exposes: Vec::new(),
                acceptance: b
                    .contracts
                    .iter()
                    .map(|c| match c {
                        ContractClause::Requires(p) => format!("requires {p}"),
                        ContractClause::Ensures(p) => format!("ensures {p}"),
                    })
                    .collect(),
                objectives: Vec::new(),
            })
            .collect();
        Ok(PipelineOutput::Nom(NomFile { concepts }))
    } else {
        let items = effected
            .blocks
            .iter()
            .map(|b| {
                if b.kind == "module" {
                    NomtuItem::Composition(CompositionDecl {
                        word: b.name.clone(),
                        composes: Vec::new(), // ref-resolution deferred
                        glue: None,
                        contracts: b.contracts.clone(),
                        effects: b.effects.clone(),
                    })
                } else {
                    NomtuItem::Entity(EntityDecl {
                        kind: b.kind.clone(),
                        word: b.name.clone(),
                        signature: String::new(), // signature-shape extract deferred
                        contracts: b.contracts.clone(),
                        effects: b.effects.clone(),
                    })
                }
            })
            .collect();
        Ok(PipelineOutput::Nomtu(NomtuFile { items }))
    }
}

/// Convenience: drive the full pipeline end-to-end from source text.
/// Returns `PipelineOutput` or the first stage's structured failure.
///
/// This is the on-ramp editors / diagnostics use. The production
/// parser path (`parse_nom` / `parse_nomtu`) is unchanged.
pub fn run_pipeline(src: &str) -> Result<PipelineOutput, StageFailure> {
    let s1 = stage1_tokenize(src)?;
    let s2 = stage2_kind_classify(&s1)?;
    let s3 = stage3_shape_extract(&s2)?;
    let s4 = stage4_contract_bind(&s3)?;
    let s5 = stage5_effect_bind(&s4)?;
    stage6_ref_resolve(&s5)
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

    // S2-S6 all wired through A4c steps 1-5. a4b05 stub test dropped.

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

    // ── A4c-step2: S3 shape_extract (intent phrase) ───────────────────────

    /// a4c06: a concept with `intended to …` yields one ShapedBlock
    /// whose `intent` field carries the prose.
    #[test]
    fn a4c06_concept_intent_extracted() {
        let src = r#"the concept auth_system is
  intended to authenticate users via jwt.
  uses the @Function matching "token verify" with at-least 0.85 confidence.
  favor correctness."#;
        let stream = stage1_tokenize(src).expect("S1");
        let classified = stage2_kind_classify(&stream).expect("S2");
        let shaped = stage3_shape_extract(&classified).expect("S3");
        assert_eq!(shaped.blocks.len(), 1);
        let b = &shaped.blocks[0];
        assert_eq!(b.kind, "concept");
        assert_eq!(b.name, "auth_system");
        assert!(
            b.intent.contains("authenticate users via jwt"),
            "intent should contain the authored prose; got {:?}",
            b.intent
        );
    }

    /// a4c07: two-block `.nomtu` source — neither uses `intended to`
    /// in the entity form — the shape_extract must reject the first
    /// block with `missing-intent` (entities don't carry intent yet;
    /// S3 in this increment is strict about every block needing one,
    /// matching the concept-style authoring idiom). This test locks
    /// the strict policy; later A4c work may relax it for entities
    /// once a separate signature-shape field carries their meaning.
    #[test]
    fn a4c07_entity_without_intent_rejected() {
        let src = r#"the function fetch_url is given a url, returns text.
  benefit cache_hit."#;
        let stream = stage1_tokenize(src).expect("S1");
        let classified = stage2_kind_classify(&stream).expect("S2");
        let err = stage3_shape_extract(&classified).expect_err("S3 must reject");
        assert_eq!(err.stage, StageId::ShapeExtract);
        assert_eq!(err.reason, "missing-intent");
    }

    // ── A4c-step3: S4 contract_bind (requires / ensures) ───────────────────

    /// a4c09: a concept with `requires` + `ensures` yields a
    /// ContractedBlock with two clauses in source order.
    #[test]
    fn a4c09_requires_ensures_extracted() {
        let src = r#"the concept auth_system is
  intended to authenticate users.
  requires the jwt signature is valid.
  ensures the token owner identity is established.
  favor correctness."#;
        let stream = stage1_tokenize(src).expect("S1");
        let classified = stage2_kind_classify(&stream).expect("S2");
        let shaped = stage3_shape_extract(&classified).expect("S3");
        let contracted = stage4_contract_bind(&shaped).expect("S4");
        assert_eq!(contracted.blocks.len(), 1);
        let b = &contracted.blocks[0];
        assert_eq!(b.contracts.len(), 2);
        match &b.contracts[0] {
            ContractClause::Requires(s) => assert!(s.contains("jwt signature is valid")),
            _ => panic!("expected Requires first"),
        }
        match &b.contracts[1] {
            ContractClause::Ensures(s) => assert!(s.contains("token owner identity")),
            _ => panic!("expected Ensures second"),
        }
    }

    /// a4c10: block with zero contract clauses yields an empty vec.
    #[test]
    fn a4c10_no_contracts_yields_empty_vec() {
        let src = r#"the concept simple is
  intended to do one thing.
  favor correctness."#;
        let stream = stage1_tokenize(src).expect("S1");
        let classified = stage2_kind_classify(&stream).expect("S2");
        let shaped = stage3_shape_extract(&classified).expect("S3");
        let contracted = stage4_contract_bind(&shaped).expect("S4");
        assert_eq!(contracted.blocks[0].contracts.len(), 0);
    }

    /// a4c11: unterminated contract clause (missing `.`) rejects with
    /// NOMX-S4-unterminated-contract.
    #[test]
    fn a4c11_unterminated_contract_rejected() {
        // Block ends at the next top-level `the`, but our `requires`
        // line here has no dot before the concept body ends. Since
        // S2's end_tok heuristic relies on seeing a dot first, we
        // construct a minimal case: no trailing dot at all.
        let src = r#"the concept broken is
  intended to surface the failure.
  requires something important
  favor correctness."#;
        let stream = stage1_tokenize(src).expect("S1");
        let classified = stage2_kind_classify(&stream).expect("S2");
        let shaped = stage3_shape_extract(&classified).expect("S3");
        let err = stage4_contract_bind(&shaped).expect_err("S4 must reject");
        assert_eq!(err.stage, StageId::ContractBind);
        assert_eq!(err.reason, "unterminated-contract");
    }

    // ── A4c-step4: S5 effect_bind (benefit / hazard) ──────────────────────

    /// a4c13: block with `benefit cache_hit, fast_path.` and
    /// `hazard timeout.` yields two EffectClauses with correct
    /// valence + names.
    #[test]
    fn a4c13_benefit_and_hazard_extracted() {
        let src = r#"the concept caching_layer is
  intended to cache upstream responses.
  benefit cache_hit, fast_path.
  hazard timeout.
  favor performance."#;
        let stream = stage1_tokenize(src).expect("S1");
        let classified = stage2_kind_classify(&stream).expect("S2");
        let shaped = stage3_shape_extract(&classified).expect("S3");
        let contracted = stage4_contract_bind(&shaped).expect("S4");
        let effected = stage5_effect_bind(&contracted).expect("S5");
        assert_eq!(effected.blocks.len(), 1);
        let b = &effected.blocks[0];
        assert_eq!(b.effects.len(), 2);
        assert_eq!(b.effects[0].valence, EffectValence::Benefit);
        assert_eq!(b.effects[0].effects, vec!["cache_hit", "fast_path"]);
        assert_eq!(b.effects[1].valence, EffectValence::Hazard);
        assert_eq!(b.effects[1].effects, vec!["timeout"]);
    }

    /// a4c14: block with zero effect clauses yields an empty vec.
    #[test]
    fn a4c14_no_effects_yields_empty_vec() {
        let src = r#"the concept simple is
  intended to do one thing.
  favor correctness."#;
        let stream = stage1_tokenize(src).expect("S1");
        let classified = stage2_kind_classify(&stream).expect("S2");
        let shaped = stage3_shape_extract(&classified).expect("S3");
        let contracted = stage4_contract_bind(&shaped).expect("S4");
        let effected = stage5_effect_bind(&contracted).expect("S5");
        assert!(effected.blocks[0].effects.is_empty());
    }

    /// a4c15: `boon` synonym maps to Benefit + `bane` maps to Hazard,
    /// per lexer synonym table.
    #[test]
    fn a4c15_boon_and_bane_synonyms_map_correctly() {
        let src = r#"the concept synonymy is
  intended to exercise the synonym table.
  boon warmup.
  bane cold_start.
  favor performance."#;
        let stream = stage1_tokenize(src).expect("S1");
        let classified = stage2_kind_classify(&stream).expect("S2");
        let shaped = stage3_shape_extract(&classified).expect("S3");
        let contracted = stage4_contract_bind(&shaped).expect("S4");
        let effected = stage5_effect_bind(&contracted).expect("S5");
        let b = &effected.blocks[0];
        assert_eq!(b.effects.len(), 2);
        assert_eq!(b.effects[0].valence, EffectValence::Benefit); // boon
        assert_eq!(b.effects[0].effects, vec!["warmup"]);
        assert_eq!(b.effects[1].valence, EffectValence::Hazard); // bane
        assert_eq!(b.effects[1].effects, vec!["cold_start"]);
    }

    /// a4c16: unterminated effect (missing `.`, hits favor) rejects.
    #[test]
    fn a4c16_unterminated_effect_rejected() {
        let src = r#"the concept broken is
  intended to surface the failure.
  benefit warmup, fast_path
  favor correctness."#;
        let stream = stage1_tokenize(src).expect("S1");
        let classified = stage2_kind_classify(&stream).expect("S2");
        let shaped = stage3_shape_extract(&classified).expect("S3");
        let contracted = stage4_contract_bind(&shaped).expect("S4");
        let err = stage5_effect_bind(&contracted).expect_err("S5 must reject");
        assert_eq!(err.stage, StageId::EffectBind);
        assert_eq!(err.reason, "unterminated-effect");
    }

    // ── A4c-step5: S6 ref_resolve + run_pipeline ──────────────────────────

    /// a4c17: end-to-end pipeline on a `.nom` concept source yields
    /// PipelineOutput::Nom with the right concept name + intent +
    /// acceptance (requires/ensures rendered).
    #[test]
    fn a4c17_pipeline_end_to_end_concept() {
        let src = r#"the concept pipeline_demo is
  intended to exercise the full annotator pipeline.
  requires the input is valid.
  ensures the output is usable.
  favor correctness."#;
        let out = run_pipeline(src).expect("pipeline must succeed");
        match out {
            PipelineOutput::Nom(f) => {
                assert_eq!(f.concepts.len(), 1);
                let c = &f.concepts[0];
                assert_eq!(c.name, "pipeline_demo");
                assert!(c.intent.contains("exercise"));
                assert_eq!(c.acceptance.len(), 2);
                assert!(c.acceptance.iter().any(|a| a.starts_with("requires")));
                assert!(c.acceptance.iter().any(|a| a.starts_with("ensures")));
                assert!(c.index.is_empty(), "index is deferred in skeletal S6");
            }
            PipelineOutput::Nomtu(_) => panic!("expected Nom output"),
        }
    }

    /// a4c18: end-to-end pipeline on a `.nomtu` entity source yields
    /// PipelineOutput::Nomtu with an Entity carrying contracts +
    /// effects.
    #[test]
    fn a4c18_pipeline_end_to_end_entity() {
        let src = r#"the function write_cache is
  intended to write an entry into the shared cache.
  requires the key is ascii.
  benefit cache_warmup, fast_path."#;
        let out = run_pipeline(src).expect("pipeline must succeed");
        match out {
            PipelineOutput::Nomtu(f) => {
                assert_eq!(f.items.len(), 1);
                match &f.items[0] {
                    NomtuItem::Entity(e) => {
                        assert_eq!(e.kind, "function");
                        assert_eq!(e.word, "write_cache");
                        assert_eq!(e.contracts.len(), 1);
                        assert_eq!(e.effects.len(), 1);
                        assert_eq!(e.effects[0].valence, EffectValence::Benefit);
                    }
                    _ => panic!("expected Entity, got Composition"),
                }
            }
            PipelineOutput::Nom(_) => panic!("expected Nomtu output"),
        }
    }

    /// a4c19: mixing a concept with a non-concept block in one file
    /// rejects with NOMX-S6-mixed-concept-and-entity.
    #[test]
    fn a4c19_mixed_kinds_in_one_file_rejected() {
        let src = r#"the concept c_part is
  intended to be a concept.
  favor correctness.

the function f_part is
  intended to be an entity.
  favor correctness."#;
        let err = run_pipeline(src).expect_err("pipeline must reject");
        assert_eq!(err.stage, StageId::RefResolve);
        assert_eq!(err.reason, "mixed-concept-and-entity");
    }

    /// a4c12: two concepts each keep their own contract scope — no
    /// cross-block leakage.
    #[test]
    fn a4c12_contracts_scoped_per_block() {
        let src = r#"the concept c_one is
  intended to do thing one.
  requires input is valid.
  favor correctness.

the concept c_two is
  intended to do thing two.
  ensures the result is usable.
  favor correctness."#;
        let stream = stage1_tokenize(src).expect("S1");
        let classified = stage2_kind_classify(&stream).expect("S2");
        let shaped = stage3_shape_extract(&classified).expect("S3");
        let contracted = stage4_contract_bind(&shaped).expect("S4");
        assert_eq!(contracted.blocks.len(), 2);
        // c_one has exactly one Requires, no Ensures.
        assert_eq!(contracted.blocks[0].contracts.len(), 1);
        assert!(matches!(&contracted.blocks[0].contracts[0], ContractClause::Requires(_)));
        // c_two has exactly one Ensures, no Requires.
        assert_eq!(contracted.blocks[1].contracts.len(), 1);
        assert!(matches!(&contracted.blocks[1].contracts[0], ContractClause::Ensures(_)));
    }

    /// a4c08: two concepts, each with its own intent, yield two
    /// ShapedBlocks with correctly-scoped intent slots.
    #[test]
    fn a4c08_two_concepts_intents_scoped_per_block() {
        let src = r#"the concept c_one is
  intended to do the first thing.
  favor correctness.

the concept c_two is
  intended to do the second thing.
  favor correctness."#;
        let stream = stage1_tokenize(src).expect("S1");
        let classified = stage2_kind_classify(&stream).expect("S2");
        let shaped = stage3_shape_extract(&classified).expect("S3");
        assert_eq!(shaped.blocks.len(), 2);
        assert!(shaped.blocks[0].intent.contains("first thing"));
        assert!(shaped.blocks[1].intent.contains("second thing"));
        assert!(!shaped.blocks[0].intent.contains("second thing"));
        assert!(!shaped.blocks[1].intent.contains("first thing"));
    }
}
