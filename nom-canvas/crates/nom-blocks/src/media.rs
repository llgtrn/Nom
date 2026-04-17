#![deny(unsafe_code)]
use serde::{Deserialize, Serialize};
use crate::block_model::NomtuRef;

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
        Self { entity, blob_hash, mime: mime.into(), width: None, height: None, duration_ms: None }
    }
    pub fn is_video(&self) -> bool { self.mime.starts_with("video/") }
    pub fn is_audio(&self) -> bool { self.mime.starts_with("audio/") }
    pub fn is_image(&self) -> bool { self.mime.starts_with("image/") }
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
}
