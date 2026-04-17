#![deny(unsafe_code)]
use crate::block_schema::{BlockSchema, Role};
use crate::flavour::{NOTE, SURFACE};
use crate::block_model::FractionalIndex;
use crate::media::BlobId;

#[derive(Clone, Debug, PartialEq)]
pub struct DocumentBlockProps {
    pub source_id: BlobId,
    pub page_count: u32,
    pub current_page: u32,
    pub page_thumbnails: Vec<BlobId>,
    pub index: FractionalIndex,
}

impl DocumentBlockProps {
    pub fn new(source_id: BlobId, page_count: u32) -> Self {
        Self {
            source_id,
            page_count,
            current_page: 0,
            page_thumbnails: Vec::new(),
            index: "a0".to_owned(),
        }
    }

    /// Navigate to page. Saturates at page_count - 1.
    pub fn go_to_page(&mut self, page: u32) {
        if self.page_count == 0 {
            self.current_page = 0;
        } else {
            self.current_page = page.min(self.page_count - 1);
        }
    }

    pub fn add_page_thumbnail(&mut self, id: BlobId) {
        self.page_thumbnails.push(id);
    }
}

pub fn document_block_schema() -> BlockSchema {
    BlockSchema {
        flavour: crate::compose::COMPOSE_DOCUMENT,
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
        let doc = DocumentBlockProps::new("blob-doc".to_owned(), 10);
        assert_eq!(doc.page_count, 10);
        assert_eq!(doc.current_page, 0);
        assert!(doc.page_thumbnails.is_empty());
    }

    #[test]
    fn page_navigation_saturates_at_last_page() {
        let mut doc = DocumentBlockProps::new("b".to_owned(), 5);
        doc.go_to_page(99);
        assert_eq!(doc.current_page, 4);
    }

    #[test]
    fn page_navigation_valid_page() {
        let mut doc = DocumentBlockProps::new("b".to_owned(), 10);
        doc.go_to_page(3);
        assert_eq!(doc.current_page, 3);
    }

    #[test]
    fn page_thumbnails_len_can_match_page_count() {
        let mut doc = DocumentBlockProps::new("b".to_owned(), 3);
        doc.add_page_thumbnail("t0".to_owned());
        doc.add_page_thumbnail("t1".to_owned());
        doc.add_page_thumbnail("t2".to_owned());
        assert_eq!(doc.page_thumbnails.len(), doc.page_count as usize);
    }

    #[test]
    fn schema_role_content() {
        assert_eq!(document_block_schema().role, Role::Content);
    }
}
