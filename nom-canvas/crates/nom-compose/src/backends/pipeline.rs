#![deny(unsafe_code)]
use crate::progress::{ComposeEvent, ProgressSink};
use crate::store::ArtifactStore;
use nom_blocks::NomtuRef;

pub struct PipelineInput {
    pub entity: NomtuRef,
    pub stage_inputs: Vec<[u8; 32]>,
}

pub struct PipelineOutput {
    pub artifact_hash: [u8; 32],
    pub stages_run: usize,
}

pub struct PipelineBackend;

impl PipelineBackend {
    pub fn compose(
        input: PipelineInput,
        store: &mut dyn ArtifactStore,
        sink: &dyn ProgressSink,
    ) -> PipelineOutput {
        sink.emit(ComposeEvent::Started {
            backend: "pipeline".into(),
            entity_id: input.entity.id.clone(),
        });
        let total = input.stage_inputs.len();
        if total == 0 {
            let artifact_hash = store.write(b"");
            sink.emit(ComposeEvent::Completed {
                artifact_hash,
                byte_size: 0,
            });
            return PipelineOutput {
                artifact_hash,
                stages_run: 0,
            };
        }
        // Chain stages: concatenate each stage's data with the next stage's data
        let mut accumulated: Vec<u8> = store.read(&input.stage_inputs[0]).unwrap_or_default();
        sink.emit(ComposeEvent::Progress {
            percent: 1.0 / total as f32,
            stage: "stage_0".into(),
            rendered_frames: None,
            encoded_frames: None,
            elapsed_ms: None,
        });
        for (i, stage_hash) in input.stage_inputs.iter().enumerate().skip(1) {
            let stage_data = store.read(stage_hash).unwrap_or_default();
            accumulated.extend_from_slice(&stage_data);
            sink.emit(ComposeEvent::Progress {
                percent: (i + 1) as f32 / total as f32,
                stage: format!("stage_{}", i),
                rendered_frames: None,
                encoded_frames: None,
                elapsed_ms: None,
            });
        }
        let artifact_hash = store.write(&accumulated);
        let byte_size = accumulated.len() as u64;
        sink.emit(ComposeEvent::Completed {
            artifact_hash,
            byte_size,
        });
        PipelineOutput {
            artifact_hash,
            stages_run: total,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::LogProgressSink;
    use crate::store::InMemoryStore;

    #[test]
    fn pipeline_chains_stages() {
        let mut store = InMemoryStore::new();
        let h1 = store.write(b"foo");
        let h2 = store.write(b"bar");
        let out = PipelineBackend::compose(
            PipelineInput {
                entity: NomtuRef {
                    id: "p1".into(),
                    word: "pipe".into(),
                    kind: "concept".into(),
                },
                stage_inputs: vec![h1, h2],
            },
            &mut store,
            &LogProgressSink,
        );
        assert_eq!(out.stages_run, 2);
        let result = store.read(&out.artifact_hash).unwrap();
        assert_eq!(result, b"foobar");
    }

    #[test]
    fn pipeline_empty_stages() {
        let mut store = InMemoryStore::new();
        let out = PipelineBackend::compose(
            PipelineInput {
                entity: NomtuRef {
                    id: "p2".into(),
                    word: "pipe".into(),
                    kind: "concept".into(),
                },
                stage_inputs: vec![],
            },
            &mut store,
            &LogProgressSink,
        );
        assert_eq!(out.stages_run, 0);
        assert!(store.exists(&out.artifact_hash));
    }
}
