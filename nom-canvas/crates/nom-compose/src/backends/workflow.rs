#![deny(unsafe_code)]
use crate::progress::{ComposeEvent, ProgressSink};
use crate::store::ArtifactStore;
use nom_blocks::NomtuRef;

pub struct WorkflowStep {
    pub node_id: String,
    pub kind: String,
    pub input: serde_json::Value,
}

pub struct WorkflowInput {
    pub entity: NomtuRef,
    pub steps: Vec<WorkflowStep>,
    pub initial_context: serde_json::Value,
}

pub struct WorkflowOutput {
    pub artifact_hash: [u8; 32],
    pub steps_completed: usize,
    pub final_value: serde_json::Value,
}

pub struct WorkflowBackend;

impl WorkflowBackend {
    pub fn compose(
        input: WorkflowInput,
        store: &mut dyn ArtifactStore,
        sink: &dyn ProgressSink,
    ) -> WorkflowOutput {
        sink.emit(ComposeEvent::Started {
            backend: "workflow".into(),
            entity_id: input.entity.id.clone(),
        });
        let total = input.steps.len();
        let mut context = input.initial_context.clone();
        for (i, step) in input.steps.iter().enumerate() {
            context =
                serde_json::json!({ "step": step.node_id, "input": step.input, "prev": context });
            sink.emit(ComposeEvent::Progress {
                percent: (i + 1) as f32 / total.max(1) as f32,
                stage: step.node_id.clone(),
            });
        }
        let bytes = serde_json::to_vec(&context).unwrap_or_default();
        let artifact_hash = store.write(&bytes);
        let byte_size = bytes.len() as u64;
        sink.emit(ComposeEvent::Completed {
            artifact_hash,
            byte_size,
        });
        WorkflowOutput {
            artifact_hash,
            steps_completed: total,
            final_value: context,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::LogProgressSink;
    use crate::store::InMemoryStore;

    #[test]
    fn workflow_empty_steps() {
        let mut store = InMemoryStore::new();
        let input = WorkflowInput {
            entity: NomtuRef {
                id: "wf1".into(),
                word: "pipeline".into(),
                kind: "concept".into(),
            },
            steps: vec![],
            initial_context: serde_json::json!({"start": true}),
        };
        let out = WorkflowBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(out.steps_completed, 0);
        assert!(store.exists(&out.artifact_hash));
    }

    #[test]
    fn workflow_with_steps() {
        let mut store = InMemoryStore::new();
        let input = WorkflowInput {
            entity: NomtuRef {
                id: "wf2".into(),
                word: "flow".into(),
                kind: "concept".into(),
            },
            steps: vec![
                WorkflowStep {
                    node_id: "step1".into(),
                    kind: "transform".into(),
                    input: serde_json::json!({"x": 1}),
                },
                WorkflowStep {
                    node_id: "step2".into(),
                    kind: "filter".into(),
                    input: serde_json::json!({"y": 2}),
                },
            ],
            initial_context: serde_json::json!({}),
        };
        let out = WorkflowBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(out.steps_completed, 2);
        assert!(store.exists(&out.artifact_hash));
    }

    #[test]
    fn workflow_final_value_reflects_last_step() {
        let mut store = InMemoryStore::new();
        let input = WorkflowInput {
            entity: NomtuRef {
                id: "wf3".into(),
                word: "etl".into(),
                kind: "concept".into(),
            },
            steps: vec![WorkflowStep {
                node_id: "extract".into(),
                kind: "extract".into(),
                input: serde_json::json!({"src": "db"}),
            }],
            initial_context: serde_json::json!({}),
        };
        let out = WorkflowBackend::compose(input, &mut store, &LogProgressSink);
        // The final_value JSON object must carry the last step's node_id
        assert_eq!(out.final_value["step"], "extract");
    }

    #[test]
    fn workflow_steps_completed_count() {
        let mut store = InMemoryStore::new();
        let steps: Vec<WorkflowStep> = (0..5)
            .map(|i| WorkflowStep {
                node_id: format!("step{i}"),
                kind: "transform".into(),
                input: serde_json::json!({"i": i}),
            })
            .collect();
        let input = WorkflowInput {
            entity: NomtuRef {
                id: "wf4".into(),
                word: "batch".into(),
                kind: "concept".into(),
            },
            steps,
            initial_context: serde_json::json!({}),
        };
        let out = WorkflowBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(out.steps_completed, 5);
    }
}
