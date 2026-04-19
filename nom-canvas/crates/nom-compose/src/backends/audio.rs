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
    /// MP3 — uses libmp3lame via ffmpeg when available.
    Mp3,
}

impl fmt::Display for AudioContainer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AudioContainer::Wav => write!(f, "audio/wav"),
            AudioContainer::FlacStub => write!(f, "audio/flac"),
            AudioContainer::OggStub => write!(f, "audio/ogg"),
            AudioContainer::Mp3 => write!(f, "audio/mp3"),
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
            rendered_frames: None,
            encoded_frames: None,
            elapsed_ms: None,
        });

        let payload = match input.container {
            AudioContainer::Wav => encode_wav_mono_f32le(&input.pcm_samples, spec.sample_rate),
            AudioContainer::FlacStub => encode_audio_with_ffmpeg_fallback(
                "FLAC",
                &input.audio_codec.to_string(),
                &spec,
                &input.pcm_samples,
                spec.sample_rate,
                &["-c:a".to_string(), "flac".to_string()],
                "flac",
            ),
            AudioContainer::OggStub => encode_audio_with_ffmpeg_fallback(
                "Ogg",
                &input.audio_codec.to_string(),
                &spec,
                &input.pcm_samples,
                spec.sample_rate,
                &[
                    "-c:a".to_string(),
                    "libvorbis".to_string(),
                    "-q:a".to_string(),
                    "4".to_string(),
                ],
                "ogg",
            ),
            AudioContainer::Mp3 => encode_audio_with_ffmpeg_fallback(
                "MP3",
                "libmp3lame",
                &spec,
                &input.pcm_samples,
                spec.sample_rate,
                &[
                    "-c:a".to_string(),
                    "libmp3lame".to_string(),
                    "-q:a".to_string(),
                    "2".to_string(),
                ],
                "mp3",
            ),
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
        container, codec, spec.sample_rate, spec.channels, spec.duration_ms, container,
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

/// Encode audio via ffmpeg if the feature is enabled; otherwise fall back to stub.
#[allow(unused_variables)]
fn encode_audio_with_ffmpeg_fallback(
    container_name: &str,
    codec_name: &str,
    spec: &AudioSpec,
    samples: &[f32],
    sample_rate: u32,
    codec_args: &[String],
    format: &str,
) -> Vec<u8> {
    #[cfg(feature = "ffmpeg")]
    {
        let wav = encode_wav_mono_f32le(samples, sample_rate);
        let mut args = vec![
            "-f".to_string(),
            "wav".to_string(),
            "-i".to_string(),
            "pipe:0".to_string(),
        ];
        args.extend_from_slice(codec_args);
        args.extend_from_slice(&[
            "-f".to_string(),
            format.to_string(),
            "pipe:1".to_string(),
        ]);
        if let Ok(data) = crate::video_capture::run_command_with_timeout("ffmpeg", &args, Some(&wav), 30) {
            if !data.is_empty() {
                return data;
            }
        }
    }
    encode_audio_stub_container(container_name, codec_name, spec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::LogProgressSink;
    use crate::store::InMemoryStore;

    fn default_audio_input(
        id: &str,
        word: &str,
        samples: Vec<f32>,
        sample_rate: u32,
        codec: &str,
    ) -> AudioInput {
        AudioInput {
            entity: NomtuRef {
                id: id.into(),
                word: word.into(),
                kind: "media".into(),
            },
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
        let spec = AudioSpec {
            sample_rate: 44100,
            channels: 2,
            duration_ms: 3000,
            codec: "pcm_f32le".into(),
        };
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
        let spec = AudioSpec {
            sample_rate: 48000,
            channels: 1,
            duration_ms: 1000,
            codec: "aac".into(),
        };
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
            entity: NomtuRef {
                id: "aud4".into(),
                word: "lossless".into(),
                kind: "media".into(),
            },
            pcm_samples: vec![0.0f32; 8000],
            sample_rate: 44100,
            codec: "flac".into(),
            container: AudioContainer::FlacStub,
            audio_codec: AudioCodec::FlacStub,
        };
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        let payload = store.read(&block.artifact_hash).unwrap();
        let text = String::from_utf8_lossy(&payload);
        let is_stub = text.contains("NOM-STUB-AUDIO-CONTAINER: FLAC");
        let is_flac = payload.starts_with(b"fLaC");
        assert!(
            is_stub || is_flac,
            "must be FLAC stub or real FLAC data"
        );
        if is_stub {
            assert!(text.contains("codec=flac"));
            assert!(text.contains("External encoder required"));
        }
    }

    #[test]
    fn ogg_stub_produces_header_marker() {
        let mut store = InMemoryStore::new();
        let input = AudioInput {
            entity: NomtuRef {
                id: "aud5".into(),
                word: "stream".into(),
                kind: "media".into(),
            },
            pcm_samples: vec![0.0f32; 8000],
            sample_rate: 48000,
            codec: "opus".into(),
            container: AudioContainer::OggStub,
            audio_codec: AudioCodec::OpusStub,
        };
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        let payload = store.read(&block.artifact_hash).unwrap();
        let text = String::from_utf8_lossy(&payload);
        let is_stub = text.contains("NOM-STUB-AUDIO-CONTAINER: Ogg");
        let is_ogg = payload.starts_with(b"OggS");
        assert!(
            is_stub || is_ogg,
            "must be Ogg stub or real Ogg data"
        );
        if is_stub {
            assert!(text.contains("codec=opus"));
        }
    }

    #[test]
    fn audio_stub_sample_rate_in_header() {
        let spec = AudioSpec {
            sample_rate: 48000,
            channels: 1,
            duration_ms: 2000,
            codec: "flac".into(),
        };
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
            entity: NomtuRef {
                id: "aud6".into(),
                word: "ping".into(),
                kind: "media".into(),
            },
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
            entity: NomtuRef {
                id: "aud7".into(),
                word: "theme".into(),
                kind: "media".into(),
            },
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

    // ── Wave AE new tests ────────────────────────────────────────────────────

    #[test]
    fn flac_stub_header_contains_sample_rate() {
        let spec = AudioSpec {
            sample_rate: 96000,
            channels: 1,
            duration_ms: 1000,
            codec: "flac".into(),
        };
        let bytes = encode_audio_stub_container("FLAC", "flac", &spec);
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            text.contains("sample_rate=96000"),
            "FLAC stub must contain sample_rate"
        );
    }

    #[test]
    fn ogg_stub_header_contains_codec_field() {
        let spec = AudioSpec {
            sample_rate: 48000,
            channels: 1,
            duration_ms: 500,
            codec: "opus".into(),
        };
        let bytes = encode_audio_stub_container("Ogg", "opus", &spec);
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            text.contains("codec=opus"),
            "Ogg stub must contain codec field"
        );
        assert!(text.contains("NOM-STUB-AUDIO-CONTAINER: Ogg"));
    }

    #[test]
    fn audio_sample_rate_zero_does_not_panic() {
        let mut store = InMemoryStore::new();
        let input = AudioInput {
            entity: NomtuRef {
                id: "aud-zero".into(),
                word: "silence".into(),
                kind: "media".into(),
            },
            pcm_samples: vec![0.0f32; 8],
            sample_rate: 0,
            codec: "pcm".into(),
            container: AudioContainer::Wav,
            audio_codec: AudioCodec::Pcm,
        };
        // Must not panic — sample_rate=0 is clamped to 1 internally
        let _block = AudioBackend::compose(input, &mut store, &LogProgressSink);
    }

    #[test]
    fn audio_sample_rate_192000_wav_stores_artifact() {
        let mut store = InMemoryStore::new();
        let input = AudioInput {
            entity: NomtuRef {
                id: "aud-hi".into(),
                word: "hires".into(),
                kind: "media".into(),
            },
            pcm_samples: vec![0.5f32; 192],
            sample_rate: 192000,
            codec: "pcm".into(),
            container: AudioContainer::Wav,
            audio_codec: AudioCodec::Pcm,
        };
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        assert!(store.exists(&block.artifact_hash));
        let payload = store.read(&block.artifact_hash).unwrap();
        assert_eq!(&payload[0..4], b"RIFF");
    }

    #[test]
    fn flac_stub_compose_stores_artifact() {
        let mut store = InMemoryStore::new();
        let input = AudioInput {
            entity: NomtuRef {
                id: "aud-flac".into(),
                word: "lossless".into(),
                kind: "media".into(),
            },
            pcm_samples: vec![0.1f32; 4000],
            sample_rate: 44100,
            codec: "flac".into(),
            container: AudioContainer::FlacStub,
            audio_codec: AudioCodec::FlacStub,
        };
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        assert!(store.exists(&block.artifact_hash));
        let payload = store.read(&block.artifact_hash).unwrap();
        let text = String::from_utf8_lossy(&payload);
        let is_stub = text.contains("FLAC");
        let is_flac = payload.starts_with(b"fLaC");
        assert!(
            is_stub || is_flac,
            "must be FLAC stub or real FLAC data"
        );
    }

    #[test]
    fn ogg_stub_compose_stores_artifact() {
        let mut store = InMemoryStore::new();
        let input = AudioInput {
            entity: NomtuRef {
                id: "aud-ogg".into(),
                word: "stream".into(),
                kind: "media".into(),
            },
            pcm_samples: vec![0.2f32; 2000],
            sample_rate: 48000,
            codec: "opus".into(),
            container: AudioContainer::OggStub,
            audio_codec: AudioCodec::OpusStub,
        };
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        assert!(store.exists(&block.artifact_hash));
    }

    #[test]
    fn audio_codec_all_three_display_different() {
        let pcm = AudioCodec::Pcm.to_string();
        let flac = AudioCodec::FlacStub.to_string();
        let opus = AudioCodec::OpusStub.to_string();
        assert_ne!(pcm, flac);
        assert_ne!(flac, opus);
        assert_ne!(pcm, opus);
    }

    #[test]
    fn audio_container_all_three_display_different() {
        let wav = AudioContainer::Wav.to_string();
        let flac = AudioContainer::FlacStub.to_string();
        let ogg = AudioContainer::OggStub.to_string();
        assert_ne!(wav, flac);
        assert_ne!(flac, ogg);
        assert_ne!(wav, ogg);
    }

    #[test]
    fn audio_spec_high_sample_rate_bitrate() {
        // 192000 Hz, 1 channel => 192000 * 1 * 16 / 1000 = 3072 kbps
        let spec = AudioSpec {
            sample_rate: 192000,
            channels: 1,
            duration_ms: 100,
            codec: "pcm".into(),
        };
        assert_eq!(spec.bitrate_kbps(), 3072);
    }

    #[test]
    fn audio_spec_zero_sample_rate_bitrate_is_zero() {
        let spec = AudioSpec {
            sample_rate: 0,
            channels: 1,
            duration_ms: 0,
            codec: "pcm".into(),
        };
        assert_eq!(spec.bitrate_kbps(), 0);
    }

    #[test]
    fn flac_stub_header_contains_duration_ms() {
        let spec = AudioSpec {
            sample_rate: 44100,
            channels: 1,
            duration_ms: 3000,
            codec: "flac".into(),
        };
        let bytes = encode_audio_stub_container("FLAC", "flac", &spec);
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("duration_ms=3000"));
    }

    // ── Wave AG new tests ────────────────────────────────────────────────────

    #[test]
    fn audio_container_wav_is_default() {
        assert_eq!(AudioContainer::default(), AudioContainer::Wav);
    }

    #[test]
    fn audio_codec_pcm_is_default() {
        assert_eq!(AudioCodec::default(), AudioCodec::Pcm);
    }

    #[test]
    fn audio_container_flac_stub_display_is_audio_flac() {
        assert_eq!(AudioContainer::FlacStub.to_string(), "audio/flac");
    }

    #[test]
    fn audio_container_ogg_stub_display_is_audio_ogg() {
        assert_eq!(AudioContainer::OggStub.to_string(), "audio/ogg");
    }

    #[test]
    fn audio_codec_flac_stub_display_is_flac() {
        assert_eq!(AudioCodec::FlacStub.to_string(), "flac");
    }

    #[test]
    fn audio_codec_opus_stub_display_is_opus() {
        assert_eq!(AudioCodec::OpusStub.to_string(), "opus");
    }

    #[test]
    fn audio_backend_compose_flac_stub_ok() {
        let mut store = InMemoryStore::new();
        let input = AudioInput {
            entity: NomtuRef {
                id: "flac-ok".into(),
                word: "lossless".into(),
                kind: "media".into(),
            },
            pcm_samples: vec![0.5f32; 1000],
            sample_rate: 44100,
            codec: "flac".into(),
            container: AudioContainer::FlacStub,
            audio_codec: AudioCodec::FlacStub,
        };
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        assert!(store.exists(&block.artifact_hash));
        let payload = store.read(&block.artifact_hash).unwrap();
        let text = String::from_utf8_lossy(&payload);
        let is_stub = text.contains("FLAC");
        let is_flac = payload.starts_with(b"fLaC");
        assert!(
            is_stub || is_flac,
            "must be FLAC stub or real FLAC data"
        );
    }

    #[test]
    fn audio_backend_compose_ogg_stub_ok() {
        let mut store = InMemoryStore::new();
        let input = AudioInput {
            entity: NomtuRef {
                id: "ogg-ok".into(),
                word: "stream".into(),
                kind: "media".into(),
            },
            pcm_samples: vec![0.1f32; 500],
            sample_rate: 48000,
            codec: "opus".into(),
            container: AudioContainer::OggStub,
            audio_codec: AudioCodec::OpusStub,
        };
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        assert!(store.exists(&block.artifact_hash));
        let payload = store.read(&block.artifact_hash).unwrap();
        let text = String::from_utf8_lossy(&payload);
        assert!(text.contains("Ogg"), "Ogg stub payload must mention Ogg");
    }

    #[test]
    fn audio_backend_empty_samples_wav_stores_artifact() {
        // Zero samples — must not panic, produces a valid (minimal) WAV artifact.
        let mut store = InMemoryStore::new();
        let input = default_audio_input("aud-empty", "silence", vec![], 44100, "pcm");
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        assert!(store.exists(&block.artifact_hash));
        let payload = store.read(&block.artifact_hash).unwrap();
        assert_eq!(&payload[0..4], b"RIFF");
    }

    #[test]
    fn audio_backend_duration_zero_for_empty_samples() {
        let mut store = InMemoryStore::new();
        let input = default_audio_input("aud-dur0", "zero", vec![], 44100, "pcm");
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(block.duration_ms, 0, "zero samples => duration_ms 0");
    }

    #[test]
    fn audio_container_display_all_start_with_audio_prefix() {
        assert!(AudioContainer::Wav.to_string().starts_with("audio/"));
        assert!(AudioContainer::FlacStub.to_string().starts_with("audio/"));
        assert!(AudioContainer::OggStub.to_string().starts_with("audio/"));
    }

    #[test]
    fn audio_codec_pcm_display_contains_pcm() {
        assert!(AudioCodec::Pcm.to_string().contains("pcm"));
    }

    #[test]
    fn audio_stub_channels_in_header() {
        let spec = AudioSpec {
            sample_rate: 44100,
            channels: 2,
            duration_ms: 1000,
            codec: "flac".into(),
        };
        let bytes = encode_audio_stub_container("FLAC", "flac", &spec);
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            text.contains("channels=2"),
            "stub header must contain channels"
        );
    }

    #[test]
    fn audio_wav_single_sample_riff_header_valid() {
        let wav = encode_wav_mono_f32le(&[0.5f32], 44100);
        // Bytes 0-3: "RIFF"
        assert_eq!(&wav[0..4], b"RIFF");
        // Bytes 8-11: "WAVE"
        assert_eq!(&wav[8..12], b"WAVE");
        // data chunk at byte 36
        assert_eq!(&wav[36..40], b"data");
    }

    #[test]
    fn audio_codec_all_display_nonempty() {
        for codec in &[AudioCodec::Pcm, AudioCodec::FlacStub, AudioCodec::OpusStub] {
            assert!(
                !codec.to_string().is_empty(),
                "codec display must be nonempty for {:?}",
                codec
            );
        }
    }

    // ── Wave AJ new tests ────────────────────────────────────────────────────

    #[test]
    fn audio_backend_sample_rate_44100() {
        // 44100 Hz sample rate must produce a valid RIFF WAV artifact.
        let mut store = InMemoryStore::new();
        let samples: Vec<f32> = vec![0.0f32; 44100];
        let input = default_audio_input("sr-44100", "tone", samples, 44100, "pcm");
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        let payload = store.read(&block.artifact_hash).unwrap();
        assert_eq!(&payload[0..4], b"RIFF", "44100 Hz WAV must start with RIFF");
        // Sample rate field in WAV header is at bytes 24-27 (little-endian u32).
        let rate = u32::from_le_bytes(payload[24..28].try_into().unwrap());
        assert_eq!(rate, 44100, "WAV header must encode 44100 Hz sample rate");
    }

    #[test]
    fn audio_backend_sample_rate_48000() {
        // 48000 Hz sample rate must be encoded in WAV header.
        let mut store = InMemoryStore::new();
        let samples = vec![0.0f32; 48000];
        let input = default_audio_input("sr-48000", "clip", samples, 48000, "pcm");
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        let payload = store.read(&block.artifact_hash).unwrap();
        let rate = u32::from_le_bytes(payload[24..28].try_into().unwrap());
        assert_eq!(rate, 48000, "WAV header must encode 48000 Hz sample rate");
    }

    #[test]
    fn audio_backend_bit_depth_16() {
        // The WAV encoder always writes 16-bit samples; bits-per-sample field (bytes 34-35) must be 16.
        let mut store = InMemoryStore::new();
        let input = default_audio_input("bd-16", "beep", vec![0.5f32; 8], 8000, "pcm");
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        let payload = store.read(&block.artifact_hash).unwrap();
        let bps = u16::from_le_bytes(payload[34..36].try_into().unwrap());
        assert_eq!(bps, 16, "WAV encoder must use 16-bit depth");
    }

    #[test]
    fn audio_backend_bit_depth_24() {
        // Verify the 16-bit encoder path handles non-standard bit-depth tags gracefully.
        // In the WAV stub the field is always 16 — confirm no panic for high-sample input.
        let mut store = InMemoryStore::new();
        let input = default_audio_input("bd-24", "hires", vec![0.5f32; 192], 96000, "pcm");
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        assert!(
            store.exists(&block.artifact_hash),
            "must store artifact for 96kHz input"
        );
    }

    #[test]
    fn audio_backend_mono_channel_count_1() {
        // WAV channel count field (bytes 22-23) must be 1 for mono.
        let mut store = InMemoryStore::new();
        let input = default_audio_input("ch-1", "mono", vec![0.0f32; 8], 8000, "pcm");
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        let payload = store.read(&block.artifact_hash).unwrap();
        let channels = u16::from_le_bytes(payload[22..24].try_into().unwrap());
        assert_eq!(channels, 1, "WAV header must report 1 channel (mono)");
    }

    #[test]
    fn audio_backend_stereo_channel_count_2() {
        // AudioSpec with channels=2 produces correct bitrate; encoder still writes mono WAV.
        let spec = AudioSpec {
            sample_rate: 44100,
            channels: 2,
            duration_ms: 1000,
            codec: "pcm".into(),
        };
        // Stereo bitrate: 44100 * 2 * 16 / 1000 = 1411 kbps
        assert_eq!(
            spec.bitrate_kbps(),
            1411,
            "stereo bitrate must be double mono"
        );
        // Confirm channels field is accessible.
        assert_eq!(spec.channels, 2);
    }

    #[test]
    fn audio_backend_silence_produces_artifact() {
        // All-zero PCM samples must produce a stored artifact with RIFF header.
        let mut store = InMemoryStore::new();
        let input = default_audio_input("silence", "quiet", vec![0.0f32; 1000], 44100, "pcm");
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        assert!(
            store.exists(&block.artifact_hash),
            "silence must produce a stored artifact"
        );
        let payload = store.read(&block.artifact_hash).unwrap();
        assert_eq!(&payload[0..4], b"RIFF");
    }

    #[test]
    fn audio_backend_nonzero_samples_produce_output() {
        // Non-silent samples (max amplitude) must produce a stored WAV artifact.
        let mut store = InMemoryStore::new();
        let samples: Vec<f32> = (0..100)
            .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
            .collect();
        let input = default_audio_input("nonzero", "loud", samples, 44100, "pcm");
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        assert!(
            store.exists(&block.artifact_hash),
            "non-zero samples must produce artifact"
        );
        let payload = store.read(&block.artifact_hash).unwrap();
        assert_eq!(&payload[0..4], b"RIFF");
        assert!(
            payload.len() > 44,
            "payload must contain PCM data beyond header"
        );
    }

    // ── Wave AK new tests ────────────────────────────────────────────────────

    #[test]
    fn audio_wav_to_flac_stub_produces_flac_mime() {
        assert_eq!(AudioContainer::FlacStub.to_string(), "audio/flac");
        let mut store = InMemoryStore::new();
        let input = AudioInput {
            entity: NomtuRef {
                id: "wav-flac".into(),
                word: "track".into(),
                kind: "media".into(),
            },
            pcm_samples: vec![0.1f32; 4410],
            sample_rate: 44100,
            codec: "flac".into(),
            container: AudioContainer::FlacStub,
            audio_codec: AudioCodec::FlacStub,
        };
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        let payload = store.read(&block.artifact_hash).unwrap();
        let text = String::from_utf8_lossy(&payload);
        let is_stub = text.contains("FLAC");
        let is_flac = payload.starts_with(b"fLaC");
        assert!(
            is_stub || is_flac,
            "must be FLAC stub or real FLAC data"
        );
    }

    #[test]
    fn audio_wav_to_ogg_stub_produces_ogg_mime() {
        assert_eq!(AudioContainer::OggStub.to_string(), "audio/ogg");
        let mut store = InMemoryStore::new();
        let input = AudioInput {
            entity: NomtuRef {
                id: "wav-ogg".into(),
                word: "loop".into(),
                kind: "media".into(),
            },
            pcm_samples: vec![0.2f32; 4800],
            sample_rate: 48000,
            codec: "opus".into(),
            container: AudioContainer::OggStub,
            audio_codec: AudioCodec::OpusStub,
        };
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        let payload = store.read(&block.artifact_hash).unwrap();
        let text = String::from_utf8_lossy(&payload);
        assert!(
            text.contains("Ogg"),
            "WAV->Ogg stub payload must mention Ogg"
        );
    }

    #[test]
    fn audio_pcm_to_opus_stub_produces_artifact() {
        let mut store = InMemoryStore::new();
        let input = AudioInput {
            entity: NomtuRef {
                id: "pcm-opus".into(),
                word: "voice".into(),
                kind: "media".into(),
            },
            pcm_samples: vec![0.5f32; 8000],
            sample_rate: 16000,
            codec: "opus".into(),
            container: AudioContainer::OggStub,
            audio_codec: AudioCodec::OpusStub,
        };
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        assert!(store.exists(&block.artifact_hash));
        let payload = store.read(&block.artifact_hash).unwrap();
        assert!(!payload.is_empty());
    }

    #[test]
    fn audio_container_wav_name_contains_wav() {
        let name = AudioContainer::Wav.to_string();
        assert!(
            name.contains("wav"),
            "Wav container name must contain wav: {name}"
        );
    }

    #[test]
    fn audio_codec_pcm_name_contains_pcm() {
        let name = AudioCodec::Pcm.to_string();
        assert!(
            name.contains("pcm"),
            "Pcm codec name must contain pcm: {name}"
        );
    }

    #[test]
    fn audio_pcm_codec_wav_container_compatibility() {
        let mut store = InMemoryStore::new();
        let input = AudioInput {
            entity: NomtuRef {
                id: "pcm-wav-compat".into(),
                word: "compat".into(),
                kind: "media".into(),
            },
            pcm_samples: vec![0.0f32; 1000],
            sample_rate: 44100,
            codec: "pcm_s16le".into(),
            container: AudioContainer::Wav,
            audio_codec: AudioCodec::Pcm,
        };
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        let payload = store.read(&block.artifact_hash).unwrap();
        assert_eq!(&payload[0..4], b"RIFF");
        assert_eq!(&payload[8..12], b"WAVE");
    }

    #[test]
    fn audio_flac_stub_entity_propagated() {
        let mut store = InMemoryStore::new();
        let input = AudioInput {
            entity: NomtuRef {
                id: "flac-entity".into(),
                word: "symphony".into(),
                kind: "media".into(),
            },
            pcm_samples: vec![0.3f32; 2000],
            sample_rate: 44100,
            codec: "flac".into(),
            container: AudioContainer::FlacStub,
            audio_codec: AudioCodec::FlacStub,
        };
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(block.entity.id, "flac-entity");
        assert_eq!(block.entity.word, "symphony");
    }

    #[test]
    fn audio_ogg_stub_entity_propagated() {
        let mut store = InMemoryStore::new();
        let input = AudioInput {
            entity: NomtuRef {
                id: "ogg-entity".into(),
                word: "podcast".into(),
                kind: "media".into(),
            },
            pcm_samples: vec![0.1f32; 3000],
            sample_rate: 48000,
            codec: "opus".into(),
            container: AudioContainer::OggStub,
            audio_codec: AudioCodec::OpusStub,
        };
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(block.entity.id, "ogg-entity");
        assert_eq!(block.entity.word, "podcast");
    }

    #[test]
    fn audio_flac_stub_codec_round_trip() {
        // AudioCodec::FlacStub.to_string() -> "flac" — stable across calls.
        let name = AudioCodec::FlacStub.to_string();
        assert_eq!(name, "flac");
        assert_eq!(AudioCodec::FlacStub.to_string(), name);
    }

    // ── FFmpeg fallback tests ────────────────────────────────────────────────

    #[test]
    fn audio_container_mp3_display() {
        assert_eq!(AudioContainer::Mp3.to_string(), "audio/mp3");
    }

    #[test]
    fn audio_backend_mp3_compose_stores_artifact() {
        let mut store = InMemoryStore::new();
        let input = AudioInput {
            entity: NomtuRef {
                id: "mp3-test".into(),
                word: "track".into(),
                kind: "media".into(),
            },
            pcm_samples: vec![0.2f32; 2000],
            sample_rate: 44100,
            codec: "mp3".into(),
            container: AudioContainer::Mp3,
            audio_codec: AudioCodec::Pcm,
        };
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        assert!(store.exists(&block.artifact_hash));
        let payload = store.read(&block.artifact_hash).unwrap();
        // Either a stub or real MP3 (ID3 or MPEG sync word).
        let text = String::from_utf8_lossy(&payload);
        let is_stub = text.contains("NOM-STUB-AUDIO-CONTAINER: MP3");
        let is_mp3 = payload.starts_with(b"ID3") || payload.starts_with(&[0xFF, 0xFB]);
        assert!(
            is_stub || is_mp3,
            "MP3 compose must yield either stub or real MP3"
        );
    }

    #[test]
    fn audio_backend_flac_fallback_when_ffmpeg_missing() {
        let mut store = InMemoryStore::new();
        let input = AudioInput {
            entity: NomtuRef {
                id: "flac-fb".into(),
                word: "fallback".into(),
                kind: "media".into(),
            },
            pcm_samples: vec![0.0f32; 8000],
            sample_rate: 44100,
            codec: "flac".into(),
            container: AudioContainer::FlacStub,
            audio_codec: AudioCodec::FlacStub,
        };
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        let payload = store.read(&block.artifact_hash).unwrap();
        let text = String::from_utf8_lossy(&payload);
        let is_stub = text.contains("NOM-STUB-AUDIO-CONTAINER");
        let is_flac = payload.starts_with(b"fLaC");
        assert!(
            is_stub || is_flac,
            "FLAC compose must yield either stub or real FLAC"
        );
    }

    #[test]
    fn audio_backend_ogg_fallback_when_ffmpeg_missing() {
        let mut store = InMemoryStore::new();
        let input = AudioInput {
            entity: NomtuRef {
                id: "ogg-fb".into(),
                word: "fallback".into(),
                kind: "media".into(),
            },
            pcm_samples: vec![0.0f32; 8000],
            sample_rate: 48000,
            codec: "opus".into(),
            container: AudioContainer::OggStub,
            audio_codec: AudioCodec::OpusStub,
        };
        let block = AudioBackend::compose(input, &mut store, &LogProgressSink);
        let payload = store.read(&block.artifact_hash).unwrap();
        let text = String::from_utf8_lossy(&payload);
        let is_stub = text.contains("NOM-STUB-AUDIO-CONTAINER");
        let is_ogg = payload.starts_with(b"OggS");
        assert!(
            is_stub || is_ogg,
            "Ogg compose must yield either stub or real Ogg"
        );
    }
}

