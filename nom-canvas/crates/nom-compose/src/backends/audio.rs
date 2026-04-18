#![deny(unsafe_code)]
use crate::progress::{ComposeEvent, ProgressSink};
use crate::store::ArtifactStore;
use nom_blocks::compose::audio_block::AudioBlock;
use nom_blocks::NomtuRef;

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
    pub fn compose(
        input: AudioInput,
        store: &mut dyn ArtifactStore,
        sink: &dyn ProgressSink,
    ) -> AudioBlock {
        sink.emit(ComposeEvent::Started {
            backend: "audio".into(),
            entity_id: input.entity.id.clone(),
        });

        let sample_rate = input.sample_rate.max(1);
        let duration_ms = ((input.pcm_samples.len() as u64) * 1000 / sample_rate as u64) as u32;

        let spec = AudioSpec {
            sample_rate,
            channels: 1,
            duration_ms,
            codec: input.codec.clone(),
        };

        sink.emit(ComposeEvent::Progress {
            percent: 0.5,
            stage: "encoding".into(),
        });

        let payload = encode_wav_mono_f32le(&input.pcm_samples, spec.sample_rate);

        let artifact_hash = store.write(&payload);
        let byte_size = store.byte_size(&artifact_hash).unwrap_or(0);

        sink.emit(ComposeEvent::Completed {
            artifact_hash,
            byte_size,
        });

        AudioBlock {
            entity: input.entity,
            artifact_hash,
            duration_ms: duration_ms as u64,
            codec: input.codec,
        }
    }
}

fn encode_wav_mono_f32le(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let data_len = samples.len() as u32 * 2;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples {
        let pcm = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        out.extend_from_slice(&pcm.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::LogProgressSink;
    use crate::store::InMemoryStore;

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
            entity: NomtuRef {
                id: "aud1".into(),
                word: "tone".into(),
                kind: "media".into(),
            },
            pcm_samples: samples,
            sample_rate: 44100,
            codec: "pcm_f32le".into(),
        };
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(block.codec, "pcm_f32le");
        assert_eq!(block.duration_ms, 1000);
        assert!(store.exists(&block.artifact_hash));
        let payload = store.read(&block.artifact_hash).unwrap();
        assert_eq!(&payload[0..4], b"RIFF");
        assert_eq!(&payload[8..12], b"WAVE");
    }

    #[test]
    fn audio_compose_entity_propagated() {
        let mut store = InMemoryStore::new();
        let input = AudioInput {
            entity: NomtuRef {
                id: "aud2".into(),
                word: "jingle".into(),
                kind: "media".into(),
            },
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
            entity: NomtuRef {
                id: "aud3".into(),
                word: "beep".into(),
                kind: "media".into(),
            },
            pcm_samples: vec![0.5f32; 22050],
            sample_rate: 22050,
            codec: "mp3".into(),
        };
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(block.duration_ms, 1000);
    }

    #[test]
    fn audio_spec_mono_bitrate() {
        let spec = AudioSpec {
            sample_rate: 48000,
            channels: 1,
            duration_ms: 1000,
            codec: "aac".into(),
        };
        // 48000 * 1 * 16 / 1000 = 768 kbps
        assert_eq!(spec.bitrate_kbps(), 768);
    }

    #[test]
    fn audio_wav_encoder_clamps_samples() {
        let wav = encode_wav_mono_f32le(&[-2.0, 0.0, 2.0], 8000);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[36..40], b"data");
        assert_eq!(u32::from_le_bytes(wav[40..44].try_into().unwrap()), 6);
    }
}
