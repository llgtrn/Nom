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
}
