#![deny(unsafe_code)]
use crate::progress::{ComposeEvent, ProgressSink};
use crate::store::ArtifactStore;
use nom_blocks::compose::audio_block::AudioBlock;
use nom_blocks::NomtuRef;
use std::fmt;

/// Output container format for audio composition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AudioContainer {
    /// WAV (RIFF PCM). Default — no external encoder needed.
    #[default]
    Wav,
    /// FLAC stub — writes a header marker; external flac encoder required.
    FlacStub,
    /// Ogg stub — writes a header marker; external libogg/libopus required.
    OggStub,
}

impl fmt::Display for AudioContainer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AudioContainer::Wav => write!(f, "audio/wav"),
            AudioContainer::FlacStub => write!(f, "audio/flac"),
            AudioContainer::OggStub => write!(f, "audio/ogg"),
        }
    }
}

/// Audio codec for composition output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AudioCodec {
    /// Uncompressed signed-16-bit PCM. Default.
    #[default]
    Pcm,
    /// FLAC stub — lossless; external encoder required.
    FlacStub,
    /// Opus stub — lossy; external libopus required.
    OpusStub,
}

impl fmt::Display for AudioCodec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AudioCodec::Pcm => write!(f, "pcm_s16le"),
            AudioCodec::FlacStub => write!(f, "flac"),
            AudioCodec::OpusStub => write!(f, "opus"),
        }
    }
}

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
    /// Container format for the output artifact. Defaults to `Wav`.
    pub container: AudioContainer,
    /// Codec used to encode samples. Defaults to `Pcm`.
    pub audio_codec: AudioCodec,
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

        let payload = match input.container {
            AudioContainer::Wav => encode_wav_mono_f32le(&input.pcm_samples, spec.sample_rate),
            AudioContainer::FlacStub => {
                encode_audio_stub_container("FLAC", &input.audio_codec.to_string(), &spec)
            }
            AudioContainer::OggStub => {
                encode_audio_stub_container("Ogg", &input.audio_codec.to_string(), &spec)
            }
        };

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

