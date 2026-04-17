#![deny(unsafe_code)]
use crate::block_model::{BlockId, NomtuRef};
use crate::slot::SlotValue;
use serde::{Deserialize, Serialize};

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
pub const FLAVOUR_SURFACE: &str = "affine:surface";
pub const FLAVOUR_NOTE: &str = "affine:note";

// Quill Delta op (simplified subset — insert/delete/retain with attrs)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DeltaOp {
    Insert {
        text: String,
        attrs: std::collections::HashMap<String, String>,
    },
    Delete {
        count: usize,
    },
    Retain {
        count: usize,
        attrs: std::collections::HashMap<String, String>,
    },
}

pub type Delta = Vec<DeltaOp>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ListType {
    Bulleted,
    Numbered,
    Todo,
    Toggle,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CalloutStyle {
    Info,
    Warning,
    Error,
    Success,
    Note,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Column {
    pub id: String,
    pub name: String,
    pub col_type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DatabaseView {
    pub id: String,
    pub mode: String,
} // mode: table, kanban, gallery, etc.

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParagraphBlock {
    pub entity: NomtuRef,
    pub text: Delta,
    pub children: Vec<BlockId>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeadingBlock {
    pub entity: NomtuRef,
    pub text: Delta,
    pub level: u8,
    pub children: Vec<BlockId>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ListBlock {
    pub entity: NomtuRef,
    pub text: Delta,
    pub list_type: ListType,
    pub checked: Option<bool>,
    pub children: Vec<BlockId>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuoteBlock {
    pub entity: NomtuRef,
    pub text: Delta,
    pub children: Vec<BlockId>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DividerBlock {
    pub entity: NomtuRef,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalloutBlock {
    pub entity: NomtuRef,
    pub text: Delta,
    pub emoji: String,
    pub style: CalloutStyle,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DatabaseBlock {
    pub entity: NomtuRef,
    pub title: String,
    pub views: Vec<DatabaseView>,
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<SlotValue>>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkedDocBlock {
    pub entity: NomtuRef,
    pub page_id: String,
    pub params: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BookmarkBlock {
    pub entity: NomtuRef,
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub favicon: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttachmentBlock {
    pub entity: NomtuRef,
    pub name: String,
    pub size: u64,
    pub blob_hash: [u8; 32],
    pub mime: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageBlock {
    pub entity: NomtuRef,
    pub blob_hash: [u8; 32],
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub caption: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CodeBlock {
    pub entity: NomtuRef,
    pub language: String,
    pub text: Delta,
    pub wrap: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmbedBlock {
    pub entity: NomtuRef,
    pub url: String,
    pub embed_type: String,
    pub aspect_ratio: f32,
}

impl HeadingBlock {
    pub fn level_clamped(&self) -> u8 {
        self.level.clamp(1, 6)
    }
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

    #[test]
    fn block_heading_nomturef_required() {
        let entity = NomtuRef::new("heading-01", "title", "concept");
        let block = HeadingBlock {
            entity: entity.clone(),
            text: vec![],
            level: 1,
            children: vec![],
        };
        assert!(!block.entity.id.is_empty());
        assert_eq!(block.entity.id, "heading-01");
    }

    #[test]
    fn block_para_text_preserved() {
        let entity = NomtuRef::new("para-01", "summarize", "verb");
        let text = vec![DeltaOp::Insert {
            text: "hello world".into(),
            attrs: std::collections::HashMap::new(),
        }];
        let block = ParagraphBlock {
            entity,
            text: text.clone(),
            children: vec![],
        };
        assert_eq!(block.text.len(), 1);
        if let DeltaOp::Insert { text: t, .. } = &block.text[0] {
            assert_eq!(t, "hello world");
        } else {
            panic!("expected Insert op");
        }
    }

    #[test]
    fn block_list_items_count() {
        let entity = NomtuRef::new("list-01", "enumerate", "verb");
        let make_insert = |s: &str| DeltaOp::Insert {
            text: s.into(),
            attrs: std::collections::HashMap::new(),
        };
        let children = vec![
            "item-1".to_string(),
            "item-2".to_string(),
            "item-3".to_string(),
        ];
        let block = ListBlock {
            entity,
            text: vec![make_insert("• items")],
            list_type: ListType::Bulleted,
            checked: None,
            children: children.clone(),
        };
        assert_eq!(block.children.len(), 3);
        assert_eq!(block.list_type, ListType::Bulleted);
    }

    #[test]
    fn block_code_language_field() {
        let entity = NomtuRef::new("code-01", "compile", "verb");
        let block = CodeBlock {
            entity,
            language: "rust".into(),
            text: vec![DeltaOp::Insert {
                text: "fn main(){}".into(),
                attrs: std::collections::HashMap::new(),
            }],
            wrap: false,
        };
        assert_eq!(block.language, "rust");
        assert_eq!(block.text.len(), 1);
    }

    #[test]
    fn block_callout_emoji_field() {
        let entity = NomtuRef::new("callout-01", "warn", "verb");
        let block = CalloutBlock {
            entity,
            text: vec![],
            emoji: "⚠️".into(),
            style: CalloutStyle::Warning,
        };
        assert_eq!(block.emoji, "⚠️");
    }

    #[test]
    fn block_divider_no_content() {
        let entity = NomtuRef::new("div-01", "separate", "verb");
        let block = DividerBlock { entity };
        // DividerBlock has no text/children fields — structural guarantee
        assert_eq!(block.entity.id, "div-01");
    }

    #[test]
    fn block_linked_doc_target_ref() {
        let entity = NomtuRef::new("link-01", "reference", "concept");
        let block = LinkedDocBlock {
            entity,
            page_id: "page-xyz".into(),
            params: String::new(),
        };
        assert_eq!(block.page_id, "page-xyz");
        assert_eq!(block.entity.id, "link-01");
    }

    #[test]
    fn heading_level_clamp_min() {
        let h = HeadingBlock {
            entity: NomtuRef::new("id", "title", "concept"),
            text: vec![],
            level: 1,
            children: vec![],
        };
        // level 1 is already within [1,6], should stay 1
        assert_eq!(h.level_clamped(), 1);
    }

    #[test]
    fn block_type_display_flavour_constants_are_distinct() {
        let flavours = [
            FLAVOUR_PARAGRAPH,
            FLAVOUR_HEADING,
            FLAVOUR_LIST,
            FLAVOUR_QUOTE,
            FLAVOUR_DIVIDER,
            FLAVOUR_CALLOUT,
            FLAVOUR_DATABASE,
            FLAVOUR_LINKED_DOC,
            FLAVOUR_BOOKMARK,
            FLAVOUR_ATTACHMENT,
            FLAVOUR_IMAGE,
            FLAVOUR_CODE,
        ];
        let unique: std::collections::HashSet<_> = flavours.iter().collect();
        assert_eq!(
            unique.len(),
            flavours.len(),
            "every block flavour constant must be distinct"
        );
    }

    #[test]
    fn list_type_variants_are_eq() {
        assert_eq!(ListType::Bulleted, ListType::Bulleted);
        assert_ne!(ListType::Bulleted, ListType::Numbered);
        assert_ne!(ListType::Todo, ListType::Toggle);
    }

    #[test]
    fn bookmark_block_optional_fields() {
        let entity = NomtuRef::new("bm-01", "link", "concept");
        let block = BookmarkBlock {
            entity,
            url: "https://example.com".into(),
            title: Some("Example".into()),
            description: None,
            favicon: None,
        };
        assert_eq!(block.url, "https://example.com");
        assert!(block.title.is_some());
        assert!(block.description.is_none());
    }
}
