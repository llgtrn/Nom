#![deny(unsafe_code)]
use crate::block_schema::{BlockSchema, Role};
use crate::flavour::{NOTE, SURFACE};
use crate::media::{BlobId, FractionalIndex};

#[derive(Clone, Debug, PartialEq)]
pub struct AudioBlockProps {
    pub source_id: BlobId,
    pub duration_ms: u64,
    pub sample_rate: u32,
    pub channels: u8,
    pub waveform: Vec<f32>,
    pub index: FractionalIndex,
}

impl AudioBlockProps {
    pub fn new(source_id: BlobId, duration_ms: u64, sample_rate: u32, channels: u8) -> Self {
        Self {
            source_id,
            duration_ms,
            sample_rate,
            channels,
            waveform: Vec::new(),
            index: "a0".to_owned(),
        }
    }

    pub fn add_waveform_samples(&mut self, samples: impl IntoIterator<Item = f32>) {
        self.waveform.extend(samples);
    }

    /// Returns true if sample_rate is a common audio multiple (44100, 48000, etc.).
    pub fn is_standard_sample_rate(&self) -> bool {
        matches!(self.sample_rate, 8000 | 11025 | 16000 | 22050 | 44100 | 48000 | 88200 | 96000)
    }
}

pub fn audio_block_schema() -> BlockSchema {
    BlockSchema {
        flavour: crate::compose::COMPOSE_AUDIO,
        version: 1,
        role: Role::Content,
        parents: &[NOTE, SURFACE],
        children: &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_schema::Role;

    #[test]
    fn new_sets_defaults() {
        let a = AudioBlockProps::new("blob-audio".to_owned(), 3000, 44100, 2);
        assert_eq!(a.duration_ms, 3000);
        assert_eq!(a.sample_rate, 44100);
        assert_eq!(a.channels, 2);
        assert!(a.waveform.is_empty());
        assert_eq!(a.index, "a0");
    }

    #[test]
    fn sample_rate_multiples_standard() {
        let a = AudioBlockProps::new("b".to_owned(), 0, 44100, 1);
        assert!(a.is_standard_sample_rate());
        let a2 = AudioBlockProps::new("b".to_owned(), 0, 48000, 2);
        assert!(a2.is_standard_sample_rate());
    }

    #[test]
    fn non_standard_sample_rate() {
        let a = AudioBlockProps::new("b".to_owned(), 0, 12345, 1);
        assert!(!a.is_standard_sample_rate());
    }

    #[test]
    fn channels_mono_and_stereo() {
        let mono = AudioBlockProps::new("b".to_owned(), 0, 44100, 1);
        let stereo = AudioBlockProps::new("b".to_owned(), 0, 44100, 2);
        assert_eq!(mono.channels, 1);
        assert_eq!(stereo.channels, 2);
    }

    #[test]
    fn waveform_populated() {
        let mut a = AudioBlockProps::new("b".to_owned(), 0, 44100, 1);
        a.add_waveform_samples([0.0_f32, 0.5, -0.5, 1.0]);
        assert_eq!(a.waveform.len(), 4);
        assert!((a.waveform[1] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn schema_role_content() {
        assert_eq!(audio_block_schema().role, Role::Content);
    }
}
