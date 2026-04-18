#[derive(Debug, Clone, PartialEq)]
pub enum MediaKind {
    Audio,
    Glyph,
    Image,
    Vector,
    Video,
}

#[derive(Debug, Clone)]
pub struct MediaUnit {
    pub id: String,
    pub kind: MediaKind,
    pub path: Option<String>,
    pub duration_ms: Option<u64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

impl MediaUnit {
    pub fn new(id: &str, kind: MediaKind) -> Self {
        Self {
            id: id.to_owned(),
            kind,
            path: None,
            duration_ms: None,
            width: None,
            height: None,
        }
    }

    pub fn with_path(mut self, path: &str) -> Self {
        self.path = Some(path.to_owned());
        self
    }

    pub fn with_dimensions(mut self, w: u32, h: u32) -> Self {
        self.width = Some(w);
        self.height = Some(h);
        self
    }

    /// True for media kinds that have a temporal dimension.
    pub fn is_temporal(&self) -> bool {
        matches!(self.kind, MediaKind::Audio | MediaKind::Video)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_unit_defaults() {
        let u = MediaUnit::new("m1", MediaKind::Image);
        assert_eq!(u.id, "m1");
        assert_eq!(u.kind, MediaKind::Image);
        assert!(u.path.is_none());
        assert!(u.width.is_none());
    }

    #[test]
    fn with_path_sets_field() {
        let u = MediaUnit::new("m2", MediaKind::Video).with_path("/tmp/clip.mp4");
        assert_eq!(u.path.as_deref(), Some("/tmp/clip.mp4"));
    }

    #[test]
    fn is_temporal_audio_and_video_only() {
        assert!(MediaUnit::new("m3", MediaKind::Audio).is_temporal());
        assert!(MediaUnit::new("m4", MediaKind::Video).is_temporal());
        assert!(!MediaUnit::new("m5", MediaKind::Image).is_temporal());
        assert!(!MediaUnit::new("m6", MediaKind::Vector).is_temporal());
        assert!(!MediaUnit::new("m7", MediaKind::Glyph).is_temporal());
    }
}
