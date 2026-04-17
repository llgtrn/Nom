use crate::block_schema::{BlockSchema, Role};
use crate::flavour::{CALLOUT, NOTE, PROSE};

/// Semantic kind of a prose block — mirrors common rich-text heading/list levels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProseKind {
    Text,
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
    Quote,
    Bulleted,
    Numbered,
    Todo,
}

/// Horizontal text alignment within a prose block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
    Justify,
}

/// Props carried by a prose (paragraph / heading / list item) block.
#[derive(Debug, Clone)]
pub struct ProseProps {
    pub text: String,
    pub text_align: TextAlign,
    pub kind: ProseKind,
    pub collapsed: bool,
}

impl Default for ProseProps {
    fn default() -> Self {
        Self {
            text: String::new(),
            text_align: TextAlign::Left,
            kind: ProseKind::Text,
            collapsed: false,
        }
    }
}

/// Static schema for prose blocks.
pub fn prose_schema() -> BlockSchema {
    BlockSchema {
        flavour: PROSE,
        version: 1,
        role: Role::Content,
        parents: &[NOTE, CALLOUT, PROSE],
        children: &[],
    }
}

/// Convert a prose block to a different kind while preserving its text.
pub fn to_kind(props: &mut ProseProps, new_kind: ProseKind) {
    props.kind = new_kind;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_schema::{validate_child, validate_parent};
    use crate::flavour::NOTE;

    #[test]
    fn schema_round_trip() {
        let s = prose_schema();
        assert_eq!(s.flavour, PROSE);
        assert_eq!(s.version, 1);
        assert_eq!(s.role, Role::Content);
        assert!(validate_parent(&s, NOTE).is_ok());
        // leaf — no children allowed
        assert!(validate_child(&s, NOTE).is_err());
    }

    #[test]
    fn to_kind_preserves_text() {
        let mut props = ProseProps {
            text: "hello world".to_string(),
            ..Default::default()
        };
        to_kind(&mut props, ProseKind::H2);
        assert_eq!(props.kind, ProseKind::H2);
        assert_eq!(props.text, "hello world");
    }
}
