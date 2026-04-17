//! Audio composition backend (TTS / music / sfx).
#![deny(unsafe_code)]

use crate::backend_trait::{CompositionBackend, ComposeSpec, ComposeOutput, ComposeError, InterruptFlag, ProgressSink};
use crate::kind::NomKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioFormat { Flac, Aac, Mp3, Opus, Wav }

impl AudioFormat {
    pub fn mime_type(self) -> &'static str {
        match self {
            Self::Flac => "audio/flac",
            Self::Aac  => "audio/aac",
            Self::Mp3  => "audio/mpeg",
            Self::Opus => "audio/opus",
            Self::Wav  => "audio/wav",
        }
    }
    pub fn is_lossless(self) -> bool { matches!(self, Self::Flac | Self::Wav) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioSource { Speech, Music, Sfx, Ambient }

#[derive(Clone, Debug, PartialEq)]
pub struct AudioSpec {
    pub text_or_prompt: String,
    pub source_kind: AudioSource,
    pub voice_id: Option<String>,
    pub sample_rate_hz: u32,
    pub channels: u8,
    pub format: AudioFormat,
    pub duration_ms: Option<u64>,
}

impl AudioSpec {
    pub fn new(text_or_prompt: impl Into<String>, source_kind: AudioSource) -> Self {
        Self {
            text_or_prompt: text_or_prompt.into(),
            source_kind,
            voice_id: None,
            sample_rate_hz: 44_100,
            channels: 2,
            format: AudioFormat::Flac,
            duration_ms: None,
        }
    }
    pub fn with_voice(mut self, voice_id: impl Into<String>) -> Self { self.voice_id = Some(voice_id.into()); self }
    pub fn with_format(mut self, format: AudioFormat) -> Self { self.format = format; self }
    pub fn with_sample_rate(mut self, hz: u32) -> Self { self.sample_rate_hz = hz; self }
    pub fn mono(mut self) -> Self { self.channels = 1; self }
    pub fn stereo(mut self) -> Self { self.channels = 2; self }
    pub fn estimate_bytes(&self) -> u64 {
        // Rough uncompressed estimate: sample_rate * channels * 16-bit * duration.
        let secs = self.duration_ms.unwrap_or(0) as f64 / 1000.0;
        (self.sample_rate_hz as f64 * self.channels as f64 * 2.0 * secs) as u64
    }
}

/// Timing-alignment word entry for lip-sync with media/video backend.
#[derive(Clone, Debug, PartialEq)]
pub struct WordTiming {
    pub word: String,
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("text/prompt must not be empty")]
    EmptyInput,
    #[error("sample_rate must be in 8000..=192000; got {0}")]
    InvalidSampleRate(u32),
    #[error("channels must be 1 or 2; got {0}")]
    InvalidChannels(u8),
}

pub fn validate(spec: &AudioSpec) -> Result<(), AudioError> {
    if spec.text_or_prompt.trim().is_empty() { return Err(AudioError::EmptyInput); }
    if !(8_000..=192_000).contains(&spec.sample_rate_hz) { return Err(AudioError::InvalidSampleRate(spec.sample_rate_hz)); }
    if !matches!(spec.channels, 1 | 2) { return Err(AudioError::InvalidChannels(spec.channels)); }
    Ok(())
}

pub struct StubAudioBackend;

impl CompositionBackend for StubAudioBackend {
    fn kind(&self) -> NomKind { NomKind::MediaAudio }
    fn name(&self) -> &str { "stub-audio" }
    fn compose(
        &self,
        _spec: &ComposeSpec,
        _progress: &dyn ProgressSink,
        _interrupt: &InterruptFlag,
    ) -> Result<ComposeOutput, ComposeError> {
        Ok(ComposeOutput {
            bytes: Vec::new(),
            mime_type: "audio/flac".to_string(),
            cost_cents: 0,
        })
    }
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mime_type_flac() {
        assert_eq!(AudioFormat::Flac.mime_type(), "audio/flac");
    }

    #[test]
    fn mime_type_aac() {
        assert_eq!(AudioFormat::Aac.mime_type(), "audio/aac");
    }

    #[test]
    fn mime_type_mp3() {
        assert_eq!(AudioFormat::Mp3.mime_type(), "audio/mpeg");
    }

    #[test]
    fn mime_type_opus() {
        assert_eq!(AudioFormat::Opus.mime_type(), "audio/opus");
    }

