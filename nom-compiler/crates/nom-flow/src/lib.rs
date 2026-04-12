//! `nom-flow` — flow concretization per §5.14.
//!
//! Every execution of a nomtu can be recorded as a `FlowArtifact`:
//! the ordered sequence of function calls, with per-step timing and
//! input/output hashes. Flows are first-class dict entries; they can
//! be rendered (DOT, Mermaid, interactive HTML), diffed, or used as
//! property-test traces.
//!
//! `FlowStep` rows live in the typed `flow_steps` side-table, keyed by
//! `(artifact_id, step_index)`. Middleware hooks run between steps
//! (borrowed from DeerFlow's pattern per §5.14.4) — each middleware is
//! itself a nomtu, so flows-with-middleware are still hash-addressable.
//!
//! This crate is the Phase-5 §5.14 scaffold. Actual recording
//! instrumentation (trace via LLVM call-site hooks, render via graph
//! visualization) arrives incrementally.

use thiserror::Error;

/// One step in a flow. Becomes one `flow_steps` row.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FlowStep {
    /// Hash of the `FlowArtifact` this step belongs to.
    pub artifact_id: String,
    /// Monotonically increasing step index within the artifact.
    pub step_index: u32,
    /// Hash of the entry that was invoked at this step.
    pub entry_id: String,
    /// Start time, wall-clock nanoseconds since flow-artifact start.
    pub start_ns: u64,
    /// End time, wall-clock nanoseconds since flow-artifact start.
    pub end_ns: u64,
    /// Hash of the step's input data (content-addressed).
    /// `None` when the step takes no input.
    pub input_hash: Option<String>,
    /// Hash of the step's output data (content-addressed).
    /// `None` when the step produces no output (pure side-effect).
    pub output_hash: Option<String>,
    /// Hashes of middleware nomtu that wrapped this step.
    /// In-order: outermost first.
    pub middleware_chain: Vec<String>,
}

impl FlowStep {
    pub fn duration_ns(&self) -> u64 {
        self.end_ns.saturating_sub(self.start_ns)
    }
}

/// Top-level flow artifact header. The full flow is this header +
/// N `FlowStep` rows joined by `artifact_id`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FlowArtifact {
    pub artifact_id: String,
    /// Hash of the root entry whose execution this flow records.
    pub root_entry_id: String,
    /// Total number of steps recorded.
    pub step_count: u32,
    /// Total wall-clock duration of the root invocation.
    pub total_duration_ns: u64,
    /// Unix seconds when the artifact was recorded.
    pub recorded_at_unix_s: i64,
    /// Optional label (e.g. workload name, test id).
    pub label: Option<String>,
}

/// Errors produced by `nom-flow`.
#[derive(Debug, Error)]
pub enum FlowError {
    #[error("recorder not yet wired (LLVM call-site instrumentation pending)")]
    RecorderNotYetImplemented,
    #[error("renderer not yet implemented for format: {0}")]
    RendererNotYetImplemented(String),
    #[error("flow artifact has no steps: {0}")]
    EmptyFlow(String),
    #[error("step index out of order at {got}, expected {expected}")]
    StepIndexOutOfOrder { got: u32, expected: u32 },
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flow_step_duration_handles_underflow() {
        let s = FlowStep {
            artifact_id: "a".into(),
            step_index: 0,
            entry_id: "e".into(),
            start_ns: 100,
            end_ns: 50, // bogus: end < start, must saturate to 0.
            input_hash: None,
            output_hash: None,
            middleware_chain: vec![],
        };
        assert_eq!(s.duration_ns(), 0);
    }

    #[test]
    fn flow_step_happy_path_duration() {
        let s = FlowStep {
            artifact_id: "a".into(),
            step_index: 1,
            entry_id: "e".into(),
            start_ns: 1_000,
            end_ns: 3_500,
            input_hash: Some("in".into()),
            output_hash: Some("out".into()),
            middleware_chain: vec!["mw1".into(), "mw2".into()],
        };
        assert_eq!(s.duration_ns(), 2_500);
    }

    #[test]
    fn flow_artifact_round_trips_through_json() {
        let a = FlowArtifact {
            artifact_id: "art_abc".into(),
            root_entry_id: "root_xyz".into(),
            step_count: 3,
            total_duration_ns: 10_000,
            recorded_at_unix_s: 1_700_000_000,
            label: Some("decode_avif_bench".into()),
        };
        let s = serde_json::to_string(&a).unwrap();
        let back: FlowArtifact = serde_json::from_str(&s).unwrap();
        assert_eq!(a, back);
    }
}
