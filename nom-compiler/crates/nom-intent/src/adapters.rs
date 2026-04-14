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

use crate::react::{AgentAction, Observation, ReActAdapter, ReActStep};
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
    fn next_step(&self, prose: &str, transcript: &[ReActStep]) -> Result<ReActStep, IntentError> {
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
                    Some(ReActStep::Observation(Observation::Proposal(intent)))
                        if !matches!(intent, NomIntent::Reject(_)) =>
                    {
                        let uid =
                            symbol_or_kind_value(intent).unwrap_or_else(|| "unknown".to_string());
                        Ok(ReActStep::Action(AgentAction::Verify { target: uid }))
                    }
                    // Default: initial Query.
                    _ => {
                        let subject =
                            first_content_token(prose).unwrap_or_else(|| prose.to_string());
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
                Observation::Candidates(c) if c.is_empty() => {
                    Ok(ReActStep::Reject(Reason::UnknownSymbol))
                }
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
                Observation::Verdict { passed: true, .. } => match last_proposal(transcript) {
                    Some(intent) => Ok(ReActStep::Answer(intent)),
                    None => Ok(ReActStep::Reject(Reason::BelowConfidenceThreshold)),
                },
                // Verdict failed → reject (CRAG signal).
                Observation::Verdict { passed: false, .. } => {
                    Ok(ReActStep::Reject(Reason::UnknownSymbol))
                }
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
            ReActStep::Action(_) => Ok(ReActStep::Reject(Reason::BelowConfidenceThreshold)),
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
        "a", "an", "the", "to", "of", "for", "and", "or", "is", "are", "this", "that", "with",
        "add", "make", "do", "get",
    ];
    prose
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_lowercase())
        .find(|t| !STOP.contains(&t.as_str()))
        .or_else(|| {
            // Fallback: if every token is a stop word, take the first
            // one anyway (better than no query).
            prose.split_whitespace().next().map(|s| s.to_lowercase())
        })
}

fn symbol_or_kind_value(intent: &NomIntent) -> Option<String> {
    match intent {
        NomIntent::Kind(s) | NomIntent::Symbol(s) | NomIntent::Flow(s) => Some(s.clone()),
        NomIntent::Reject(_) => None,
    }
}

// ── McpAdapter: stdio JSON-RPC line-delimited adapter ────────────────

/// Adapter that delegates `next_step` to an external MCP-style process
/// over line-delimited JSON-RPC 2.0. Generic over `Read + Write` so
/// tests inject in-memory pipes and production wires stdin/stdout of a
/// spawned child.
///
/// Protocol (simplified MCP 2024-11-05 shape):
///
/// Request  `{"jsonrpc":"2.0","id":N,"method":"react/next_step","params":{prose,transcript}}\n`
/// Response `{"jsonrpc":"2.0","id":N,"result":<ReActStep-json>}\n`
/// Error    `{"jsonrpc":"2.0","id":N,"error":{"code":C,"message":"..."}}\n`
///
/// Each `next_step` call increments `id`; responses must match the
/// request id. Mis-matched ids raise `IntentError::StubMissing` for now
/// (slice-5b-mcp-hardening will add correlation retry / timeout).
pub struct McpAdapter<R, W> {
    reader: std::cell::RefCell<std::io::BufReader<R>>,
    writer: std::cell::RefCell<W>,
    next_id: std::cell::Cell<u64>,
}

impl<R: std::io::Read, W: std::io::Write> McpAdapter<R, W> {
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader: std::cell::RefCell::new(std::io::BufReader::new(reader)),
            writer: std::cell::RefCell::new(writer),
            next_id: std::cell::Cell::new(1),
        }
    }
}

