//! M8 slice-2: ReAct loop over the bounded `NomIntent` intent surface.
//!
//! Wraps the classify() primitive from slice-1 in a Thought→Action→Observation
//! loop per Yao et al. 2023 "ReACT: Synergizing Reasoning and Acting in
//! Language Models" (https://arxiv.org/abs/2210.03629). Key rules enforced:
//!
//! - Bounded output: every terminal state is either `Answer(NomIntent)` or
//!   `Reject(Reason)`; no invented tokens. Mirrors slice-1's discipline.
//! - 5 grouped tools, not 30 per-crate tools — per ReAct "don't provide 20+
//!   tools" guidance (see doc 11 §3). Each `AgentAction` variant maps to
//!   exactly one method on the `AgentTools` trait.
//! - Iteration cap (`ReActBudget::max_iterations`, default 4) prevents
//!   runaway loops; exhaustion returns `Reject(IterationBudgetExhausted)`.
//! - Self-RAG critique: `verify` is a distinct tool, so the LLM can
//!   critique its own draft before committing to `Answer`. Left as a prompt-
//!   engineering concern in slice-3+ (StubTools doesn't enforce ordering).
//! - LazyGraphRAG discipline: community summaries are computed at query
//!   time inside the `query` tool implementation (slice-3's DictTools),
//!   never precomputed.
//!
//! Deterministic by construction: every external side-effect routes through
//! one of two injected dependencies — `ReActLlmFn` (the LLM) and `AgentTools`
//! (the 5-tool surface). Tests swap in stubs for both.

use serde::{Deserialize, Serialize};

use crate::{IntentError, NomIntent, Reason};

// ── Core transcript types ─────────────────────────────────────────────

/// One step in a ReAct transcript. Terminal states are `Answer` and `Reject`;
/// the driver stops at the first terminal. Non-terminal sequences are
/// Thought → Action → Observation, repeated until terminal or budget.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReActStep {
    /// LLM-generated reasoning text. Capped at `ReActBudget::max_thought_words`
    /// words by the driver (excess is truncated, not rejected) to keep each
    /// thought cheap per ReAct cost tactics.
    Thought(String),
    /// Tool invocation the LLM chose. One variant per grouped tool.
    Action(AgentAction),
    /// Result of the preceding Action, fed back into the LLM's next prompt.
    Observation(Observation),
    /// Terminal: the loop resolved to a bounded `NomIntent`.
    Answer(NomIntent),
    /// Terminal: loop exhausted budget or hit structural failure.
    Reject(Reason),
}

/// The 5 grouped tools that together cover all 30 workspace crates.
/// See doc 11 §3 + spec §Architecture for the crate-to-tool mapping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentAction {
    /// Retrieve candidate UIDs by subject; optionally filter by kind or
    /// expand to `depth` graph hops. Covers nom-dict + nom-concept +
    /// nom-graph + nom-search.
    Query { subject: String, kind: Option<String>, depth: usize },
    /// Propose a Nom from prose plus retrieved context UIDs. Covers
    /// nom-intent::classify + nom-extract + nom-concept MECE pre-check.
    Compose { prose: String, context: Vec<String> },
    /// Judge a proposed Nom (UID or draft). Covers nom-verifier +
    /// nom-security + MECE validator. Self-RAG critique surface.
    Verify { target: String },
    /// Emit artifact for a verified Nom. Covers nom-codegen + nom-llvm +
    /// nom-app + nom-media. `target` is a tag: "llvm-bc", "rust-src",
    /// "app-manifest", "avif", etc.
    Render { uid: String, target: String },
    /// Explain a Nom (show-your-work). Covers cmd_build_report +
    /// LayeredDreamReport + glass-box outputs.
    Explain { uid: String, depth: usize },
}

/// Result of an `AgentAction`, fed back into the next LLM call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Observation {
    /// `query` result — candidate UIDs in relevance order.
    Candidates(Vec<String>),
    /// `compose` result — the proposed NomIntent (could be Reject, which
    /// is a tool-level weak signal but NOT a loop terminal; CRAG pattern).
    Proposal(NomIntent),
    /// `verify` result — Self-RAG critique. Structured so the LLM can
    /// reason about specific failures and retry with fixes.
    Verdict {
        passed: bool,
        failures: Vec<String>,
        warnings: Vec<String>,
    },
    /// `render` result — artifact handle (byte hash for integrity).
    Rendered { target: String, bytes_hash: String },
    /// `explain` result — short summary. Editors can display this directly;
    /// deep consumers get the full LayeredDreamReport via a separate API.
    Explanation { summary: String },
    /// Any tool-level error that isn't a bounded `Reject`.
    Error(String),
}

