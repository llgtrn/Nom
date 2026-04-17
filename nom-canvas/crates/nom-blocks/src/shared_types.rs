#![deny(unsafe_code)]
use serde::{Deserialize, Serialize};

/// One step in the deep_think() ReAct loop
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeepThinkStep {
    pub hypothesis: String,
    pub evidence: Vec<String>,
    pub confidence: f32,
    pub counterevidence: Vec<String>,
    pub refined_from: Option<String>,
}

/// Streaming event from nom-intent::deep_think()
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DeepThinkEvent {
    Step(DeepThinkStep),
    Final(CompositionPlan),
}

/// Stub re-export — real definition lives in nom-compiler/nom-planner (Wave C)
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CompositionPlan {
    pub intent: String,
    pub steps: Vec<PlanStep>,
    pub confidence: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: String,
    pub description: String,
    pub kind: String,
    pub depends_on: Vec<String>,
}

/// 14-variant run event enum (Rowboat pattern, all 14 exact variants)
/// Used by both nom-compiler-bridge and nom-panels right dock
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RunEvent {
    LLMStream {
        delta: String,
        model: String,
    },
    ToolInvocation {
        tool_name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_name: String,
        output: serde_json::Value,
        duration_ms: u64,
    },
    PermissionRequest {
        tool_name: String,
        reason: String,
    },
    AskHuman {
        question: String,
        placeholder: Option<String>,
    },
    SpawnSubFlow {
        agent_name: String,
        intent: String,
    },
    TextMessage {
        content: String,
        role: String,
    },
    Status {
        message: String,
        progress: Option<f32>,
    },
    ThinkingStream {
        delta: String,
    },
    DeepThinkStep(DeepThinkStep),
    ComposeProgress {
        target: String,
        stage: String,
        percent: f32,
    },
    Error {
        message: String,
        code: Option<String>,
    },
    RunCompleted {
        summary: String,
        artifact_hashes: Vec<[u8; 32]>,
    },
    Interrupt {
        reason: String,
    },
}

impl RunEvent {
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
        if let RunEvent::RunCompleted { artifact_hashes, .. } = &ev {
            assert_eq!(artifact_hashes.len(), 1);
            assert_eq!(artifact_hashes[0], [0xFFu8; 32]);
        } else {
            panic!("expected RunCompleted");
        }
    }
}
