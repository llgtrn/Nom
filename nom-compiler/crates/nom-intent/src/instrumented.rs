//! Slice-4: `InstrumentedTools` — glass-box logging wrapper around any
//! `AgentTools` impl. Forwards every call to the inner impl and records
//! a structured log entry so callers (editor drill-through, CLI `--trace`
//! flag, glass-box report) can surface "what the agent actually did."
//!
//! Design choices:
//!
//! - **Decorator pattern**, not inheritance — takes any `dyn AgentTools`,
//!   produces a new `AgentTools` impl that logs + delegates. Keeps
//!   `DictTools` / `StubTools` implementations unaware of logging.
//! - **Interior mutability via `RefCell`** — tests use cell-borrows;
//!   production uses `Mutex` under feature-gated compile (TODO: the
//!   current single-thread discipline is documented but not yet
//!   enforced — slice-4-mt adds `Send + Sync` bounds later).
//! - **Structured log entries** with `CallKind` enum, not strings, so
//!   downstream editors can format entries without parsing.
//! - **Forward-only recording** — entries are append-only; no retroactive
//!   editing. Matches glass-box report invariant from M1 (`41aedfe`).

use std::cell::RefCell;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::react::{AgentTools, Observation};

/// One entry in an `InstrumentedTools` call log. Each entry carries the
/// method that fired + its arguments + the observation that came back,
/// timestamped with wall-clock duration spent inside the inner impl.
///
/// Serializable so editors can ingest transcripts as JSON; the enum
/// shape matches `AgentAction` from the ReAct driver so a single
/// transcript walker can handle both.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogEntry {
    pub call: CallKind,
    pub observation: Observation,
    /// Wall-clock microseconds spent inside the inner tool. Useful for
    /// latency attribution in glass-box reports; ReAct literature
    /// flagged agentic loops at 10+ seconds / 3-4 calls, so per-call
    /// attribution matters.
    pub duration_us: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CallKind {
    Query { subject: String, kind: Option<String>, depth: usize },
    Compose { prose: String, context: Vec<String> },
    Verify { target: String },
    Render { uid: String, target: String },
    Explain { uid: String, depth: usize },
}

/// Wraps any `AgentTools` impl and records every call + result.
///
/// Usage in a ReAct loop:
/// ```rust,ignore
/// let dict_tools = DictTools::new(&dict);
/// let instrumented = InstrumentedTools::new(&dict_tools);
/// let transcript = classify_with_react(prose, &budget, &llm, &instrumented)?;
/// for entry in instrumented.entries() {
///     eprintln!("tool {:?} → {:?} ({} μs)", entry.call, entry.observation, entry.duration_us);
/// }
/// ```
///
/// The wrapper owns no state beyond the log; the inner impl does all
/// the work. Thread-safety: single-thread only (uses `RefCell`);
/// mt variant ships in slice-4-mt.
pub struct InstrumentedTools<'a> {
    inner: &'a dyn AgentTools,
    log: RefCell<Vec<LogEntry>>,
}

impl<'a> InstrumentedTools<'a> {
    pub fn new(inner: &'a dyn AgentTools) -> Self {
        Self {
            inner,
            log: RefCell::new(Vec::new()),
        }
    }

    /// Snapshot of the current log in insertion order.
    pub fn entries(&self) -> Vec<LogEntry> {
        self.log.borrow().clone()
    }

    /// Number of calls recorded. Useful for assertion counts in tests.
    pub fn call_count(&self) -> usize {
        self.log.borrow().len()
    }

    /// Drain the log, returning all entries and leaving the log empty.
    /// Callers who want to segment a loop into phases (e.g. "query
    /// phase" vs "verify phase") can drain between phases.
    pub fn drain(&self) -> Vec<LogEntry> {
        std::mem::take(&mut *self.log.borrow_mut())
    }

    fn record(&self, call: CallKind, observation: Observation, duration: Duration) {
        self.log.borrow_mut().push(LogEntry {
            call,
            observation,
            duration_us: duration.as_micros() as u64,
        });
    }
}

impl<'a> AgentTools for InstrumentedTools<'a> {
    fn query(&self, subject: &str, kind: Option<&str>, depth: usize) -> Observation {
        let start = std::time::Instant::now();
        let obs = self.inner.query(subject, kind, depth);
        self.record(
            CallKind::Query {
                subject: subject.to_string(),
                kind: kind.map(|s| s.to_string()),
                depth,
            },
            obs.clone(),
            start.elapsed(),
        );
        obs
    }

    fn compose(&self, prose: &str, context: &[String]) -> Observation {
        let start = std::time::Instant::now();
        let obs = self.inner.compose(prose, context);
        self.record(
            CallKind::Compose {
                prose: prose.to_string(),
                context: context.to_vec(),
            },
            obs.clone(),
            start.elapsed(),
        );
        obs
    }

