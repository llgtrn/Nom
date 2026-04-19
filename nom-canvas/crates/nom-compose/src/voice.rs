//! Controllable voice synthesis / TTS interface.
//!
//! Inspired by VoxCPM. MVP provides a trait + stub + HTTP client for an
//! external TTS inference server (FastAPI/gRPC). Future: native ONNX/Candle
//! voice-cloning model.

use serde::{Deserialize, Serialize};

/// Error type for voice synthesis operations.
#[derive(Debug)]
pub enum VoiceError {
    Http(reqwest::Error),
    Json(serde_json::Error),
    Api { status: u16, message: String },
    NotConfigured,
}

impl std::fmt::Display for VoiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VoiceError::Http(e) => write!(f, "HTTP error: {e}"),
            VoiceError::Json(e) => write!(f, "JSON error: {e}"),
            VoiceError::Api { status, message } => write!(f, "API error {status}: {message}"),
            VoiceError::NotConfigured => write!(f, "voice backend not configured"),
        }
    }
}

impl std::error::Error for VoiceError {}

impl From<reqwest::Error> for VoiceError {
    fn from(e: reqwest::Error) -> Self {
        VoiceError::Http(e)
    }
}

impl From<serde_json::Error> for VoiceError {
    fn from(e: serde_json::Error) -> Self {
        VoiceError::Json(e)
    }
}

/// Voice identity / speaker descriptor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Speaker {
    pub id: String,
    pub name: String,
}

impl Speaker {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
        }
    }
}

/// Synthesis parameters (speed, pitch, emotion).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisParams {
    pub speed: f32,
    pub pitch: f32,
}

impl Default for SynthesisParams {
    fn default() -> Self {
        Self {
            speed: 1.0,
            pitch: 1.0,
        }
    }
}

/// Trait for text-to-speech backends.
pub trait TtsBackend: Send + Sync {
    /// Synthesize `text` into audio bytes (e.g. WAV or MP3).
    fn synthesize(&self, text: &str, speaker: &Speaker, params: &SynthesisParams) -> Result<Vec<u8>, VoiceError>;
    /// List available speakers.
    fn speakers(&self) -> Result<Vec<Speaker>, VoiceError>;
}

/// Stub TTS backend that returns a synthetic sine-wave WAV.
pub struct StubTtsBackend;

impl StubTtsBackend {
    pub fn new() -> Self {
        Self
    }

    /// Generate a minimal mono WAV file (sine wave) for the given text length.
    fn synthesize_wav(&self, text: &str) -> Vec<u8> {
        let sample_rate: u32 = 16000;
        let duration_sec = (text.len() as f32 * 0.1).max(0.5);
        let num_samples = (sample_rate as f32 * duration_sec) as u32;
        let data_size = num_samples * 2;
        let chunk_size: u32 = 36 + data_size;
        let mut data = Vec::with_capacity(44 + data_size as usize);

        // WAV header
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&chunk_size.to_le_bytes());
        data.extend_from_slice(b"WAVE");
        data.extend_from_slice(b"fmt ");
        data.extend_from_slice(&16u32.to_le_bytes()); // subchunk1 size
        data.extend_from_slice(&1u16.to_le_bytes()); // PCM
        data.extend_from_slice(&1u16.to_le_bytes()); // mono
        data.extend_from_slice(&sample_rate.to_le_bytes());
        data.extend_from_slice(&(sample_rate * 2).to_le_bytes()); // byte rate
        data.extend_from_slice(&2u16.to_le_bytes()); // block align
        data.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
        data.extend_from_slice(b"data");
        data.extend_from_slice(&data_size.to_le_bytes());

        // Sine wave samples
        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;
            let freq = 440.0;
            let sample = ((t * freq * 2.0 * std::f32::consts::PI).sin() * 32767.0) as i16;
            data.extend_from_slice(&sample.to_le_bytes());
        }
        data
    }
}

impl Default for StubTtsBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl TtsBackend for StubTtsBackend {
    fn synthesize(&self, text: &str, _speaker: &Speaker, _params: &SynthesisParams) -> Result<Vec<u8>, VoiceError> {
        Ok(self.synthesize_wav(text))
    }

    fn speakers(&self) -> Result<Vec<Speaker>, VoiceError> {
        Ok(vec![Speaker::new("default", "Default Voice")])
    }
}

/// HTTP client for an external TTS server.
pub struct HttpTtsBackend {
    base_url: String,
    client: reqwest::blocking::Client,
}

impl HttpTtsBackend {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: reqwest::blocking::Client::new(),
        }
    }
}

impl TtsBackend for HttpTtsBackend {
    fn synthesize(&self, text: &str, speaker: &Speaker, params: &SynthesisParams) -> Result<Vec<u8>, VoiceError> {
        let req = serde_json::json!({
            "text": text,
            "speaker_id": speaker.id,
            "speed": params.speed,
            "pitch": params.pitch,
        });
        let resp = self
            .client
            .post(format!("{}/synthesize", self.base_url))
            .json(&req)
            .send()?;
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = resp.text().unwrap_or_default();
            return Err(VoiceError::Api { status, message });
        }
        Ok(resp.bytes()?.to_vec())
    }

    fn speakers(&self) -> Result<Vec<Speaker>, VoiceError> {
        let resp = self.client.get(format!("{}/speakers", self.base_url)).send()?;
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = resp.text().unwrap_or_default();
            return Err(VoiceError::Api { status, message });
        }
        let speakers: Vec<Speaker> = resp.json()?;
        Ok(speakers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_tts_produces_wav() {
        let backend = StubTtsBackend::new();
        let speaker = Speaker::new("s1", "Test");
        let wav = backend.synthesize("hello world", &speaker, &SynthesisParams::default()).unwrap();
        assert!(wav.len() > 44, "WAV must have header + data");
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
    }

    #[test]
    fn stub_tts_speakers_returns_default() {
        let backend = StubTtsBackend::new();
        let speakers = backend.speakers().unwrap();
        assert_eq!(speakers.len(), 1);
        assert_eq!(speakers[0].id, "default");
    }

    #[test]
    fn synthesis_params_default() {
        let p = SynthesisParams::default();
        assert_eq!(p.speed, 1.0);
        assert_eq!(p.pitch, 1.0);
    }

    #[test]
    fn voice_error_display() {
        let e = VoiceError::NotConfigured;
        assert_eq!(format!("{e}"), "voice backend not configured");
    }
}
