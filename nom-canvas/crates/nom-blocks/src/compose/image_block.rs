#![deny(unsafe_code)]
use crate::block_schema::{BlockSchema, Role};
use crate::flavour::{NOTE, SURFACE};
use crate::block_model::FractionalIndex;
use crate::media::BlobId;

#[derive(Clone, Debug, PartialEq)]
pub struct ImageBlockProps {
    pub source_id: BlobId,
    pub width: u32,
    pub height: u32,
    pub variants: Vec<BlobId>,
    pub selected_variant: usize,
    pub index: FractionalIndex,
}

impl ImageBlockProps {
    pub fn new(source_id: BlobId, width: u32, height: u32) -> Self {
        Self {
            source_id,
            width,
            height,
            variants: Vec::new(),
            selected_variant: 0,
            index: "a0".to_owned(),
        }
    }

    /// Select a variant by index. Clamps to len-1 if out of bounds.
    pub fn select_variant(&mut self, idx: usize) {
        if self.variants.is_empty() {
            self.selected_variant = 0;
        } else {
            self.selected_variant = idx.min(self.variants.len() - 1);
        }
    }

    pub fn add_variant(&mut self, id: BlobId) {
        self.variants.push(id);
    }
}

pub fn image_block_schema() -> BlockSchema {
    BlockSchema {
        flavour: crate::compose::COMPOSE_IMAGE,
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
        let img = ImageBlockProps::new("blob-img".to_owned(), 800, 600);
        assert_eq!(img.source_id, "blob-img");
        assert_eq!(img.width, 800);
        assert_eq!(img.height, 600);
        assert!(img.variants.is_empty());
        assert_eq!(img.selected_variant, 0);
    }

    #[test]
    fn variant_pick_in_range() {
        let mut img = ImageBlockProps::new("b".to_owned(), 100, 100);
        img.add_variant("v0".to_owned());
        img.add_variant("v1".to_owned());
        img.add_variant("v2".to_owned());
        img.select_variant(1);
        assert_eq!(img.selected_variant, 1);
    }

    #[test]
    fn clamp_selected_variant_to_len_minus_one() {
        let mut img = ImageBlockProps::new("b".to_owned(), 100, 100);
        img.add_variant("v0".to_owned());
        img.add_variant("v1".to_owned());
        img.select_variant(99);
        assert_eq!(img.selected_variant, 1);
    }

    #[test]
    fn select_variant_empty_variants_stays_zero() {
        let mut img = ImageBlockProps::new("b".to_owned(), 100, 100);
        img.select_variant(5);
        assert_eq!(img.selected_variant, 0);
    }

    #[test]
    fn schema_role_content() {
        assert_eq!(image_block_schema().role, Role::Content);
    }
}