    fn verify(&self, target: &str) -> Observation {
        let start = std::time::Instant::now();
        let obs = self.inner.verify(target);
        self.record(
            CallKind::Verify { target: target.to_string() },
            obs.clone(),
            start.elapsed(),
        );
        obs
    }

    fn render(&self, uid: &str, target: &str) -> Observation {
        let start = std::time::Instant::now();
        let obs = self.inner.render(uid, target);
        self.record(
            CallKind::Render {
                uid: uid.to_string(),
                target: target.to_string(),
            },
            obs.clone(),
            start.elapsed(),
        );
        obs
    }

    fn explain(&self, uid: &str, depth: usize) -> Observation {
        let start = std::time::Instant::now();
        let obs = self.inner.explain(uid, depth);
        self.record(
            CallKind::Explain {
                uid: uid.to_string(),
                depth,
            },
            obs.clone(),
            start.elapsed(),
        );
        obs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::react::StubTools;

    #[test]
    fn instrumented_forwards_query_and_logs() {
        let stub = StubTools::default();
        let inst = InstrumentedTools::new(&stub);
        let obs = inst.query("add", Some("function"), 1);
        // Delegation: stub returns canned Candidates.
        assert!(matches!(obs, Observation::Candidates(_)));
        // Side-effect: stub's per-call counter bumped.
        assert_eq!(stub.query_calls.get(), 1);
        // Logging: exactly one entry, kind matches.
        assert_eq!(inst.call_count(), 1);
        let entry = &inst.entries()[0];
        match &entry.call {
            CallKind::Query { subject, kind, depth } => {
                assert_eq!(subject, "add");
                assert_eq!(kind.as_deref(), Some("function"));
                assert_eq!(*depth, 1);
            }
            other => panic!("expected Query call, got {other:?}"),
        }
    }

    #[test]
    fn instrumented_logs_all_five_tools_in_order() {
        let stub = StubTools::default();
        let inst = InstrumentedTools::new(&stub);
        inst.query("q", None, 0);
        inst.compose("p", &["u".to_string()]);
        inst.verify("t");
        inst.render("u", "llvm");
        inst.explain("u", 1);
        let entries = inst.entries();
        assert_eq!(entries.len(), 5);
        assert!(matches!(entries[0].call, CallKind::Query { .. }));
        assert!(matches!(entries[1].call, CallKind::Compose { .. }));
        assert!(matches!(entries[2].call, CallKind::Verify { .. }));
        assert!(matches!(entries[3].call, CallKind::Render { .. }));
        assert!(matches!(entries[4].call, CallKind::Explain { .. }));
    }

    #[test]
    fn drain_empties_the_log_and_returns_all_entries() {
        let stub = StubTools::default();
        let inst = InstrumentedTools::new(&stub);
        inst.query("a", None, 0);
        inst.query("b", None, 0);
        let drained = inst.drain();
        assert_eq!(drained.len(), 2);
        assert_eq!(inst.call_count(), 0, "drain must clear the log");
        // Fresh calls accumulate into an empty log.
        inst.query("c", None, 0);
        assert_eq!(inst.call_count(), 1);
    }

    #[test]
    fn duration_us_is_recorded_and_non_negative() {
        let stub = StubTools::default();
        let inst = InstrumentedTools::new(&stub);
        inst.verify("target");
        let entry = &inst.entries()[0];
        // Stub is O(1) so duration is small but non-negative; the field
        // is u64 so >=0 is trivially true — the real test is that the
        // field exists and is populated, which serde JSON round-trip
        // verifies below.
        let json = serde_json::to_string(entry).unwrap();
        assert!(json.contains("\"duration_us\":"));
    }

    #[test]
    fn log_entry_round_trips_through_json() {
        let stub = StubTools::default();
        let inst = InstrumentedTools::new(&stub);
        inst.render("aaaa", "llvm-native");
        let entry = &inst.entries()[0];
        let json = serde_json::to_string(entry).unwrap();
        let back: LogEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, &back);
    }

    #[test]
    fn compose_logs_context_and_prose_verbatim() {
        let stub = StubTools::default();
        let inst = InstrumentedTools::new(&stub);
        let ctx = vec!["u1".to_string(), "u2".to_string()];
        inst.compose("greet the world", &ctx);
        let entry = &inst.entries()[0];
        match &entry.call {
            CallKind::Compose { prose, context } => {
                assert_eq!(prose, "greet the world");
                assert_eq!(context, &ctx);
            }
            other => panic!("expected Compose, got {other:?}"),
        }
    }
}
