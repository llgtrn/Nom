//! Shared event and plan types used across the nom-canvas crate family.
#![deny(unsafe_code)]
use serde::{Deserialize, Serialize};

/// One step in the deep_think() ReAct loop
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeepThinkStep {
    /// Current hypothesis being evaluated.
    pub hypothesis: String,
    /// Supporting evidence strings.
    pub evidence: Vec<String>,
    /// Confidence score for this step (0.0–1.0).
    pub confidence: f32,
    /// Counter-evidence strings.
    pub counterevidence: Vec<String>,
    /// ID of the step this was refined from, if any.
    pub refined_from: Option<String>,
}

/// Streaming event from nom-intent::deep_think()
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DeepThinkEvent {
    /// An intermediate reasoning step.
    Step(DeepThinkStep),
    /// The final composition plan.
    Final(CompositionPlan),
}

/// Stub re-export — real definition lives in nom-compiler/nom-planner (Wave C)
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CompositionPlan {
    /// Natural-language intent that generated this plan.
    pub intent: String,
    /// Ordered plan steps.
    pub steps: Vec<PlanStep>,
    /// Overall plan confidence (0.0–1.0).
    pub confidence: f32,
}

/// One step within a [`CompositionPlan`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlanStep {
    /// Unique step identifier.
    pub id: String,
    /// Human-readable description of what this step does.
    pub description: String,
    /// Grammar kind this step targets.
    pub kind: String,
    /// IDs of steps that must complete before this one.
    pub depends_on: Vec<String>,
}

/// 14-variant run event enum (Rowboat pattern, all 14 exact variants)
/// Used by both nom-compiler-bridge and nom-panels right dock
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RunEvent {
    /// Streaming delta from an LLM.
    LLMStream {
        /// Text chunk.
        delta: String,
        /// Model identifier.
        model: String,
    },
    /// A tool is being called.
    ToolInvocation {
        /// Tool name.
        tool_name: String,
        /// Tool input JSON.
        input: serde_json::Value,
    },
    /// Result of a tool call.
    ToolResult {
        /// Tool name.
        tool_name: String,
        /// Tool output JSON.
        output: serde_json::Value,
        /// Wall-clock duration in milliseconds.
        duration_ms: u64,
    },
    /// The agent needs permission to proceed.
    PermissionRequest {
        /// Tool requesting permission.
        tool_name: String,
        /// Human-readable reason.
        reason: String,
    },
    /// The agent is asking the human a question.
    AskHuman {
        /// Question text.
        question: String,
        /// Optional input placeholder.
        placeholder: Option<String>,
    },
    /// A sub-agent is being spawned.
    SpawnSubFlow {
        /// Sub-agent name.
        agent_name: String,
        /// Intent passed to the sub-agent.
        intent: String,
    },
    /// A free-form text message.
    TextMessage {
        /// Message body.
        content: String,
        /// Sender role (e.g. `"user"`, `"assistant"`).
        role: String,
    },
    /// Progress status update.
    Status {
        /// Status message.
        message: String,
        /// Optional progress fraction (0.0–1.0).
        progress: Option<f32>,
    },
    /// Streaming thinking delta.
    ThinkingStream {
        /// Text chunk.
        delta: String,
    },
    /// An embedded deep-think reasoning step.
    DeepThinkStep(DeepThinkStep),
    /// Composition pipeline progress.
    ComposeProgress {
        /// Target artifact name.
        target: String,
        /// Current pipeline stage.
        stage: String,
        /// Completion percentage (0.0–100.0).
        percent: f32,
    },
    /// A fatal run error.
    Error {
        /// Error message.
        message: String,
        /// Optional error code.
        code: Option<String>,
    },
    /// The run completed successfully.
    RunCompleted {
        /// Summary string.
        summary: String,
        /// Content-addressed hashes of produced artifacts.
        artifact_hashes: Vec<[u8; 32]>,
    },
    /// The run was interrupted.
    Interrupt {
        /// Reason for the interruption.
        reason: String,
    },
}

