#![deny(unsafe_code)]
use nom_blocks::compose::audio_block::AudioBlock;
use nom_blocks::NomtuRef;
use crate::store::ArtifactStore;
use crate::progress::{ProgressSink, ComposeEvent};

pub struct AudioInput {
    pub entity: NomtuRef,
    pub pcm_samples: Vec<f32>,
    pub sample_rate: u32,
    pub codec: String,
}

pub struct AudioBackend;

impl AudioBackend {
    pub fn compose(input: AudioInput, store: &mut dyn ArtifactStore, sink: &dyn ProgressSink) -> AudioBlock {
        sink.emit(ComposeEvent::Started { backend: "audio".into(), entity_id: input.entity.id.clone() });
        // Serialize f32 samples to raw bytes (little-endian)
        let raw: Vec<u8> = input.pcm_samples.iter()
            .flat_map(|s| s.to_le_bytes())
            .collect();
        sink.emit(ComposeEvent::Progress { percent: 0.5, stage: "encoding".into() });
        let artifact_hash = store.write(&raw);
        let byte_size = store.byte_size(&artifact_hash).unwrap_or(0);
        let duration_ms = if input.sample_rate > 0 {
            (input.pcm_samples.len() as u64) * 1000 / input.sample_rate as u64
        } else {
            0
        };
        sink.emit(ComposeEvent::Completed { artifact_hash, byte_size });
        AudioBlock {
            entity: input.entity,
            artifact_hash,
            duration_ms,
            codec: input.codec,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::InMemoryStore;
    use crate::progress::LogProgressSink;
    #[test]
    fn audio_compose_basic() {
        let mut store = InMemoryStore::new();
        let samples: Vec<f32> = (0..44100).map(|i| (i as f32 / 44100.0).sin()).collect();
        let input = AudioInput {
            entity: NomtuRef { id: "aud1".into(), word: "tone".into(), kind: "media".into() },
            pcm_samples: samples,
            sample_rate: 44100,
            codec: "pcm_f32le".into(),
        };
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(block.codec, "pcm_f32le");
        assert_eq!(block.duration_ms, 1000);
        assert!(store.exists(&block.artifact_hash));
    }
}