impl<R: std::io::Read, W: std::io::Write> ReActAdapter for McpAdapter<R, W> {
    fn next_step(&self, prose: &str, transcript: &[ReActStep]) -> Result<ReActStep, IntentError> {
        use std::io::BufRead;

        let id = self.next_id.get();
        self.next_id.set(id + 1);

        // Serialize request.
        let params = serde_json::json!({
            "prose": prose,
            "transcript": transcript,
        });
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "react/next_step",
            "params": params,
        });
        let req_line = serde_json::to_string(&req).map_err(|e| {
            IntentError::RetrievalFailed(format!("McpAdapter: serialize request: {e}"))
        })?;

        {
            let mut writer = self.writer.borrow_mut();
            writer.write_all(req_line.as_bytes()).map_err(|e| {
                IntentError::RetrievalFailed(format!("McpAdapter: write request: {e}"))
            })?;
            writer.write_all(b"\n").map_err(|e| {
                IntentError::RetrievalFailed(format!("McpAdapter: write newline: {e}"))
            })?;
            writer
                .flush()
                .map_err(|e| IntentError::RetrievalFailed(format!("McpAdapter: flush: {e}")))?;
        }

        // Read response.
        let mut line = String::new();
        {
            let mut reader = self.reader.borrow_mut();
            let n = reader.read_line(&mut line).map_err(|e| {
                IntentError::RetrievalFailed(format!("McpAdapter: read response: {e}"))
            })?;
            if n == 0 {
                return Err(IntentError::RetrievalFailed(
                    "McpAdapter: EOF before response".into(),
                ));
            }
        }

        // Parse response envelope.
        let resp: serde_json::Value = serde_json::from_str(line.trim()).map_err(|e| {
            IntentError::RetrievalFailed(format!(
                "McpAdapter: parse response json: {e}; line={line:?}"
            ))
        })?;

        // Verify id matches.
        let resp_id = resp.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
        if resp_id != id {
            return Err(IntentError::RetrievalFailed(format!(
                "McpAdapter: response id {resp_id} != request id {id}"
            )));
        }

        // Handle error branch.
        if let Some(err) = resp.get("error") {
            let msg = err
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("<no message>");
            return Err(IntentError::RetrievalFailed(format!(
                "McpAdapter: remote error: {msg}"
            )));
        }

        // Parse result as ReActStep.
        let result_value = resp.get("result").ok_or_else(|| {
            IntentError::RetrievalFailed(
                "McpAdapter: response missing both `result` and `error`".into(),
            )
        })?;
        let step: ReActStep = serde_json::from_value(result_value.clone()).map_err(|e| {
            IntentError::RetrievalFailed(format!("McpAdapter: parse ReActStep: {e}"))
        })?;
        Ok(step)
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
        let step = adapter.next_step("add two numbers", &[]).unwrap();
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
            ReActStep::Observation(Observation::Candidates(vec!["uid1".into(), "uid2".into()])),
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
            ReActStep::Action(AgentAction::Verify {
                target: "add".into(),
            }),
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
        assert_eq!(first_content_token("add the").as_deref(), Some("add"));
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

    // ── McpAdapter tests ────────────────────────────────────────────

    fn mcp_with_response(response_json: &str) -> McpAdapter<std::io::Cursor<Vec<u8>>, Vec<u8>> {
        let reader = std::io::Cursor::new(format!("{response_json}\n").into_bytes());
        let writer = Vec::new();
        McpAdapter::new(reader, writer)
    }

    #[test]
    fn mcp_adapter_parses_answer_response() {
        let resp = r#"{"jsonrpc":"2.0","id":1,"result":{"Answer":{"Kind":"app"}}}"#;
        let adapter = mcp_with_response(resp);
        let step = adapter.next_step("any prose", &[]).unwrap();
        assert!(matches!(step, ReActStep::Answer(NomIntent::Kind(_))));
    }

    #[test]
    fn mcp_adapter_parses_reject_response() {
        let resp = r#"{"jsonrpc":"2.0","id":1,"result":{"Reject":"Unparseable"}}"#;
        let adapter = mcp_with_response(resp);
        let step = adapter.next_step("bad", &[]).unwrap();
        assert!(matches!(step, ReActStep::Reject(Reason::Unparseable)));
    }

    #[test]
    fn mcp_adapter_reports_remote_error() {
        let resp =
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32603,"message":"LLM unavailable"}}"#;
        let adapter = mcp_with_response(resp);
        let err = adapter.next_step("x", &[]).expect_err("must fail");
        match err {
            IntentError::RetrievalFailed(msg) => {
                assert!(
                    msg.contains("LLM unavailable"),
                    "error should contain remote message, got {msg}"
                );
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn mcp_adapter_rejects_id_mismatch() {
        // Response id=99, adapter expects id=1 on first call.
        let resp = r#"{"jsonrpc":"2.0","id":99,"result":{"Answer":{"Kind":"x"}}}"#;
        let adapter = mcp_with_response(resp);
        let err = adapter.next_step("prose", &[]).expect_err("must fail");
        match err {
            IntentError::RetrievalFailed(msg) => {
                assert!(msg.contains("response id 99"));
                assert!(msg.contains("request id 1"));
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn mcp_adapter_writes_proper_jsonrpc_request() {
        let resp = r#"{"jsonrpc":"2.0","id":1,"result":{"Reject":"Unparseable"}}"#;
        let reader = std::io::Cursor::new(format!("{resp}\n").into_bytes());
        let mut captured_writer = Vec::new();
        {
            let adapter = McpAdapter::new(reader, &mut captured_writer);
            let _ = adapter.next_step("hello", &[]);
        }
        let sent = String::from_utf8(captured_writer).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(sent.trim()).unwrap();
        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["id"], 1);
        assert_eq!(parsed["method"], "react/next_step");
        assert_eq!(parsed["params"]["prose"], "hello");
        assert!(parsed["params"]["transcript"].is_array());
    }

    #[test]
    fn mcp_adapter_errors_on_eof() {
        let reader = std::io::Cursor::new(Vec::<u8>::new()); // immediate EOF
        let writer = Vec::new();
        let adapter = McpAdapter::new(reader, writer);
        let err = adapter.next_step("x", &[]).expect_err("must fail");
        match err {
            IntentError::RetrievalFailed(msg) => assert!(msg.contains("EOF")),
            other => panic!("wrong variant: {other:?}"),
        }
    }
}
