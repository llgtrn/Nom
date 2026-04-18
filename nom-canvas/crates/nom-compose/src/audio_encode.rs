/// AudioFormat — supported audio encoding formats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioFormat {
    Wav,
    Mp3,
    Ogg,
    Flac,
}

impl AudioFormat {
    /// Returns the human-readable format name.
    pub fn format_name(&self) -> &str {
        match self {
            AudioFormat::Wav => "WAV",
            AudioFormat::Mp3 => "MP3",
            AudioFormat::Ogg => "OGG",
            AudioFormat::Flac => "FLAC",
        }
    }

    /// Returns the file extension (without leading dot).
    pub fn file_extension(&self) -> &str {
        match self {
            AudioFormat::Wav => "wav",
            AudioFormat::Mp3 => "mp3",
            AudioFormat::Ogg => "ogg",
            AudioFormat::Flac => "flac",
        }
    }
}

/// AudioBuffer — PCM audio buffer holding interleaved f32 samples.
#[derive(Debug, Clone)]
pub struct AudioBuffer {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
}

impl AudioBuffer {
    /// Creates an empty buffer with the given sample rate and channel count.
    pub fn new(sample_rate: u32, channels: u16) -> Self {
        Self {
            samples: Vec::new(),
            sample_rate,
            channels,
        }
    }

    /// Appends a single interleaved sample value.
    pub fn push_sample(&mut self, s: f32) {
        self.samples.push(s);
    }

    /// Duration in seconds: total samples / (sample_rate * channels).
    pub fn duration_secs(&self) -> f32 {
        self.samples.len() as f32 / (self.sample_rate as f32 * self.channels as f32)
    }

    /// Number of complete frames (one sample per channel = one frame).
    pub fn frame_count(&self) -> usize {
        self.samples.len() / self.channels as usize
    }
}

/// AudioEncoder — encodes an AudioBuffer into bytes for a given format.
#[derive(Debug, Clone)]
pub struct AudioEncoder {
    pub format: AudioFormat,
}

impl AudioEncoder {
    /// Creates a new encoder for the specified format.
    pub fn new(format: AudioFormat) -> Self {
        Self { format }
    }

    /// Returns a stub header: sample_rate bytes (4 × u8) followed by channels (u16 → 2 × u8).
    pub fn encode_header(&self, buf: &AudioBuffer) -> Vec<u8> {
        let sr_bytes = buf.sample_rate.to_le_bytes(); // [u8; 4]
        let ch_bytes = buf.channels.to_le_bytes();    // [u8; 2]
        let mut header = Vec::with_capacity(6);
        header.extend_from_slice(&sr_bytes);
        header.extend_from_slice(&ch_bytes);
        header
    }

    /// Estimated output size: frame_count * channels * 2 bytes (16-bit PCM).
    pub fn estimated_output_bytes(&self, buf: &AudioBuffer) -> usize {
        buf.frame_count() * buf.channels as usize * 2
    }
}

/// RodioBackend — stub backend wrapping an AudioEncoder (rodio/symphonia pattern).
#[derive(Debug, Clone)]
pub struct RodioBackend {
    pub encoder: AudioEncoder,
}

impl RodioBackend {
    /// Wraps an existing AudioEncoder.
    pub fn new(encoder: AudioEncoder) -> Self {
        Self { encoder }
    }

    /// Stub render: returns the encoded header bytes.
    pub fn render(&self, buf: &AudioBuffer) -> Vec<u8> {
        self.encoder.encode_header(buf)
    }

    /// Returns false — this backend is a stub, not a real-time renderer.
    pub fn is_realtime(&self) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod audio_encode_tests {
    use super::*;

    #[test]
    fn audio_format_format_name() {
        assert_eq!(AudioFormat::Wav.format_name(), "WAV");
        assert_eq!(AudioFormat::Mp3.format_name(), "MP3");
        assert_eq!(AudioFormat::Ogg.format_name(), "OGG");
        assert_eq!(AudioFormat::Flac.format_name(), "FLAC");
    }

    #[test]
    fn audio_format_file_extension() {
        assert_eq!(AudioFormat::Wav.file_extension(), "wav");
        assert_eq!(AudioFormat::Mp3.file_extension(), "mp3");
        assert_eq!(AudioFormat::Ogg.file_extension(), "ogg");
        assert_eq!(AudioFormat::Flac.file_extension(), "flac");
    }

    #[test]
    fn audio_buffer_push_and_frame_count() {
        let mut buf = AudioBuffer::new(44100, 2);
        // Push 6 interleaved samples → 3 frames (stereo)
        for i in 0..6 {
            buf.push_sample(i as f32 * 0.1);
        }
        assert_eq!(buf.samples.len(), 6);
        assert_eq!(buf.frame_count(), 3);
    }

    #[test]
    fn audio_buffer_duration_secs() {
        let mut buf = AudioBuffer::new(44100, 1);
        // Push 44100 samples at 44100 Hz mono → 1.0 second
        for _ in 0..44100 {
            buf.push_sample(0.0);
        }
        let dur = buf.duration_secs();
        assert!((dur - 1.0_f32).abs() < 1e-5, "expected ~1.0 s, got {dur}");
    }

    #[test]
    fn audio_buffer_channels() {
        let buf = AudioBuffer::new(48000, 6);
        assert_eq!(buf.channels, 6);
        assert_eq!(buf.sample_rate, 48000);
    }

    #[test]
    fn audio_encoder_encode_header_not_empty() {
        let buf = AudioBuffer::new(44100, 2);
        let encoder = AudioEncoder::new(AudioFormat::Wav);
        let header = encoder.encode_header(&buf);
        assert!(
            !header.is_empty(),
            "encode_header must return non-empty bytes"
        );
        // Stub header = 4 bytes (sample_rate) + 2 bytes (channels) = 6 bytes
        assert_eq!(header.len(), 6, "stub header must be 6 bytes");
        // Verify sample_rate round-trips through the first 4 bytes
        let sr = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
        assert_eq!(sr, 44100);
        let ch = u16::from_le_bytes([header[4], header[5]]);
        assert_eq!(ch, 2);
    }

    #[test]
    fn audio_encoder_estimated_output_bytes() {
        let mut buf = AudioBuffer::new(44100, 2);
        // 4 frames × 2 channels = 8 samples
        for _ in 0..8 {
            buf.push_sample(0.0);
        }
        let encoder = AudioEncoder::new(AudioFormat::Flac);
        // frame_count=4, channels=2, 16-bit → 4*2*2 = 16
        assert_eq!(encoder.estimated_output_bytes(&buf), 16);
    }

    #[test]
    fn rodio_backend_render_returns_bytes() {
        let buf = AudioBuffer::new(22050, 1);
        let encoder = AudioEncoder::new(AudioFormat::Mp3);
        let backend = RodioBackend::new(encoder);
        let bytes = backend.render(&buf);
        assert!(!bytes.is_empty(), "render must return non-empty bytes");
    }

    #[test]
    fn rodio_backend_is_realtime_false() {
        let encoder = AudioEncoder::new(AudioFormat::Ogg);
        let backend = RodioBackend::new(encoder);
        assert!(!backend.is_realtime(), "stub backend must not be realtime");
    }
}
