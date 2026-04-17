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
}