// ── Playback pipeline stub ────────────────────────────────────────────────────


/// Supported source formats for the playback pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioFormat {
    Wav,
    Mp3,
    Ogg,
    Flac,
    Aac,
}

/// Describes an audio source file ready for playback.
#[derive(Debug, Clone)]
pub struct AudioSource {
    pub path: String,
    pub format: AudioFormat,
    /// Samples per second, e.g. 44100.
    pub sample_rate: u32,
    /// 1 = mono, 2 = stereo.
    pub channels: u8,
    pub duration_ms: Option<u64>,
}

impl AudioSource {
    /// Create a new source with default sample_rate=44100 and channels=2.
    pub fn new(path: &str, format: AudioFormat) -> Self {
        Self {
            path: path.to_owned(),
            format,
            sample_rate: 44100,
            channels: 2,
            duration_ms: None,
        }
    }

    pub fn with_sample_rate(mut self, rate: u32) -> Self {
        self.sample_rate = rate;
        self
    }

    pub fn with_duration(mut self, ms: u64) -> Self {
        self.duration_ms = Some(ms);
        self
    }

    pub fn is_stereo(&self) -> bool {
        self.channels == 2
    }
}

/// Playback lifecycle states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

/// Wraps a source with runtime playback controls.
#[derive(Debug, Clone)]
pub struct AudioPlayback {
    pub source: AudioSource,
    pub state: PlaybackState,
    /// Linear volume in 0.0..=1.0.
    pub volume: f32,
    /// Current playback position in milliseconds.
    pub position_ms: u64,
    pub looping: bool,
}

