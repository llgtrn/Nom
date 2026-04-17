#![deny(unsafe_code)]
use crate::block_schema::{BlockSchema, Role};
use crate::flavour::{NOTE, SURFACE};
use crate::block_model::FractionalIndex;
use crate::media::BlobId;

#[derive(Clone, Debug, PartialEq)]
pub struct VideoBlockProps {
    pub source_id: BlobId,
    pub duration_ms: u64,
    pub width: u32,
    pub height: u32,
    pub fps: u16,
    pub thumbnail_strip: Vec<BlobId>,
    pub index: FractionalIndex,
}

impl VideoBlockProps {
    pub fn new(source_id: BlobId, duration_ms: u64, width: u32, height: u32, fps: u16) -> Self {
        Self {
            source_id,
            duration_ms,
            width,
            height,
            fps,
            thumbnail_strip: Vec::new(),
            index: "a0".to_owned(),
        }
    }

    pub fn aspect_ratio(&self) -> f32 {
        if self.height == 0 {
            1.0
        } else {
            self.width as f32 / self.height as f32
        }
    }

    pub fn add_thumbnail(&mut self, id: BlobId) {
        self.thumbnail_strip.push(id);
    }
}

pub fn video_block_schema() -> BlockSchema {
    BlockSchema {
        flavour: crate::compose::COMPOSE_VIDEO,
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
        let v = VideoBlockProps::new("blob-vid".to_owned(), 5000, 1920, 1080, 30);
        assert_eq!(v.source_id, "blob-vid");
        assert_eq!(v.duration_ms, 5000);
        assert_eq!(v.fps, 30);
        assert!(v.thumbnail_strip.is_empty());
        assert_eq!(v.index, "a0");
    }

    #[test]
    fn aspect_ratio_calculation() {
        let v = VideoBlockProps::new("b".to_owned(), 0, 1920, 1080, 24);
        let ratio = v.aspect_ratio();
        assert!((ratio - (16.0 / 9.0)).abs() < 1e-4, "ratio={}", ratio);
    }

    #[test]
    fn aspect_ratio_zero_height_returns_one() {
        let v = VideoBlockProps::new("b".to_owned(), 0, 100, 0, 24);
        assert_eq!(v.aspect_ratio(), 1.0);
    }

    #[test]
    fn add_thumbnail_appends() {
        let mut v = VideoBlockProps::new("b".to_owned(), 1000, 640, 360, 25);
        v.add_thumbnail("frame-0".to_owned());
        v.add_thumbnail("frame-1".to_owned());
        assert_eq!(v.thumbnail_strip.len(), 2);
    }

    #[test]
    fn schema_role_content() {
        assert_eq!(video_block_schema().role, Role::Content);
    }

    #[test]
    fn schema_flavour_correct() {
        assert_eq!(video_block_schema().flavour, "nom:compose:video");
    }
}
