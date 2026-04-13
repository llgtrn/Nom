//! Slice-5b-nom-cli: concrete `ReActAdapter` impls that don't need an
//! external LLM. The default `NomCliAdapter` makes the nom-compiler
//! itself the oracle: it inspects the ReAct transcript and emits the
//! next step deterministically, matching what a well-behaved external
//! LLM would produce when prompted with the same state.
//!
//! Per spec `docs/superpowers/specs/2026-04-14-graph-rag-agentic-design.md`
//! (slice-5b clarification) + memory
//! `project_react_llm_adapter_polymorphism.md`. The compiler is its own
//! oracle — no external API keys, completely offline, deterministic.
//! Doc 04 §10.3.1 fixpoint discipline preserved (LLM stays pre-build;
//! here the "LLM" is the compiler's own token-overlap + MECE).
//!
//! State machine (pure transcript inspection):
//!
//! ```text
//! []                                    → Thought("query for `<prose>`")
//! [..., Thought(_)]                     → Action(Query { subject })
//! [..., Candidates([])]                 → Reject(UnknownSymbol)
//! [..., Candidates(nonempty)]           → Thought("compose from candidates")
//! [..., Candidates, Thought]            → Action(Compose { prose, context })
//! [..., Proposal(Symbol|Kind|Flow)]     → Answer(intent)
//! [..., Proposal(Reject(_))]            → Reject (propagate)
//! [..., Verdict{passed:true}]           → Answer(last known intent)
//! [..., Verdict{passed:false}]          → Reject(UnknownSymbol) [CRAG retry hint]
//! anything else (shouldn't happen)      → Reject(IterationBudgetExhausted)
//! ```
//!
//! The adapter doesn't know about `AgentTools` — the ReAct driver wires
//! those separately. It purely turns transcript state into next step.

use crate::react::{
    AgentAction, Observation, ReActAdapter, ReActStep,
};
use crate::{IntentError, NomIntent, Reason};

/// Nom-compiler-as-oracle adapter. Deterministic state machine over
/// the ReAct transcript. Default adapter for `nom agent classify`
/// (slice-5b-cli-flag will wire the selector).
pub struct NomCliAdapter;

impl NomCliAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NomCliAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ReActAdapter for NomCliAdapter {
    fn next_step(
        &self,
        prose: &str,
        transcript: &[ReActStep],
    ) -> Result<ReActStep, IntentError> {
        let Some(last) = transcript.last() else {
            // Empty transcript: seed the loop with a Thought.
            return Ok(ReActStep::Thought(format!(
                "query for candidates matching `{}`",
                summarize_prose(prose, 40)
            )));
        };
        match last {
            ReActStep::Thought(_) => {
                // Decide which action to take based on what came BEFORE
                // the Thought. Pair-preceding Observation (if any) drives
                // the choice; otherwise it's an initial Query.
                let prev = transcript.get(transcript.len().saturating_sub(2));
                match prev {
                    // After a Candidates observation → compose.
                    Some(ReActStep::Observation(Observation::Candidates(c))) => {
                        Ok(ReActStep::Action(AgentAction::Compose {
                            prose: prose.to_string(),
                            context: c.clone(),
                        }))
                    }
                    // After a Proposal that wasn't Reject → verify.
                    Some(ReActStep::Observation(Observation::Proposal(
                        intent,
                    ))) if !matches!(intent, NomIntent::Reject(_)) => {
                        let uid = symbol_or_kind_value(intent)
                            .unwrap_or_else(|| "unknown".to_string());
                        Ok(ReActStep::Action(AgentAction::Verify { target: uid }))
                    }
                    // Default: initial Query.
                    _ => {
                        let subject = first_content_token(prose)
                            .unwrap_or_else(|| prose.to_string());
                        Ok(ReActStep::Action(AgentAction::Query {
                            subject,
                            kind: None,
                            depth: 0,
                        }))
                    }
                }
            }
            ReActStep::Observation(obs) => match obs {
                // Empty candidates → give up (CRAG signal: retrieval was weak).
                Observation::Candidates(c) if c.is_empty() => Ok(ReActStep::Reject(
                    Reason::UnknownSymbol,
                )),
                // Got candidates → think about composing next.
                Observation::Candidates(_) => Ok(ReActStep::Thought(
                    "compose a Nom from retrieved candidates".into(),
                )),
                // Proposal with an intent we can use → verify.
                Observation::Proposal(intent) => match intent {
                    NomIntent::Reject(r) => Ok(ReActStep::Reject(r.clone())),
                    _ => Ok(ReActStep::Thought(
                        "verify proposed intent before answering".into(),
                    )),
                },
                // Verdict passed → answer. Look back for the last Proposal.
                Observation::Verdict { passed: true, .. } => {
                    match last_proposal(transcript) {
                        Some(intent) => Ok(ReActStep::Answer(intent)),
                        None => Ok(ReActStep::Reject(
                            Reason::BelowConfidenceThreshold,
                        )),
                    }
                }
                // Verdict failed → reject (CRAG signal).
                Observation::Verdict { passed: false, .. } => Ok(ReActStep::Reject(
                    Reason::UnknownSymbol,
                )),
                // Other observations (Rendered, Explanation, Error) default to
                // terminating — nom-cli state machine doesn't chain them.
                _ => Ok(ReActStep::Reject(Reason::BelowConfidenceThreshold)),
            },
            // Terminal steps shouldn't be seen (ReAct driver stops on
            // Answer/Reject), but defensive branch returns same terminal.
            ReActStep::Answer(intent) => Ok(ReActStep::Answer(intent.clone())),
            ReActStep::Reject(r) => Ok(ReActStep::Reject(r.clone())),
            // Action without following observation = wait state; shouldn't
            // happen because the ReAct driver appends Observation after
            // every Action. Defensive: emit Reject to terminate.
            ReActStep::Action(_) => Ok(ReActStep::Reject(
                Reason::BelowConfidenceThreshold,
            )),
        }
    }
}