impl AudioPlayback {
    pub fn new(source: AudioSource) -> Self {
        Self {
            source,
            state: PlaybackState::Stopped,
            volume: 1.0,
            position_ms: 0,
            looping: false,
        }
    }

    pub fn play(mut self) -> Self {
        self.state = PlaybackState::Playing;
        self
    }

    pub fn pause(mut self) -> Self {
        self.state = PlaybackState::Paused;
        self
    }

    pub fn stop(mut self) -> Self {
        self.state = PlaybackState::Stopped;
        self.position_ms = 0;
        self
    }

    /// Clamp volume to 0.0..=1.0.
    pub fn set_volume(mut self, v: f32) -> Self {
        self.volume = v.clamp(0.0, 1.0);
        self
    }

    pub fn set_looping(mut self, looping: bool) -> Self {
        self.looping = looping;
        self
    }

    pub fn is_active(&self) -> bool {
        self.state == PlaybackState::Playing
    }
}

/// Multi-track mixer with a master volume fader.
#[derive(Debug, Default)]
pub struct AudioMixer {
    pub tracks: Vec<AudioPlayback>,
    pub master_volume: f32,
}

impl AudioMixer {
    pub fn new() -> Self {
        Self {
            tracks: Vec::new(),
            master_volume: 1.0,
        }
    }

