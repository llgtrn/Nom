#![deny(unsafe_code)]
use crate::block_model::NomtuRef;
use crate::prose::{CodeBlock, FLAVOUR_CODE};
use serde::{Deserialize, Serialize};

pub const NOMX_LANGUAGE: &str = "nomx";

/// A .nomx code block — backed by nom-editor buffer (Wave B)
/// Uses nom:code flavour with language="nomx"
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NomxBlock {
    pub entity: NomtuRef,
    pub source: String,
    pub wrap: bool,
    pub show_line_numbers: bool,
}

impl NomxBlock {
    pub fn new(entity: NomtuRef, source: impl Into<String>) -> Self {
        Self {
            entity,
            source: source.into(),
            wrap: false,
            show_line_numbers: true,
        }
    }

    pub fn flavour() -> &'static str {
        FLAVOUR_CODE
    }

    pub fn language() -> &'static str {
        NOMX_LANGUAGE
    }

    pub fn to_code_block(&self) -> CodeBlock {
        CodeBlock {
            entity: self.entity.clone(),
            language: NOMX_LANGUAGE.into(),
            text: vec![crate::prose::DeltaOp::Insert {
                text: self.source.clone(),
                attrs: Default::default(),
            }],
            wrap: self.wrap,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nomx_block_flavour() {
        assert_eq!(NomxBlock::flavour(), "nom:code");
        assert_eq!(NomxBlock::language(), "nomx");
    }

    #[test]
    fn nomx_converts_to_code_block() {
        let entity = NomtuRef::new("id", "summarize", "verb");
        let block = NomxBlock::new(entity, "define x that is 42");
        let code = block.to_code_block();
        assert_eq!(code.language, "nomx");
        assert!(!code.text.is_empty());
    }

    #[test]
    fn nomx_block_source_preserved() {
        let entity = NomtuRef::new("nx-01", "compile", "verb");
        let src = "define area that width * height";
        let block = NomxBlock::new(entity, src);
        assert_eq!(block.source, src);
    }

    #[test]
    fn nomx_block_defaults() {
        let entity = NomtuRef::new("nx-02", "run", "verb");
        let block = NomxBlock::new(entity, "x");
        assert!(!block.wrap);
        assert!(block.show_line_numbers);
    }

    #[test]
    fn nomx_to_code_block_text_contains_source() {
        let entity = NomtuRef::new("nx-03", "eval", "verb");
        let src = "define y that 99";
        let block = NomxBlock::new(entity, src);
        let code = block.to_code_block();
        // The single Insert delta op should contain the source text
        if let crate::prose::DeltaOp::Insert { text, .. } = &code.text[0] {
            assert_eq!(text, src);
        } else {
            panic!("expected Insert delta op");
        }
    }

    #[test]
    fn nomx_to_code_block_wrap_matches() {
        let entity = NomtuRef::new("nx-04", "format", "verb");
        let mut block = NomxBlock::new(entity, "x");
        block.wrap = true;
        let code = block.to_code_block();
        assert!(code.wrap);
    }

    #[test]
    fn nomx_block_entity_preserved() {
        let entity = NomtuRef::new("nx-05", "parse", "verb");
        let block = NomxBlock::new(entity, "");
        assert_eq!(block.entity.id, "nx-05");
        assert_eq!(block.entity.word, "parse");
        assert_eq!(block.entity.kind, "verb");
    }

    /// NomX serialization roundtrip: serialize to JSON then deserialize back
    #[test]
    fn nomx_block_json_roundtrip() {
        let entity = NomtuRef::new("nx-rt", "serialize", "verb");
        let block = NomxBlock::new(entity, "define z that 7");
        let json = serde_json::to_string(&block).expect("serialize");
        let back: NomxBlock = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.source, "define z that 7");
        assert_eq!(back.entity.id, "nx-rt");
    }
}
