#![deny(unsafe_code)]
use serde::{Deserialize, Serialize};
use crate::block_model::{NomtuRef, BlockId};
use crate::slot::SlotValue;

pub const FLAVOUR_PARAGRAPH: &str = "affine:paragraph";
pub const FLAVOUR_HEADING: &str = "affine:heading";
pub const FLAVOUR_LIST: &str = "affine:list";
pub const FLAVOUR_QUOTE: &str = "affine:quote";
pub const FLAVOUR_DIVIDER: &str = "affine:divider";
pub const FLAVOUR_CALLOUT: &str = "affine:callout";
pub const FLAVOUR_DATABASE: &str = "affine:database";
pub const FLAVOUR_LINKED_DOC: &str = "affine:linked-doc";
pub const FLAVOUR_BOOKMARK: &str = "affine:bookmark";
pub const FLAVOUR_ATTACHMENT: &str = "affine:attachment";
pub const FLAVOUR_IMAGE: &str = "affine:image";
pub const FLAVOUR_CODE: &str = "affine:code";
pub const FLAVOUR_EMBED: &str = "affine:embed-*";

// Quill Delta op (simplified subset — insert/delete/retain with attrs)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DeltaOp {
    Insert { text: String, attrs: std::collections::HashMap<String, String> },
    Delete { count: usize },
    Retain { count: usize, attrs: std::collections::HashMap<String, String> },
}

pub type Delta = Vec<DeltaOp>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ListType { Bulleted, Numbered, Todo, Toggle }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CalloutStyle { Info, Warning, Error, Success, Note }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Column { pub id: String, pub name: String, pub col_type: String }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DatabaseView { pub id: String, pub mode: String }  // mode: table, kanban, gallery, etc.

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParagraphBlock { pub entity: NomtuRef, pub text: Delta, pub children: Vec<BlockId> }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeadingBlock  { pub entity: NomtuRef, pub text: Delta, pub level: u8, pub children: Vec<BlockId> }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ListBlock { pub entity: NomtuRef, pub text: Delta, pub list_type: ListType, pub checked: Option<bool>, pub children: Vec<BlockId> }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuoteBlock { pub entity: NomtuRef, pub text: Delta, pub children: Vec<BlockId> }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DividerBlock { pub entity: NomtuRef }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalloutBlock { pub entity: NomtuRef, pub text: Delta, pub emoji: String, pub style: CalloutStyle }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DatabaseBlock { pub entity: NomtuRef, pub title: String, pub views: Vec<DatabaseView>, pub columns: Vec<Column>, pub rows: Vec<Vec<SlotValue>> }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkedDocBlock { pub entity: NomtuRef, pub page_id: String, pub params: String }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BookmarkBlock { pub entity: NomtuRef, pub url: String, pub title: Option<String>, pub description: Option<String>, pub favicon: Option<String> }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttachmentBlock { pub entity: NomtuRef, pub name: String, pub size: u64, pub blob_hash: [u8; 32], pub mime: String }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageBlock { pub entity: NomtuRef, pub blob_hash: [u8; 32], pub width: Option<f32>, pub height: Option<f32>, pub caption: String }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CodeBlock { pub entity: NomtuRef, pub language: String, pub text: Delta, pub wrap: bool }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmbedBlock { pub entity: NomtuRef, pub url: String, pub embed_type: String, pub aspect_ratio: f32 }

impl HeadingBlock {
    pub fn level_clamped(&self) -> u8 { self.level.clamp(1, 6) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_level_clamp() {
        let h = HeadingBlock {
            entity: NomtuRef::new("id", "title", "concept"),
            text: vec![],
            level: 9,
            children: vec![],
        };
        assert_eq!(h.level_clamped(), 6);
    }

    #[test]
    fn flavour_constants() {
        assert_eq!(FLAVOUR_PARAGRAPH, "affine:paragraph");
        assert_eq!(FLAVOUR_CODE, "affine:code");
        assert_eq!(FLAVOUR_DATABASE, "affine:database");
    }
}