/// Produce a stub container payload with a machine-readable header marker.
/// The marker indicates that an external encoder is needed to produce a real file.
fn encode_audio_stub_container(container: &str, codec: &str, spec: &AudioSpec) -> Vec<u8> {
    format!(
        "# NOM-STUB-AUDIO-CONTAINER: {} codec={} sample_rate={} channels={} duration_ms={}\n\
         # External encoder required to produce a real {} file.\n",
        container,
        codec,
        spec.sample_rate,
        spec.channels,
        spec.duration_ms,
        container,
    )
    .into_bytes()
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

    fn default_audio_input(id: &str, word: &str, samples: Vec<f32>, sample_rate: u32, codec: &str) -> AudioInput {
        AudioInput {
            entity: NomtuRef { id: id.into(), word: word.into(), kind: "media".into() },
            pcm_samples: samples,
            sample_rate,
            codec: codec.into(),
            container: AudioContainer::default(),
            audio_codec: AudioCodec::default(),
        }
    }

    // --- existing tests (backward-compat) ---

    #[test]
    fn audio_spec_bitrate_kbps() {
        let spec = AudioSpec { sample_rate: 44100, channels: 2, duration_ms: 3000, codec: "pcm_f32le".into() };
        assert_eq!(spec.bitrate_kbps(), 1411);
        let mono = AudioSpec { sample_rate: 44100, channels: 1, duration_ms: 1000, codec: "pcm_f32le".into() };
        assert_eq!(mono.bitrate_kbps(), 705);
    }

    #[test]
    fn audio_compose_basic() {
        let mut store = InMemoryStore::new();
        let samples: Vec<f32> = (0..44100).map(|i| (i as f32 / 44100.0).sin()).collect();
        let input = default_audio_input("aud1", "tone", samples, 44100, "pcm_f32le");
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
        let input = default_audio_input("aud2", "jingle", vec![0.0f32; 8000], 8000, "opus");
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(block.entity.id, "aud2");
        assert_eq!(block.entity.word, "jingle");
    }

    #[test]
    fn audio_compose_duration_ms_correct() {
        let mut store = InMemoryStore::new();
        let input = default_audio_input("aud3", "beep", vec![0.5f32; 22050], 22050, "mp3");
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(block.duration_ms, 1000);
    }

    #[test]
    fn audio_spec_mono_bitrate() {
        let spec = AudioSpec { sample_rate: 48000, channels: 1, duration_ms: 1000, codec: "aac".into() };
        assert_eq!(spec.bitrate_kbps(), 768);
    }

    #[test]
    fn audio_wav_encoder_clamps_samples() {
        let wav = encode_wav_mono_f32le(&[-2.0, 0.0, 2.0], 8000);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[36..40], b"data");
        assert_eq!(u32::from_le_bytes(wav[40..44].try_into().unwrap()), 6);
    }

    // --- new codec/container tests ---

    #[test]
    fn audio_container_default_is_wav() {
        assert_eq!(AudioContainer::default(), AudioContainer::Wav);
    }

    #[test]
    fn audio_codec_default_is_pcm() {
        assert_eq!(AudioCodec::default(), AudioCodec::Pcm);
    }

    #[test]
    fn audio_container_display_mime_types() {
        assert_eq!(AudioContainer::Wav.to_string(), "audio/wav");
        assert_eq!(AudioContainer::FlacStub.to_string(), "audio/flac");
        assert_eq!(AudioContainer::OggStub.to_string(), "audio/ogg");
    }

    #[test]
    fn audio_codec_display_names() {
        assert_eq!(AudioCodec::Pcm.to_string(), "pcm_s16le");
        assert_eq!(AudioCodec::FlacStub.to_string(), "flac");
        assert_eq!(AudioCodec::OpusStub.to_string(), "opus");
    }

    #[test]
    fn flac_stub_produces_header_marker() {
        let mut store = InMemoryStore::new();
        let input = AudioInput {
            entity: NomtuRef { id: "aud4".into(), word: "lossless".into(), kind: "media".into() },
            pcm_samples: vec![0.0f32; 8000],
            sample_rate: 44100,
            codec: "flac".into(),
            container: AudioContainer::FlacStub,
            audio_codec: AudioCodec::FlacStub,
        };
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        let payload = store.read(&block.artifact_hash).unwrap();
        let text = String::from_utf8_lossy(&payload);
        assert!(text.contains("NOM-STUB-AUDIO-CONTAINER: FLAC"));
        assert!(text.contains("codec=flac"));
        assert!(text.contains("External encoder required"));
    }

    #[test]
    fn ogg_stub_produces_header_marker() {
        let mut store = InMemoryStore::new();
        let input = AudioInput {
            entity: NomtuRef { id: "aud5".into(), word: "stream".into(), kind: "media".into() },
            pcm_samples: vec![0.0f32; 8000],
            sample_rate: 48000,
            codec: "opus".into(),
            container: AudioContainer::OggStub,
            audio_codec: AudioCodec::OpusStub,
        };
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        let payload = store.read(&block.artifact_hash).unwrap();
        let text = String::from_utf8_lossy(&payload);
        assert!(text.contains("NOM-STUB-AUDIO-CONTAINER: Ogg"));
        assert!(text.contains("codec=opus"));
    }

    #[test]
    fn audio_stub_sample_rate_in_header() {
        let spec = AudioSpec { sample_rate: 48000, channels: 1, duration_ms: 2000, codec: "flac".into() };
        let bytes = encode_audio_stub_container("FLAC", "flac", &spec);
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("sample_rate=48000"));
        assert!(text.contains("duration_ms=2000"));
    }

    #[test]
    fn wav_default_backward_compat_round_trip() {
        // Default AudioInput uses Wav + Pcm — output must start with RIFF.
        let mut store = InMemoryStore::new();
        let input = AudioInput {
            entity: NomtuRef { id: "aud6".into(), word: "ping".into(), kind: "media".into() },
            pcm_samples: vec![0.1f32, -0.1f32],
            sample_rate: 8000,
            codec: "pcm".into(),
            container: AudioContainer::Wav,
            audio_codec: AudioCodec::Pcm,
        };
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        let payload = store.read(&block.artifact_hash).unwrap();
        assert_eq!(&payload[0..4], b"RIFF");
    }

    #[test]
    fn audio_new_fields_do_not_break_entity_propagation() {
        let mut store = InMemoryStore::new();
        let input = AudioInput {
            entity: NomtuRef { id: "aud7".into(), word: "theme".into(), kind: "media".into() },
            pcm_samples: vec![0.0f32; 1000],
            sample_rate: 44100,
            codec: "opus".into(),
            container: AudioContainer::OggStub,
            audio_codec: AudioCodec::OpusStub,
        };
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(block.entity.id, "aud7");
        assert_eq!(block.entity.word, "theme");
    }
}