impl RunEvent {
    /// Returns `true` for terminal events (`RunCompleted`, `Error`, `Interrupt`).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            RunEvent::RunCompleted { .. } | RunEvent::Error { .. } | RunEvent::Interrupt { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_event_terminal() {
        assert!(RunEvent::RunCompleted {
            summary: "done".into(),
            artifact_hashes: vec![]
        }
        .is_terminal());
        assert!(RunEvent::Error {
            message: "err".into(),
            code: None
        }
        .is_terminal());
        assert!(!RunEvent::Status {
            message: "ok".into(),
            progress: None
        }
        .is_terminal());
    }

    #[test]
    fn deep_think_step() {
        let step = DeepThinkStep {
            hypothesis: "H1".into(),
            evidence: vec!["E1".into()],
            confidence: 0.75,
            counterevidence: vec![],
            refined_from: None,
        };
        let event = DeepThinkEvent::Step(step.clone());
        assert!(matches!(event, DeepThinkEvent::Step(_)));
    }

    #[test]
    fn run_event_interrupt_is_terminal() {
        let ev = RunEvent::Interrupt {
            reason: "user cancelled".into(),
        };
        assert!(ev.is_terminal());
    }

    #[test]
    fn run_event_llm_stream_not_terminal() {
        let ev = RunEvent::LLMStream {
            delta: "hello".into(),
            model: "sonnet".into(),
        };
        assert!(!ev.is_terminal());
    }

    #[test]
    fn run_event_tool_invocation_not_terminal() {
        let ev = RunEvent::ToolInvocation {
            tool_name: "grep".into(),
            input: serde_json::json!({"pattern": "x"}),
        };
        assert!(!ev.is_terminal());
    }

    #[test]
    fn run_event_thinking_stream_not_terminal() {
        let ev = RunEvent::ThinkingStream {
            delta: "...".into(),
        };
        assert!(!ev.is_terminal());
    }

    #[test]
    fn composition_plan_default() {
        let plan = CompositionPlan::default();
        assert!(plan.intent.is_empty());
        assert!(plan.steps.is_empty());
        assert_eq!(plan.confidence, 0.0);
    }

    #[test]
    fn plan_step_depends_on() {
        let step = PlanStep {
            id: "step-2".into(),
            description: "second".into(),
            kind: "action".into(),
            depends_on: vec!["step-1".into()],
        };
        assert_eq!(step.depends_on.len(), 1);
        assert_eq!(step.depends_on[0], "step-1");
    }

    #[test]
    fn deep_think_event_final_variant() {
        let plan = CompositionPlan {
            intent: "build app".into(),
            steps: vec![],
            confidence: 0.9,
        };
        let ev = DeepThinkEvent::Final(plan);
        assert!(matches!(ev, DeepThinkEvent::Final(_)));
    }

    #[test]
    fn deep_think_step_with_refined_from() {
        let step = DeepThinkStep {
            hypothesis: "refined hypo".into(),
            evidence: vec![],
            confidence: 0.8,
            counterevidence: vec!["counter-1".into()],
            refined_from: Some("original-hypo".into()),
        };
        assert!(step.refined_from.is_some());
        assert_eq!(step.refined_from.as_deref(), Some("original-hypo"));
        assert_eq!(step.counterevidence.len(), 1);
    }

    #[test]
    fn run_event_run_completed_has_artifact_hashes() {
        let ev = RunEvent::RunCompleted {
            summary: "all done".into(),
            artifact_hashes: vec![[0xFFu8; 32]],
        };
        assert!(ev.is_terminal());
        if let RunEvent::RunCompleted {
            artifact_hashes, ..
        } = &ev
        {
            assert_eq!(artifact_hashes.len(), 1);
            assert_eq!(artifact_hashes[0], [0xFFu8; 32]);
        } else {
            panic!("expected RunCompleted");
        }
    }
}
