#![deny(unsafe_code)]
use nom_blocks::compose::audio_block::AudioBlock;
use nom_blocks::NomtuRef;
use crate::store::ArtifactStore;
use crate::progress::{ProgressSink, ComposeEvent};

/// Audio composition spec.
#[derive(Debug, Clone)]
pub struct AudioSpec {
    pub sample_rate: u32,
    pub channels: u8,
    pub duration_ms: u32,
    pub codec: String,
}

impl AudioSpec {
    /// Estimated bitrate in kbps: sample_rate * channels * 16-bit / 1000.
    pub fn bitrate_kbps(&self) -> u32 {
        (self.sample_rate * self.channels as u32 * 16) / 1000
    }
}

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

        let sample_rate = input.sample_rate.max(1);
        let duration_ms = ((input.pcm_samples.len() as u64) * 1000 / sample_rate as u64) as u32;

        let spec = AudioSpec {
            sample_rate,
            channels: 1,
            duration_ms,
            codec: input.codec.clone(),
        };

        // Encode f32 PCM samples to little-endian bytes.
        let raw: Vec<u8> = input.pcm_samples.iter()
            .flat_map(|s| s.to_le_bytes())
            .collect();

        sink.emit(ComposeEvent::Progress { percent: 0.5, stage: "encoding".into() });

        // Write spec metadata as JSON alongside the raw audio.
        let spec_json = serde_json::json!({
            "sample_rate": spec.sample_rate,
            "channels": spec.channels,
            "duration_ms": spec.duration_ms,
            "codec": spec.codec,
            "bitrate_kbps": spec.bitrate_kbps(),
        });
        let mut payload = spec_json.to_string().into_bytes();
        payload.push(b'\0');
        payload.extend_from_slice(&raw);

        let artifact_hash = store.write(&payload);
        let byte_size = store.byte_size(&artifact_hash).unwrap_or(0);

        sink.emit(ComposeEvent::Completed { artifact_hash, byte_size });

        AudioBlock {
            entity: input.entity,
            artifact_hash,
            duration_ms: duration_ms as u64,
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
    fn audio_spec_bitrate_kbps() {
        let spec = AudioSpec {
            sample_rate: 44100,
            channels: 2,
            duration_ms: 3000,
            codec: "pcm_f32le".into(),
        };
        // 44100 * 2 * 16 / 1000 = 1411 kbps
        assert_eq!(spec.bitrate_kbps(), 1411);

        let mono = AudioSpec {
            sample_rate: 44100,
            channels: 1,
            duration_ms: 1000,
            codec: "pcm_f32le".into(),
        };
        assert_eq!(mono.bitrate_kbps(), 705);
    }

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

    #[test]
    fn audio_compose_entity_propagated() {
        let mut store = InMemoryStore::new();
        let input = AudioInput {
            entity: NomtuRef { id: "aud2".into(), word: "jingle".into(), kind: "media".into() },
            pcm_samples: vec![0.0f32; 8000],
            sample_rate: 8000,
            codec: "opus".into(),
        };
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(block.entity.id, "aud2");
        assert_eq!(block.entity.word, "jingle");
    }

    #[test]
    fn audio_compose_duration_ms_correct() {
        let mut store = InMemoryStore::new();
        // 22050 samples at 22050 Hz = 1000 ms
        let input = AudioInput {
            entity: NomtuRef { id: "aud3".into(), word: "beep".into(), kind: "media".into() },
            pcm_samples: vec![0.5f32; 22050],
            sample_rate: 22050,
            codec: "mp3".into(),
        };
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(block.duration_ms, 1000);
    }

    #[test]
    fn audio_spec_mono_bitrate() {
        let spec = AudioSpec { sample_rate: 48000, channels: 1, duration_ms: 1000, codec: "aac".into() };
        // 48000 * 1 * 16 / 1000 = 768 kbps
        assert_eq!(spec.bitrate_kbps(), 768);
    }
}