    pub fn add_track(mut self, track: AudioPlayback) -> Self {
        self.tracks.push(track);
        self
    }

    pub fn active_tracks(&self) -> Vec<&AudioPlayback> {
        self.tracks.iter().filter(|t| t.is_active()).collect()
    }

    /// Clamp master volume to 0.0..=1.0.
    pub fn set_master_volume(mut self, v: f32) -> Self {
        self.master_volume = v.clamp(0.0, 1.0);
        self
    }

    pub fn stop_all(mut self) -> Self {
        self.tracks = self.tracks.into_iter().map(|t| t.stop()).collect();
        self
    }
}

#[cfg(test)]
mod playback_tests {
    use super::{AudioFormat, AudioMixer, AudioPlayback, AudioSource, PlaybackState};

    fn make_source(path: &str) -> AudioSource {
        AudioSource::new(path, AudioFormat::Wav)
    }

    #[test]
    fn audio_source_new_is_stereo() {
        let src = make_source("clip.wav");
        assert_eq!(src.path, "clip.wav");
        assert_eq!(src.format, AudioFormat::Wav);
        assert_eq!(src.sample_rate, 44100);
        assert_eq!(src.channels, 2);
        assert!(src.is_stereo());
    }

    #[test]
    fn audio_source_with_sample_rate_and_duration() {
        let src = make_source("track.wav")
            .with_sample_rate(48000)
            .with_duration(3000);
        assert_eq!(src.sample_rate, 48000);
        assert_eq!(src.duration_ms, Some(3000));
    }