fn summarize_prose(prose: &str, max_chars: usize) -> String {
    let trimmed = prose.trim();
    if trimmed.chars().count() <= max_chars {
        trimmed.to_string()
    } else {
        let cut: String = trimmed.chars().take(max_chars).collect();
        format!("{cut}…")
    }
}

fn first_content_token(prose: &str) -> Option<String> {
    // Pick the first alphabetic-majority token as subject; skip stop
    // words that won't match dict entries cleanly. Mirrors CRAG-style
    // narrowing: pick the most discriminating word.
    const STOP: &[&str] = &[
        "a", "an", "the", "to", "of", "for", "and", "or", "is", "are",
        "this", "that", "with", "add", "make", "do", "get",
    ];
    prose
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_lowercase())
        .find(|t| !STOP.contains(&t.as_str()))
        .or_else(|| {
            // Fallback: if every token is a stop word, take the first
            // one anyway (better than no query).
            prose
                .split_whitespace()
                .next()
                .map(|s| s.to_lowercase())
        })
}

fn symbol_or_kind_value(intent: &NomIntent) -> Option<String> {
    match intent {
        NomIntent::Kind(s) | NomIntent::Symbol(s) | NomIntent::Flow(s) => {
            Some(s.clone())
        }
        NomIntent::Reject(_) => None,
    }
}