    #[test]
    fn mime_type_wav() {
        assert_eq!(AudioFormat::Wav.mime_type(), "audio/wav");
    }

    #[test]
    fn is_lossless_flac_and_wav() {
        assert!(AudioFormat::Flac.is_lossless());
        assert!(AudioFormat::Wav.is_lossless());
    }

    #[test]
    fn is_lossless_lossy_formats() {
        assert!(!AudioFormat::Aac.is_lossless());
        assert!(!AudioFormat::Mp3.is_lossless());
        assert!(!AudioFormat::Opus.is_lossless());
    }

    #[test]
    fn audio_spec_new_defaults() {
        let spec = AudioSpec::new("hello", AudioSource::Speech);
        assert_eq!(spec.sample_rate_hz, 44_100);
        assert_eq!(spec.channels, 2);
        assert_eq!(spec.format, AudioFormat::Flac);
        assert!(spec.voice_id.is_none());
        assert!(spec.duration_ms.is_none());
    }

    #[test]
    fn builder_chain() {
        let spec = AudioSpec::new("test prompt", AudioSource::Music)
            .with_voice("en-us-neural")
            .with_format(AudioFormat::Mp3)
            .with_sample_rate(22_050);
        assert_eq!(spec.voice_id.as_deref(), Some("en-us-neural"));
        assert_eq!(spec.format, AudioFormat::Mp3);
        assert_eq!(spec.sample_rate_hz, 22_050);
    }

    #[test]
    fn mono_and_stereo_builder() {
        let mono = AudioSpec::new("sfx", AudioSource::Sfx).mono();
        assert_eq!(mono.channels, 1);
        let stereo = mono.stereo();
        assert_eq!(stereo.channels, 2);
    }

    #[test]
    fn estimate_bytes_one_second_stereo_44100() {
        // 44100 samples/s * 2 channels * 2 bytes/sample * 1 second = 176400
        let spec = AudioSpec::new("x", AudioSource::Ambient)
            .with_sample_rate(44_100)
            .stereo();
        let mut s = spec.clone();
        s.duration_ms = Some(1_000);
        assert_eq!(s.estimate_bytes(), 176_400);
    }

    #[test]
    fn estimate_bytes_zero_when_no_duration() {
        let spec = AudioSpec::new("x", AudioSource::Speech);
        assert_eq!(spec.estimate_bytes(), 0);
    }

    #[test]
    fn validate_ok() {
        let spec = AudioSpec::new("hello world", AudioSource::Speech);
        assert!(validate(&spec).is_ok());
    }

    #[test]
    fn validate_empty_prompt() {
        let spec = AudioSpec::new("   ", AudioSource::Speech);
        assert!(matches!(validate(&spec), Err(AudioError::EmptyInput)));
    }

    #[test]
    fn validate_invalid_sample_rate() {
        let mut spec = AudioSpec::new("hello", AudioSource::Speech);
        spec.sample_rate_hz = 1_000; // below 8000
        assert!(matches!(validate(&spec), Err(AudioError::InvalidSampleRate(1_000))));
    }

    #[test]
    fn validate_invalid_channels() {
        let mut spec = AudioSpec::new("hello", AudioSource::Speech);
        spec.channels = 4;
        assert!(matches!(validate(&spec), Err(AudioError::InvalidChannels(4))));
    }

    #[test]
    fn word_timing_round_trip() {
        let wt = WordTiming { word: "hello".into(), start_ms: 0, end_ms: 350 };
        assert_eq!(wt.word, "hello");
        assert_eq!(wt.start_ms, 0);
        assert_eq!(wt.end_ms, 350);
    }

    #[test]
    fn stub_backend_kind_and_name() {
        let b = StubAudioBackend;
        assert_eq!(b.kind(), NomKind::MediaAudio);
        assert_eq!(b.name(), "stub-audio");
    }

    #[test]
    fn stub_backend_compose_returns_empty_flac() {
        use crate::backend_trait::InterruptFlag;
        struct NoopSink;
        impl ProgressSink for NoopSink {
            fn notify(&self, _: u32, _: &str) {}
        }
        let b = StubAudioBackend;
        let spec = ComposeSpec { kind: NomKind::MediaAudio, params: vec![] };
        let out = b.compose(&spec, &NoopSink, &InterruptFlag::new()).unwrap();
        assert!(out.bytes.is_empty());
        assert_eq!(out.mime_type, "audio/flac");
        assert_eq!(out.cost_cents, 0);
    }
}