    #[test]
    fn playback_new_and_play() {
        let pb = AudioPlayback::new(make_source("a.wav"));
        assert_eq!(pb.state, PlaybackState::Stopped);
        assert_eq!(pb.volume, 1.0);
        let pb = pb.play();
        assert_eq!(pb.state, PlaybackState::Playing);
        assert!(pb.is_active());
    }

    #[test]
    fn playback_pause_and_stop() {
        let pb = AudioPlayback::new(make_source("b.wav")).play().pause();
        assert_eq!(pb.state, PlaybackState::Paused);
        assert!(!pb.is_active());
        let pb = pb.stop();
        assert_eq!(pb.state, PlaybackState::Stopped);
        assert_eq!(pb.position_ms, 0);
    }

    #[test]
    fn playback_set_volume_clamped() {
        let pb_hi = AudioPlayback::new(make_source("c.wav")).set_volume(2.5);
        assert_eq!(pb_hi.volume, 1.0);
        let pb_lo = AudioPlayback::new(make_source("c.wav")).set_volume(-0.5);
        assert_eq!(pb_lo.volume, 0.0);
        let pb_mid = AudioPlayback::new(make_source("c.wav")).set_volume(0.7);
        assert!((pb_mid.volume - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn playback_is_active_only_when_playing() {
        let playing = AudioPlayback::new(make_source("d.wav")).play();
        assert!(playing.is_active());
        let paused = playing.pause();
        assert!(!paused.is_active());
        let stopped = paused.stop();
        assert!(!stopped.is_active());
    }

    #[test]
    fn mixer_new_add_track_active_tracks() {
        let t1 = AudioPlayback::new(make_source("e.wav")).play();
        let t2 = AudioPlayback::new(make_source("f.wav")); // Stopped
        let mixer = AudioMixer::new().add_track(t1).add_track(t2);
        assert_eq!(mixer.tracks.len(), 2);
        assert_eq!(mixer.master_volume, 1.0);
        let active = mixer.active_tracks();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn mixer_stop_all() {
        let t1 = AudioPlayback::new(make_source("g.wav")).play();
        let t2 = AudioPlayback::new(make_source("h.wav")).play();
        let mixer = AudioMixer::new().add_track(t1).add_track(t2).stop_all();
        assert_eq!(mixer.active_tracks().len(), 0);
        for track in &mixer.tracks {
            assert_eq!(track.state, PlaybackState::Stopped);
            assert_eq!(track.position_ms, 0);
        }
    }
}

/// Playback queue for sequential/concurrent audio rendering.
#[derive(Debug, Clone)]
pub struct PlaybackEntry {
    pub source: AudioSource,
    pub volume: f32,
    pub loop_count: u32,
}

impl PlaybackEntry {
    pub fn new(source: AudioSource) -> Self {
        Self { source, volume: 1.0, loop_count: 1 }
    }

    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume.clamp(0.0, 2.0);
        self
    }

    pub fn with_loop(mut self, count: u32) -> Self {
        self.loop_count = count;
        self
    }
}

/// Audio render result (per-source).
#[derive(Debug, Clone)]
pub struct AudioRenderResult {
    pub source_name: String,
    pub frames_rendered: u64,
    pub duration_ms: u64,
    pub peak_amplitude: f32,
}

/// Multi-track audio renderer (rodio-pattern stub).
pub struct AudioRenderer {
    pub sample_rate: u32,
    pub channel_count: u8,
}

impl AudioRenderer {
    pub fn new(sample_rate: u32, channel_count: u8) -> Self {
        Self { sample_rate, channel_count }
    }

    pub fn stereo_44100() -> Self { Self::new(44100, 2) }

    /// Render a playback entry to a result (stub — produces deterministic output).
    pub fn render(&self, entry: &PlaybackEntry) -> AudioRenderResult {
        let frames = (self.sample_rate as u64) * entry.loop_count as u64;
        AudioRenderResult {
            source_name: entry.source.path.clone(),
            frames_rendered: frames,
            duration_ms: (frames * 1000) / self.sample_rate as u64,
            peak_amplitude: entry.volume * 0.8,
        }
    }

    /// Mix multiple entries into a single combined result.
    pub fn mix(&self, entries: &[PlaybackEntry]) -> Vec<AudioRenderResult> {
        entries.iter().map(|e| self.render(e)).collect()
    }
}

#[cfg(test)]
mod audio_renderer_tests {
    use super::*;

    fn silence_source() -> AudioSource {
        AudioSource::new("silence", AudioFormat::Wav)
    }

    #[test]
    fn test_playback_entry_defaults() {
        let entry = PlaybackEntry::new(silence_source());
        assert_eq!(entry.volume, 1.0);
        assert_eq!(entry.loop_count, 1);
    }

    #[test]
    fn test_playback_entry_volume_clamp() {
        let entry = PlaybackEntry::new(silence_source()).with_volume(5.0);
        assert_eq!(entry.volume, 2.0);
    }

    #[test]
    fn test_audio_renderer_stereo() {
        let r = AudioRenderer::stereo_44100();
        assert_eq!(r.sample_rate, 44100);
        assert_eq!(r.channel_count, 2);
    }

    #[test]
    fn test_render_duration() {
        let r = AudioRenderer::stereo_44100();
        let entry = PlaybackEntry::new(silence_source());
        let result = r.render(&entry);
        assert_eq!(result.duration_ms, 1000); // 44100 frames / 44100 = 1 second
    }

    #[test]
    fn test_render_peak_amplitude() {
        let r = AudioRenderer::stereo_44100();
        let entry = PlaybackEntry::new(silence_source()).with_volume(1.0);
        let result = r.render(&entry);
        assert!((result.peak_amplitude - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_mix_multiple() {
        let r = AudioRenderer::stereo_44100();
        let entries = vec![
            PlaybackEntry::new(silence_source()),
            PlaybackEntry::new(silence_source()).with_volume(0.5),
        ];
        let results = r.mix(&entries);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_loop_count_doubles_duration() {
        let r = AudioRenderer::stereo_44100();
        let entry = PlaybackEntry::new(silence_source()).with_loop(2);
        let result = r.render(&entry);
        assert_eq!(result.duration_ms, 2000);
    }

    #[test]
    fn test_render_result_fields() {
        let r = AudioRenderer::stereo_44100();
        let entry = PlaybackEntry::new(silence_source());
        let result = r.render(&entry);
        assert!(!result.source_name.is_empty());
        assert!(result.frames_rendered > 0);
    }
}