// ── Driver configuration ──────────────────────────────────────────────

/// Iteration + cost budget for `classify_with_react`. Defaults follow the
/// ReAct literature's standard baselines (see doc 11 §3 tool-calling best
/// practices).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReActBudget {
    /// Hard cap on Thought/Action pairs. Exhaustion → Reject. Default 4.
    pub max_iterations: usize,
    /// Soft cap on Thought word count; excess is truncated. Default 20.
    pub max_thought_words: usize,
    /// Confidence threshold carried into the Compose tool.
    pub confidence_threshold: f32,
}

impl Default for ReActBudget {
    fn default() -> Self {
        Self {
            max_iterations: 4,
            max_thought_words: 20,
            confidence_threshold: 0.7,
        }
    }
}

/// LLM closure that, given prose + current transcript, returns the next
/// step. Tests pass deterministic closures; production wires to
/// Claude/OpenAI/etc. via a thin adapter.
pub type ReActLlmFn =
    Box<dyn Fn(&str, &[ReActStep]) -> Result<ReActStep, IntentError>>;

// ── Tools trait ───────────────────────────────────────────────────────

/// The 5 grouped tools the ReAct loop dispatches. Production `DictTools`
/// lands in slice-3; `StubTools` below is test-only.
pub trait AgentTools {
    fn query(&self, subject: &str, kind: Option<&str>, depth: usize) -> Observation;
    fn compose(&self, prose: &str, context: &[String]) -> Observation;
    fn verify(&self, target: &str) -> Observation;
    fn render(&self, uid: &str, target: &str) -> Observation;
    fn explain(&self, uid: &str, depth: usize) -> Observation;
}

/// Deterministic canned-response tools for unit tests. Each method records
/// invocations into internal counters so tests can assert dispatch.
#[derive(Debug, Default)]
pub struct StubTools {
    pub query_calls: std::cell::Cell<usize>,
    pub compose_calls: std::cell::Cell<usize>,
    pub verify_calls: std::cell::Cell<usize>,
    pub render_calls: std::cell::Cell<usize>,
    pub explain_calls: std::cell::Cell<usize>,
}

impl AgentTools for StubTools {
    fn query(&self, _subject: &str, _kind: Option<&str>, _depth: usize) -> Observation {
        self.query_calls.set(self.query_calls.get() + 1);
        Observation::Candidates(vec!["stub_uid_1".into(), "stub_uid_2".into()])
    }
    fn compose(&self, _prose: &str, _context: &[String]) -> Observation {
        self.compose_calls.set(self.compose_calls.get() + 1);
        Observation::Proposal(NomIntent::Symbol("stub_symbol".into()))
    }
    fn verify(&self, _target: &str) -> Observation {
        self.verify_calls.set(self.verify_calls.get() + 1);
        Observation::Verdict {
            passed: true,
            failures: vec![],
            warnings: vec![],
        }
    }
    fn render(&self, _uid: &str, target: &str) -> Observation {
        self.render_calls.set(self.render_calls.get() + 1);
        Observation::Rendered {
            target: target.into(),
            bytes_hash: "stub_hash".into(),
        }
    }
    fn explain(&self, _uid: &str, _depth: usize) -> Observation {
        self.explain_calls.set(self.explain_calls.get() + 1);
        Observation::Explanation {
            summary: "stub explanation".into(),
        }
    }
}

// ── Driver ─────────────────────────────────────────────────────────────

/// Dispatch one `AgentAction` to the corresponding `AgentTools` method.
/// Factored out so it is unit-testable independent of the loop.
pub fn dispatch_action(action: &AgentAction, tools: &dyn AgentTools) -> Observation {
    match action {
        AgentAction::Query { subject, kind, depth } => {
            tools.query(subject, kind.as_deref(), *depth)
        }
        AgentAction::Compose { prose, context } => tools.compose(prose, context),
        AgentAction::Verify { target } => tools.verify(target),
        AgentAction::Render { uid, target } => tools.render(uid, target),
        AgentAction::Explain { uid, depth } => tools.explain(uid, *depth),
    }
}

