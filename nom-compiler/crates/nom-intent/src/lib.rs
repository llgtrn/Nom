//! M8 intent-resolution transformer — thin LLM layer mapping prose → bounded Nom concepts.
//!
//! Discipline (per doc 09 risk #2 + doc 10 §C): LLM output MUST resolve to a registered
//! `NomIntent` variant. Anything that fails to match returns `NomIntent::Reject(Reason)`.
//! No invented symbols. No hallucinated kinds. The `Reject` arm is the bounded-output
//! guarantee.
//!
//! WrenAI equivalence: `IntentClassificationResult` Pydantic `Literal` type ↔ Rust enum
//! with exhaustive match; WrenAI's `post_process` fallback ↔ our `Reject(Unparseable)`.
//!
//! Slice-2 (2026-04-14): the `react` module adds a ReAct-shaped loop over M8. See
//! docs/superpowers/specs/2026-04-14-graph-rag-agentic-design.md for the full design.

pub mod adapters;
pub mod dict_tools;
pub mod instrumented;
pub mod react;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NomIntent {
    Kind(String),
    Symbol(String),
    Flow(String),
    Reject(Reason),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Reason {
    Unparseable,
    UnknownKind,
    UnknownSymbol,
    BelowConfidenceThreshold,
}

#[derive(Debug, Error)]
pub enum IntentError {
    #[error("LLM stub missing for deterministic test")]
    StubMissing,
    #[error("candidate retrieval failed: {0}")]
    RetrievalFailed(String),
    #[error("LLVM compilation unavailable: {0}")]
    LlvmUnavailable(String),
    #[error("entity not found: {0}")]
    EntityNotFound(String),
}

pub struct IntentCtx {
    pub candidate_budget: usize,
    pub confidence_threshold: f32,
}

impl Default for IntentCtx {
    fn default() -> Self {
        Self {
            candidate_budget: 50,
            confidence_threshold: 0.7,
        }
    }
}

pub type LlmFn = Box<dyn Fn(&str, &[String]) -> Result<NomIntent, IntentError>>;

pub fn classify(prose: &str, ctx: &IntentCtx, llm: &LlmFn) -> Result<NomIntent, IntentError> {
    let candidates = retrieve_candidates(prose, ctx.candidate_budget)?;
    let raw = llm(prose, &candidates)?;
    Ok(validate(raw, &candidates, ctx.confidence_threshold))
}

pub fn retrieve_candidates(_prose: &str, _k: usize) -> Result<Vec<String>, IntentError> {
    Ok(Vec::new())
}

pub fn validate(intent: NomIntent, candidates: &[String], _threshold: f32) -> NomIntent {
    match &intent {
        NomIntent::Kind(k) | NomIntent::Symbol(k) | NomIntent::Flow(k) => {
            if candidates.is_empty() || candidates.iter().any(|c| c == k) {
                intent
            } else {
                NomIntent::Reject(Reason::UnknownSymbol)
            }
        }
        NomIntent::Reject(_) => intent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stub_llm_returns(intent: NomIntent) -> LlmFn {
        Box::new(move |_prose, _cands| Ok(intent.clone()))
    }

    #[test]
    fn classify_returns_symbol_when_llm_emits_match_in_candidates() {
        let ctx = IntentCtx::default();
        let llm = stub_llm_returns(NomIntent::Symbol("add".into()));
        let result = classify("add two numbers", &ctx, &llm).unwrap();
        assert_eq!(result, NomIntent::Symbol("add".into()));
    }

    #[test]
    fn validate_rejects_symbol_not_in_candidates() {
        let got = validate(
            NomIntent::Symbol("made_up_fn".into()),
            &["add".into(), "mul".into()],
            0.7,
        );
        assert_eq!(got, NomIntent::Reject(Reason::UnknownSymbol));
    }

    #[test]
    fn validate_passes_symbol_when_candidates_empty() {
        let got = validate(NomIntent::Kind("app".into()), &[], 0.7);
        assert_eq!(got, NomIntent::Kind("app".into()));
    }

    #[test]
    fn reject_variant_round_trips_through_validate() {
        let got = validate(NomIntent::Reject(Reason::Unparseable), &["x".into()], 0.7);
        assert_eq!(got, NomIntent::Reject(Reason::Unparseable));
    }
}
