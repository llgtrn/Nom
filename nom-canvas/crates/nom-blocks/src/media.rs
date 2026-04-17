//! Image and attachment block schemas.
#![deny(unsafe_code)]

use crate::block_schema::{BlockSchema, Role};
use crate::flavour::{MEDIA_ATTACHMENT, MEDIA_IMAGE, NOTE, SURFACE};

/// Content-addressed blob identifier (SHA-256 or equivalent handle).
pub type BlobId = String;

/// Fractional index for stable sibling ordering.
pub type FractionalIndex = String;

/// Bounding box in model coordinates: "x y width height".
pub type Xywh = String;

// ---------------------------------------------------------------------------
// Image
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct ImageProps {
    pub source_id: BlobId,
    pub xywh: Xywh,
    pub rotate: f32,
    pub width: u32,
    pub height: u32,
    pub caption: Option<String>,
    pub index: FractionalIndex,
}

impl ImageProps {
    pub fn new(source_id: BlobId, width: u32, height: u32) -> Self {
        Self {
            xywh: format!("0 0 {} {}", width, height),
            rotate: 0.0,
            caption: None,
            index: "a0".to_owned(),
            source_id,
            width,
            height,
        }
    }

    pub fn with_caption(mut self, caption: impl Into<String>) -> Self {
        self.caption = Some(caption.into());
        self
    }

    pub fn aspect_ratio(&self) -> f32 {
        if self.height == 0 {
            1.0
        } else {
            self.width as f32 / self.height as f32
        }
    }
}

/// Static schema for image blocks.
pub fn image_schema() -> BlockSchema {
    BlockSchema {
        flavour: MEDIA_IMAGE,
        version: 1,
        role: Role::Content,
        parents: &[NOTE, SURFACE],
        children: &[],
    }
}

// ---------------------------------------------------------------------------
// Attachment
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct AttachmentProps {
    pub source_id: BlobId,
    pub name: String,
    pub size: u64,
    pub mime: String,
    pub embed: bool,
    pub caption: Option<String>,
}

impl AttachmentProps {
    pub fn new(
        source_id: BlobId,
        name: impl Into<String>,
        size: u64,
        mime: impl Into<String>,
    ) -> Self {
        Self {
            source_id,
            name: name.into(),
            size,
            mime: mime.into(),
            embed: false,
            caption: None,
        }
    }

    /// Returns `true` for PDF or image/* MIME types.
    pub fn is_embeddable(&self) -> bool {
        self.mime == "application/pdf" || self.mime.starts_with("image/")
    }
}

/// Static schema for attachment blocks.
pub fn attachment_schema() -> BlockSchema {
    BlockSchema {
        flavour: MEDIA_ATTACHMENT,
        version: 1,
        role: Role::Content,
        parents: &[NOTE, SURFACE],
        children: &[],
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_schema::Role;

    #[test]
    fn image_props_new_sensible_defaults() {
        let p = ImageProps::new("blob-abc".to_owned(), 800, 600);
        assert_eq!(p.source_id, "blob-abc");
        assert_eq!(p.width, 800);
        assert_eq!(p.height, 600);
        assert_eq!(p.xywh, "0 0 800 600");
        assert_eq!(p.rotate, 0.0);
        assert_eq!(p.caption, None);
        assert_eq!(p.index, "a0");
    }

    #[test]
    fn image_aspect_ratio_zero_height_returns_one() {
        let p = ImageProps::new("blob".to_owned(), 100, 0);
        assert_eq!(p.aspect_ratio(), 1.0);
    }

    #[test]
    fn image_aspect_ratio_sixteen_by_nine() {
        let p = ImageProps::new("blob".to_owned(), 1920, 1080);
        let ratio = p.aspect_ratio();
        assert!((ratio - (16.0 / 9.0)).abs() < 1e-4, "ratio={}", ratio);
    }

    #[test]
    fn image_with_caption_chains() {
        let p = ImageProps::new("blob".to_owned(), 100, 100).with_caption("hello");
        assert_eq!(p.caption.as_deref(), Some("hello"));
    }

    #[test]
    fn image_schema_role_is_content() {
        assert_eq!(image_schema().role, Role::Content);
    }

    #[test]
    fn attachment_embeddable_pdf_and_image() {
        let pdf = AttachmentProps::new("b1".to_owned(), "doc.pdf", 1024, "application/pdf");
        assert!(pdf.is_embeddable());

        let png = AttachmentProps::new("b2".to_owned(), "photo.png", 512, "image/png");
        assert!(png.is_embeddable());
    }

    #[test]
    fn attachment_not_embeddable_zip() {
        let zip = AttachmentProps::new("b3".to_owned(), "archive.zip", 2048, "application/zip");
        assert!(!zip.is_embeddable());
    }

    #[test]
    fn attachment_schema_role_is_content() {
        assert_eq!(attachment_schema().role, Role::Content);
    }
}