fn last_proposal(transcript: &[ReActStep]) -> Option<NomIntent> {
    transcript.iter().rev().find_map(|s| match s {
        ReActStep::Observation(Observation::Proposal(intent))
            if !matches!(intent, NomIntent::Reject(_)) =>
        {
            Some(intent.clone())
        }
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_transcript_produces_opening_thought() {
        let adapter = NomCliAdapter::new();
        let step = adapter
            .next_step("add two numbers", &[])
            .unwrap();
        match step {
            ReActStep::Thought(t) => assert!(t.contains("query")),
            other => panic!("expected Thought, got {other:?}"),
        }
    }

    #[test]
    fn after_opening_thought_emits_query_action() {
        let adapter = NomCliAdapter::new();
        let transcript = vec![ReActStep::Thought("query …".into())];
        let step = adapter.next_step("add two numbers", &transcript).unwrap();
        match step {
            ReActStep::Action(AgentAction::Query { subject, .. }) => {
                // first_content_token skips "add" (stop word) → "two"
                assert_eq!(subject, "two");
            }
            other => panic!("expected Query, got {other:?}"),
        }
    }

    #[test]
    fn empty_candidates_reject_unknown_symbol() {
        let adapter = NomCliAdapter::new();
        let transcript = vec![
            ReActStep::Thought("q".into()),
            ReActStep::Action(AgentAction::Query {
                subject: "x".into(),
                kind: None,
                depth: 0,
            }),
            ReActStep::Observation(Observation::Candidates(Vec::new())),
        ];
        let step = adapter.next_step("irrelevant", &transcript).unwrap();
        assert!(matches!(step, ReActStep::Reject(Reason::UnknownSymbol)));
    }

    #[test]
    fn nonempty_candidates_triggers_compose_thought() {
        let adapter = NomCliAdapter::new();
        let transcript = vec![
            ReActStep::Thought("q".into()),
            ReActStep::Action(AgentAction::Query {
                subject: "x".into(),
                kind: None,
                depth: 0,
            }),
            ReActStep::Observation(Observation::Candidates(vec!["uid1".into()])),
        ];
        let step = adapter.next_step("prose", &transcript).unwrap();
        match step {
            ReActStep::Thought(t) => assert!(t.contains("compose")),
            other => panic!("expected Thought, got {other:?}"),
        }
    }

    #[test]
    fn compose_thought_after_candidates_produces_compose_action() {
        let adapter = NomCliAdapter::new();
        let transcript = vec![
            ReActStep::Observation(Observation::Candidates(vec![
                "uid1".into(),
                "uid2".into(),
            ])),
            ReActStep::Thought("compose …".into()),
        ];
        let step = adapter.next_step("add two numbers", &transcript).unwrap();
        match step {
            ReActStep::Action(AgentAction::Compose { prose, context }) => {
                assert_eq!(prose, "add two numbers");
                assert_eq!(context, vec!["uid1".to_string(), "uid2".to_string()]);
            }
            other => panic!("expected Compose, got {other:?}"),
        }
    }

    #[test]
    fn proposal_with_reject_propagates() {
        let adapter = NomCliAdapter::new();
        let transcript = vec![ReActStep::Observation(Observation::Proposal(
            NomIntent::Reject(Reason::UnknownSymbol),
        ))];
        let step = adapter.next_step("p", &transcript).unwrap();
        assert!(matches!(step, ReActStep::Reject(Reason::UnknownSymbol)));
    }

    #[test]
    fn proposal_with_symbol_triggers_verify_thought_then_verify_action() {
        let adapter = NomCliAdapter::new();
        // After Proposal → adapter emits Thought about verify.
        let tr1 = vec![ReActStep::Observation(Observation::Proposal(
            NomIntent::Symbol("add".into()),
        ))];
        let step1 = adapter.next_step("p", &tr1).unwrap();
        match step1 {
            ReActStep::Thought(t) => assert!(t.contains("verify")),
            other => panic!("expected Thought, got {other:?}"),
        }
        // After that Thought → Action(Verify) targeting the proposal's word.
        let tr2 = vec![
            ReActStep::Observation(Observation::Proposal(NomIntent::Symbol("add".into()))),
            ReActStep::Thought("verify proposed intent".into()),
        ];
        let step2 = adapter.next_step("p", &tr2).unwrap();
        match step2 {
            ReActStep::Action(AgentAction::Verify { target }) => {
                assert_eq!(target, "add");
            }
            other => panic!("expected Verify, got {other:?}"),
        }
    }

    #[test]
    fn passed_verdict_after_proposal_produces_answer() {
        let adapter = NomCliAdapter::new();
        let transcript = vec![
            ReActStep::Observation(Observation::Proposal(NomIntent::Symbol("add".into()))),
            ReActStep::Action(AgentAction::Verify { target: "add".into() }),
            ReActStep::Observation(Observation::Verdict {
                passed: true,
                failures: vec![],
                warnings: vec![],
            }),
        ];
        let step = adapter.next_step("p", &transcript).unwrap();
        match step {
            ReActStep::Answer(NomIntent::Symbol(w)) => assert_eq!(w, "add"),
            other => panic!("expected Answer(Symbol), got {other:?}"),
        }
    }

    #[test]
    fn failed_verdict_triggers_reject() {
        let adapter = NomCliAdapter::new();
        let transcript = vec![ReActStep::Observation(Observation::Verdict {
            passed: false,
            failures: vec!["bad body_kind".into()],
            warnings: vec![],
        })];
        let step = adapter.next_step("p", &transcript).unwrap();
        assert!(matches!(step, ReActStep::Reject(_)));
    }

    #[test]
    fn first_content_token_skips_stop_words() {
        assert_eq!(
            first_content_token("add two numbers").as_deref(),
            Some("two")
        );
        // All stop words → fallback to first token.
        assert_eq!(
            first_content_token("add the").as_deref(),
            Some("add")
        );
        // Empty input → None.
        assert_eq!(first_content_token(""), None);
    }

    #[test]
    fn summarize_prose_truncates_with_ellipsis() {
        let long = "a".repeat(100);
        let out = summarize_prose(&long, 10);
        assert!(out.ends_with('…'));
        assert!(out.chars().count() <= 11);
        assert_eq!(summarize_prose("short", 40), "short");
    }
}
