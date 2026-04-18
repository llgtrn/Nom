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

    #[test]
    fn paragraph_block_nesting_via_children() {
        let parent = ParagraphBlock {
            entity: NomtuRef::new("p-parent", "contain", "verb"),
            text: vec![],
            children: vec!["child-1".to_string(), "child-2".to_string()],
        };
        assert_eq!(parent.children.len(), 2);
        assert_eq!(parent.children[0], "child-1");
    }

    #[test]
    fn quote_block_text_content() {
        let entity = NomtuRef::new("q-01", "cite", "verb");
        let text = vec![DeltaOp::Insert {
            text: "To be or not to be".into(),
            attrs: Default::default(),
        }];
        let block = QuoteBlock {
            entity,
            text,
            children: vec![],
        };
        assert_eq!(block.text.len(), 1);
        if let DeltaOp::Insert { text: t, .. } = &block.text[0] {
            assert_eq!(t, "To be or not to be");
        }
    }

    #[test]
    fn delta_op_delete_and_retain_variants() {
        let delete_op = DeltaOp::Delete { count: 5 };
        let retain_op = DeltaOp::Retain {
            count: 3,
            attrs: Default::default(),
        };
        assert_eq!(delete_op, DeltaOp::Delete { count: 5 });
        assert_ne!(delete_op, DeltaOp::Delete { count: 3 });
        assert_eq!(
            retain_op,
            DeltaOp::Retain {
                count: 3,
                attrs: Default::default(),
            }
        );
    }

    #[test]
    fn list_block_todo_checked_field() {
        let entity = NomtuRef::new("lst-01", "track", "verb");
        let block = ListBlock {
            entity,
            text: vec![],
            list_type: ListType::Todo,
            checked: Some(true),
            children: vec![],
        };
        assert_eq!(block.list_type, ListType::Todo);
        assert_eq!(block.checked, Some(true));
    }

    #[test]
    fn database_block_columns_and_rows() {
        let entity = NomtuRef::new("db-01", "store", "verb");
        let columns = vec![
            Column {
                id: "c1".into(),
                name: "Name".into(),
                col_type: "text".into(),
            },
            Column {
                id: "c2".into(),
                name: "Score".into(),
                col_type: "number".into(),
            },
        ];
        let block = DatabaseBlock {
            entity,
            title: "My DB".into(),
            views: vec![DatabaseView {
                id: "v1".into(),
                mode: "table".into(),
            }],
            columns,
            rows: vec![vec![
                crate::slot::SlotValue::Text("Alice".into()),
                crate::slot::SlotValue::Number(95.0),
            ]],
        };
        assert_eq!(block.columns.len(), 2);
        assert_eq!(block.rows.len(), 1);
        assert_eq!(block.title, "My DB");
        assert_eq!(block.views[0].mode, "table");
    }

    #[test]
    fn attachment_block_fields() {
        let entity = NomtuRef::new("att-01", "attach", "verb");
        let block = AttachmentBlock {
            entity,
            name: "report.pdf".into(),
            size: 204800,
            blob_hash: [0xCCu8; 32],
            mime: "application/pdf".into(),
        };
        assert_eq!(block.name, "report.pdf");
        assert_eq!(block.size, 204800);
        assert_eq!(block.mime, "application/pdf");
        assert_eq!(block.blob_hash, [0xCCu8; 32]);
    }

    #[test]
    fn image_block_caption_and_dimensions() {
        let entity = NomtuRef::new("img-01", "display", "verb");
        let block = ImageBlock {
            entity,
            blob_hash: [0x01u8; 32],
            width: Some(800.0),
            height: Some(600.0),
            caption: "A sunset".into(),
        };
        assert_eq!(block.caption, "A sunset");
        assert_eq!(block.width, Some(800.0));
        assert_eq!(block.height, Some(600.0));
    }

    #[test]
    fn embed_block_in_prose_aspect_ratio() {
        let entity = NomtuRef::new("emb-01", "embed", "concept");
        let block = EmbedBlock {
            entity,
            url: "https://youtube.com/watch?v=abc".into(),
            embed_type: "youtube".into(),
            aspect_ratio: 16.0 / 9.0,
        };
        let expected = 16.0_f32 / 9.0_f32;
        assert!((block.aspect_ratio - expected).abs() < 0.001);
    }

    #[test]
    fn heading_level_range_valid_range() {
        for level in 1u8..=6 {
            let h = HeadingBlock {
                entity: NomtuRef::new("h", "t", "concept"),
                text: vec![],
                level,
                children: vec![],
            };
            assert_eq!(h.level_clamped(), level);
        }
    }

    #[test]
    fn heading_level_zero_clamps_to_one() {
        let h = HeadingBlock {
            entity: NomtuRef::new("h0", "title", "concept"),
            text: vec![],
            level: 0,
            children: vec![],
        };
        assert_eq!(h.level_clamped(), 1);
    }

    // ── plain text extraction helpers ─────────────────────────────────────────

    fn delta_plain_text(delta: &Delta) -> String {
        delta
            .iter()
            .filter_map(|op| {
                if let DeltaOp::Insert { text, .. } = op {
                    Some(text.as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("")
    }

    fn word_count(delta: &Delta) -> usize {
        let text = delta_plain_text(delta);
        text.split_whitespace().count()
    }

    #[test]
    fn plain_text_extraction_single_insert() {
        let delta = vec![DeltaOp::Insert {
            text: "hello world".into(),
            attrs: Default::default(),
        }];
        assert_eq!(delta_plain_text(&delta), "hello world");
    }

    #[test]
    fn plain_text_extraction_multiple_inserts() {
        let delta = vec![
            DeltaOp::Insert { text: "foo".into(), attrs: Default::default() },
            DeltaOp::Insert { text: " bar".into(), attrs: Default::default() },
        ];
        assert_eq!(delta_plain_text(&delta), "foo bar");
    }

    #[test]
    fn plain_text_extraction_ignores_delete_ops() {
        let delta = vec![
            DeltaOp::Insert { text: "hello".into(), attrs: Default::default() },
            DeltaOp::Delete { count: 3 },
        ];
        // delete ops contribute no text
        assert_eq!(delta_plain_text(&delta), "hello");
    }

    #[test]
    fn plain_text_extraction_ignores_retain_ops() {
        let delta = vec![
            DeltaOp::Retain { count: 5, attrs: Default::default() },
            DeltaOp::Insert { text: "world".into(), attrs: Default::default() },
        ];
        assert_eq!(delta_plain_text(&delta), "world");
    }

    #[test]
    fn plain_text_empty_delta_yields_empty_string() {
        let delta: Delta = vec![];
        assert_eq!(delta_plain_text(&delta), "");
    }

    #[test]
    fn word_count_single_word() {
        let delta = vec![DeltaOp::Insert { text: "nom".into(), attrs: Default::default() }];
        assert_eq!(word_count(&delta), 1);
    }

    #[test]
    fn word_count_multiple_words() {
        let delta = vec![DeltaOp::Insert {
            text: "one two three four five".into(),
            attrs: Default::default(),
        }];
        assert_eq!(word_count(&delta), 5);
    }

    #[test]
    fn word_count_empty_delta_is_zero() {
        let delta: Delta = vec![];
        assert_eq!(word_count(&delta), 0);
    }

    #[test]
    fn word_count_whitespace_only_delta_is_zero() {
        let delta = vec![DeltaOp::Insert { text: "   ".into(), attrs: Default::default() }];
        assert_eq!(word_count(&delta), 0);
    }

    #[test]
    fn word_count_splits_across_ops() {
        let delta = vec![
            DeltaOp::Insert { text: "hello ".into(), attrs: Default::default() },
            DeltaOp::Insert { text: "world".into(), attrs: Default::default() },
        ];
        assert_eq!(word_count(&delta), 2);
    }

    #[test]
    fn paragraph_block_plain_text_roundtrip() {
        let entity = NomtuRef::new("p-rt", "write", "verb");
        let block = ParagraphBlock {
            entity,
            text: vec![DeltaOp::Insert {
                text: "Nom is a language".into(),
                attrs: Default::default(),
            }],
            children: vec![],
        };
        let plain = delta_plain_text(&block.text);
        assert_eq!(plain, "Nom is a language");
        assert_eq!(word_count(&block.text), 4);
    }

    #[test]
    fn code_block_text_preserved_verbatim() {
        let entity = NomtuRef::new("code-rt", "compile", "verb");
        let src = "fn foo() -> u32 { 42 }";
        let block = CodeBlock {
            entity,
            language: "nom".into(),
            text: vec![DeltaOp::Insert { text: src.into(), attrs: Default::default() }],
            wrap: false,
        };
        assert_eq!(delta_plain_text(&block.text), src);
    }

    #[test]
    fn list_block_numbered_variant() {
        let entity = NomtuRef::new("lst-num", "list", "verb");
        let block = ListBlock {
            entity,
            text: vec![],
            list_type: ListType::Numbered,
            checked: None,
            children: vec![],
        };
        assert_eq!(block.list_type, ListType::Numbered);
        assert!(block.checked.is_none());
    }

    #[test]
    fn list_block_toggle_variant() {
        let entity = NomtuRef::new("lst-tog", "toggle", "verb");
        let block = ListBlock {
            entity,
            text: vec![],
            list_type: ListType::Toggle,
            checked: None,
            children: vec![],
        };
        assert_eq!(block.list_type, ListType::Toggle);
    }
}
