#[derive(Debug, Clone, PartialEq)]
pub enum Codec {
    Aac,
    H264,
    H265,
    Opus,
    Png,
    ProRes,
    Vp9,
    Webp,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Container {
    Mkv,
    Mov,
    Mp3,
    Mp4,
    Ogg,
    Webm,
}

impl Codec {
    pub fn is_video(&self) -> bool {
        matches!(self, Codec::H264 | Codec::H265 | Codec::Vp9 | Codec::ProRes)
    }

    pub fn is_audio(&self) -> bool {
        matches!(self, Codec::Aac | Codec::Opus)
    }

    pub fn preferred_container(&self) -> Container {
        match self {
            Codec::H264 | Codec::Aac => Container::Mp4,
            Codec::H265 => Container::Mp4,
            Codec::Vp9 | Codec::Opus => Container::Webm,
            Codec::ProRes => Container::Mov,
            Codec::Png | Codec::Webp => Container::Mp4, // image codecs: sensible default
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_video_correct() {
        assert!(Codec::H264.is_video());
        assert!(Codec::H265.is_video());
        assert!(Codec::Vp9.is_video());
        assert!(Codec::ProRes.is_video());
        assert!(!Codec::Aac.is_video());
        assert!(!Codec::Opus.is_video());
        assert!(!Codec::Png.is_video());
    }

    #[test]
    fn is_audio_correct() {
        assert!(Codec::Aac.is_audio());
        assert!(Codec::Opus.is_audio());
        assert!(!Codec::H264.is_audio());
        assert!(!Codec::Png.is_audio());
    }

    #[test]
    fn preferred_container_mapping() {
        assert_eq!(Codec::H264.preferred_container(), Container::Mp4);
        assert_eq!(Codec::Vp9.preferred_container(), Container::Webm);
        assert_eq!(Codec::Opus.preferred_container(), Container::Webm);
        assert_eq!(Codec::ProRes.preferred_container(), Container::Mov);
        assert_eq!(Codec::H265.preferred_container(), Container::Mp4);
    }
}