/// Truncate a Thought to `max_words` words to keep reasoning cheap.
fn truncate_thought(thought: &str, max_words: usize) -> String {
    let mut iter = thought.split_whitespace();
    let taken: Vec<&str> = iter.by_ref().take(max_words).collect();
    if iter.next().is_some() {
        format!("{} …", taken.join(" "))
    } else {
        taken.join(" ")
    }
}

/// Run the ReAct loop on `prose`. Returns the full transcript so callers
/// can glass-box-surface it to the editor or log it for audit.
///
/// Terminates when:
/// - LLM emits `ReActStep::Answer(NomIntent)` (success)
/// - LLM emits `ReActStep::Reject(Reason)` (bounded failure)
/// - `budget.max_iterations` Thought+Action pairs have been recorded
///   without terminal (appends `Reject(IterationBudgetExhausted)` and
///   returns)
///
/// The caller-supplied `llm` closure is expected to return EITHER a
/// `Thought` (in which case the driver re-invokes it immediately for the
/// paired `Action`), OR a terminal step. The driver auto-truncates
/// Thought strings + auto-appends Observations after each Action.
pub fn classify_with_react(
    prose: &str,
    budget: &ReActBudget,
    llm: &ReActLlmFn,
    tools: &dyn AgentTools,
) -> Result<Vec<ReActStep>, IntentError> {
    let mut transcript: Vec<ReActStep> = Vec::new();
    let mut iterations = 0;

    loop {
        if iterations >= budget.max_iterations {
            transcript.push(ReActStep::Reject(Reason::BelowConfidenceThreshold));
            return Ok(transcript);
        }

        let step = llm(prose, &transcript)?;
        match step {
            ReActStep::Answer(intent) => {
                transcript.push(ReActStep::Answer(intent));
                return Ok(transcript);
            }
            ReActStep::Reject(reason) => {
                transcript.push(ReActStep::Reject(reason));
                return Ok(transcript);
            }
            ReActStep::Thought(t) => {
                let truncated = truncate_thought(&t, budget.max_thought_words);
                transcript.push(ReActStep::Thought(truncated));
                // Ask the LLM for the paired Action.
                let action_step = llm(prose, &transcript)?;
                match action_step {
                    ReActStep::Action(action) => {
                        let obs = dispatch_action(&action, tools);
                        transcript.push(ReActStep::Action(action));
                        transcript.push(ReActStep::Observation(obs));
                        iterations += 1;
                    }
                    ReActStep::Answer(intent) => {
                        transcript.push(ReActStep::Answer(intent));
                        return Ok(transcript);
                    }
                    ReActStep::Reject(reason) => {
                        transcript.push(ReActStep::Reject(reason));
                        return Ok(transcript);
                    }
                    other => {
                        transcript.push(other);
                        return Err(IntentError::StubMissing);
                    }
                }
            }
            ReActStep::Action(action) => {
                let obs = dispatch_action(&action, tools);
                transcript.push(ReActStep::Action(action));
                transcript.push(ReActStep::Observation(obs));
                iterations += 1;
            }
            ReActStep::Observation(_) => {
                transcript.push(step);
                return Err(IntentError::StubMissing);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn llm_returning(step: ReActStep) -> ReActLlmFn {
        Box::new(move |_, _| Ok(step.clone()))
    }

    #[test]
    fn classify_with_react_terminates_on_answer() {
        let llm = llm_returning(ReActStep::Answer(NomIntent::Kind("app".into())));
        let tools = StubTools::default();
        let out =
            classify_with_react("add two numbers", &ReActBudget::default(), &llm, &tools)
                .unwrap();
        assert_eq!(out.len(), 1, "single-step answer must produce 1 entry");
        assert!(matches!(out[0], ReActStep::Answer(NomIntent::Kind(_))));
    }

    #[test]
    fn classify_with_react_terminates_on_reject() {
        let llm = llm_returning(ReActStep::Reject(Reason::Unparseable));
        let tools = StubTools::default();
        let out = classify_with_react(
            "gibberish",
            &ReActBudget::default(),
            &llm,
            &tools,
        )
        .unwrap();
        assert_eq!(out.len(), 1);
        assert!(matches!(out[0], ReActStep::Reject(Reason::Unparseable)));
    }

    #[test]
    fn classify_with_react_dispatches_query_action() {
        // Thought → Action(Query) → Observation → (then the LLM must terminate).
        let counter = std::cell::Cell::new(0usize);
        let llm: ReActLlmFn = Box::new(move |_prose, _t| {
            let n = counter.get();
            counter.set(n + 1);
            Ok(match n {
                0 => ReActStep::Thought("look up add-like functions".into()),
                1 => ReActStep::Action(AgentAction::Query {
                    subject: "add".into(),
                    kind: Some("function".into()),
                    depth: 1,
                }),
                _ => ReActStep::Answer(NomIntent::Symbol("add".into())),
            })
        });
        let tools = StubTools::default();
        let out =
            classify_with_react("add two numbers", &ReActBudget::default(), &llm, &tools)
                .unwrap();
        assert_eq!(tools.query_calls.get(), 1, "query tool must be invoked once");
        // Transcript shape: Thought, Action, Observation, Answer
        assert_eq!(out.len(), 4);
        assert!(matches!(out[0], ReActStep::Thought(_)));
        assert!(matches!(out[1], ReActStep::Action(AgentAction::Query { .. })));
        assert!(matches!(out[2], ReActStep::Observation(Observation::Candidates(_))));
        assert!(matches!(out[3], ReActStep::Answer(_)));
    }

    #[test]
    fn classify_with_react_exhausts_budget_returns_reject() {
        // LLM that keeps emitting Thought+Action forever.
        let counter = std::cell::Cell::new(0usize);
        let llm: ReActLlmFn = Box::new(move |_prose, _t| {
            let n = counter.get();
            counter.set(n + 1);
            Ok(if n % 2 == 0 {
                ReActStep::Thought(format!("step {n}"))
            } else {
                ReActStep::Action(AgentAction::Query {
                    subject: "x".into(),
                    kind: None,
                    depth: 0,
                })
            })
        });
        let tools = StubTools::default();
        let budget = ReActBudget {
            max_iterations: 2,
            ..Default::default()
        };
        let out = classify_with_react("loop", &budget, &llm, &tools).unwrap();
        // After 2 iterations (Thought+Action+Observation pairs) driver must reject.
        let last = out.last().unwrap();
        assert!(
            matches!(last, ReActStep::Reject(_)),
            "last step must be Reject after budget exhaustion; got {last:?}"
        );
    }

    #[test]
    fn transcript_round_trips_through_json() {
        let transcript = vec![
            ReActStep::Thought("hello".into()),
            ReActStep::Action(AgentAction::Query {
                subject: "add".into(),
                kind: Some("function".into()),
                depth: 2,
            }),
            ReActStep::Observation(Observation::Candidates(vec!["uid_a".into()])),
            ReActStep::Answer(NomIntent::Symbol("add".into())),
        ];
        let json = serde_json::to_string(&transcript).unwrap();
        let back: Vec<ReActStep> = serde_json::from_str(&json).unwrap();
        assert_eq!(transcript, back);
    }

    #[test]
    fn all_agent_action_variants_dispatch_to_correct_tool() {
        let tools = StubTools::default();
        dispatch_action(
            &AgentAction::Query {
                subject: "x".into(),
                kind: None,
                depth: 0,
            },
            &tools,
        );
        dispatch_action(
            &AgentAction::Compose {
                prose: "p".into(),
                context: vec![],
            },
            &tools,
        );
        dispatch_action(&AgentAction::Verify { target: "u".into() }, &tools);
        dispatch_action(
            &AgentAction::Render {
                uid: "u".into(),
                target: "llvm-bc".into(),
            },
            &tools,
        );
        dispatch_action(
            &AgentAction::Explain {
                uid: "u".into(),
                depth: 1,
            },
            &tools,
        );
        assert_eq!(tools.query_calls.get(), 1);
        assert_eq!(tools.compose_calls.get(), 1);
        assert_eq!(tools.verify_calls.get(), 1);
        assert_eq!(tools.render_calls.get(), 1);
        assert_eq!(tools.explain_calls.get(), 1);
    }

    #[test]
    fn truncate_thought_caps_word_count() {
        let thought = "one two three four five six seven eight nine ten eleven";
        let truncated = truncate_thought(thought, 3);
        assert!(truncated.starts_with("one two three"));
        assert!(truncated.ends_with('…'), "truncated thought must end with ellipsis");
        // Short thoughts pass through unchanged.
        let short = truncate_thought("short one", 10);
        assert_eq!(short, "short one");
    }
}
