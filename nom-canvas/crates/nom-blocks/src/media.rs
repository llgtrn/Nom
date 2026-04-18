#![deny(unsafe_code)]
use crate::block_model::NomtuRef;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaBlock {
    pub entity: NomtuRef,
    pub blob_hash: [u8; 32],
    pub mime: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_ms: Option<u64>,
}

impl MediaBlock {
    pub fn new(entity: NomtuRef, blob_hash: [u8; 32], mime: impl Into<String>) -> Self {
        Self {
            entity,
            blob_hash,
            mime: mime.into(),
            width: None,
            height: None,
            duration_ms: None,
        }
    }
    pub fn is_video(&self) -> bool {
        self.mime.starts_with("video/")
    }
    pub fn is_audio(&self) -> bool {
        self.mime.starts_with("audio/")
    }
    pub fn is_image(&self) -> bool {
        self.mime.starts_with("image/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn media_type_detection() {
        let entity = NomtuRef::new("id", "w", "media");
        let m = MediaBlock::new(entity, [0u8; 32], "video/mp4");
        assert!(m.is_video());
        assert!(!m.is_audio());
    }

    #[test]
    fn media_block_is_audio() {
        let entity = NomtuRef::new("aud-01", "play", "verb");
        let m = MediaBlock::new(entity, [0u8; 32], "audio/mp3");
        assert!(m.is_audio());
        assert!(!m.is_video());
        assert!(!m.is_image());
    }

    #[test]
    fn media_block_is_image() {
        let entity = NomtuRef::new("img-01", "display", "verb");
        let m = MediaBlock::new(entity, [1u8; 32], "image/png");
        assert!(m.is_image());
        assert!(!m.is_video());
        assert!(!m.is_audio());
    }

    #[test]
    fn media_block_optional_fields_default_none() {
        let entity = NomtuRef::new("med-02", "record", "verb");
        let m = MediaBlock::new(entity, [0u8; 32], "video/webm");
        assert!(m.width.is_none());
        assert!(m.height.is_none());
        assert!(m.duration_ms.is_none());
    }

    #[test]
    fn media_block_blob_hash_stored() {
        let entity = NomtuRef::new("med-03", "encode", "verb");
        let hash = [0xABu8; 32];
        let m = MediaBlock::new(entity, hash, "image/jpeg");
        assert_eq!(m.blob_hash, [0xABu8; 32]);
    }

    #[test]
    fn media_block_entity_preserved() {
        let entity = NomtuRef::new("med-04", "stream", "verb");
        let m = MediaBlock::new(entity, [0u8; 32], "video/mp4");
        assert_eq!(m.entity.id, "med-04");
        assert_eq!(m.entity.word, "stream");
    }

    #[test]
    fn media_block_mime_generic_not_video_audio_image() {
        let entity = NomtuRef::new("med-05", "attach", "verb");
        let m = MediaBlock::new(entity, [0u8; 32], "application/pdf");
        assert!(!m.is_video());
        assert!(!m.is_audio());
        assert!(!m.is_image());
    }

    #[test]
    fn media_block_with_dimensions_set() {
        let entity = NomtuRef::new("med-06", "frame", "verb");
        let mut m = MediaBlock::new(entity, [0u8; 32], "image/webp");
        m.width = Some(1920);
        m.height = Some(1080);
        assert_eq!(m.width, Some(1920));
        assert_eq!(m.height, Some(1080));
    }

    #[test]
    fn media_block_with_duration_set() {
        let entity = NomtuRef::new("med-07", "play", "verb");
        let mut m = MediaBlock::new(entity, [0u8; 32], "audio/ogg");
        m.duration_ms = Some(300_000);
        assert_eq!(m.duration_ms, Some(300_000));
    }

    #[test]
    fn media_block_is_image_jpeg() {
        let entity = NomtuRef::new("med-08", "display", "verb");
        let m = MediaBlock::new(entity, [0u8; 32], "image/jpeg");
        assert!(m.is_image());
        assert!(!m.is_video());
        assert!(!m.is_audio());
    }

    #[test]
    fn media_block_is_video_webm() {
        let entity = NomtuRef::new("med-09", "stream", "verb");
        let m = MediaBlock::new(entity, [0u8; 32], "video/webm");
        assert!(m.is_video());
    }

    #[test]
    fn media_block_is_audio_wav() {
        let entity = NomtuRef::new("med-10", "record", "verb");
        let m = MediaBlock::new(entity, [0u8; 32], "audio/wav");
        assert!(m.is_audio());
    }

    #[test]
    fn media_block_clone_preserves_all_fields() {
        let entity = NomtuRef::new("med-11", "encode", "verb");
        let hash = [0x42u8; 32];
        let mut m = MediaBlock::new(entity, hash, "video/mp4");
        m.width = Some(640);
        m.height = Some(480);
        m.duration_ms = Some(5000);
        let cloned = m.clone();
        assert_eq!(cloned.entity.id, "med-11");
        assert_eq!(cloned.blob_hash, hash);
        assert_eq!(cloned.width, Some(640));
        assert_eq!(cloned.height, Some(480));
        assert_eq!(cloned.duration_ms, Some(5000));
    }

    #[test]
    fn media_block_mime_text_not_media() {
        let entity = NomtuRef::new("med-12", "read", "verb");
        let m = MediaBlock::new(entity, [0u8; 32], "text/plain");
        assert!(!m.is_video());
        assert!(!m.is_audio());
        assert!(!m.is_image());
    }

    #[test]
    fn media_block_mime_stored_exactly() {
        let entity = NomtuRef::new("med-13", "store", "verb");
        let m = MediaBlock::new(entity, [0u8; 32], "image/svg+xml");
        assert_eq!(m.mime, "image/svg+xml");
        assert!(m.is_image());
    }

    #[test]
    fn media_block_entity_kind_stored() {
        let entity = NomtuRef::new("med-14", "capture", "MediaUnit");
        let m = MediaBlock::new(entity, [0u8; 32], "video/mp4");
        assert_eq!(m.entity.kind, "MediaUnit");
    }

    // ── wave AG-8: additional media tests ────────────────────────────────────

    #[test]
    fn media_mime_type_for_png() {
        let entity = NomtuRef::new("m-png", "display", "verb");
        let m = MediaBlock::new(entity, [0u8; 32], "image/png");
        assert_eq!(m.mime, "image/png");
        assert!(m.is_image());
    }

    #[test]
    fn media_mime_type_for_mp4() {
        let entity = NomtuRef::new("m-mp4", "play", "verb");
        let m = MediaBlock::new(entity, [0u8; 32], "video/mp4");
        assert_eq!(m.mime, "video/mp4");
        assert!(m.is_video());
    }

    #[test]
    fn media_dimensions_positive_when_set() {
        let entity = NomtuRef::new("m-dim", "capture", "verb");
        let mut m = MediaBlock::new(entity, [0u8; 32], "image/png");
        m.width = Some(1920);
        m.height = Some(1080);
        assert!(m.width.unwrap() > 0);
        assert!(m.height.unwrap() > 0);
    }

    #[test]
    fn media_file_size_zero_for_empty_hash() {
        // An all-zero blob hash signals an empty or placeholder blob
        let entity = NomtuRef::new("m-empty", "placeholder", "verb");
        let m = MediaBlock::new(entity, [0u8; 32], "application/octet-stream");
        assert_eq!(m.blob_hash, [0u8; 32]);
    }

    #[test]
    fn media_block_is_image_gif() {
        let entity = NomtuRef::new("m-gif", "animate", "verb");
        let m = MediaBlock::new(entity, [0u8; 32], "image/gif");
        assert!(m.is_image());
        assert!(!m.is_video());
        assert!(!m.is_audio());
    }
}
