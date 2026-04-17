#![deny(unsafe_code)]
use serde::{Deserialize, Serialize};
use crate::block_model::NomtuRef;
use crate::prose::{CodeBlock, FLAVOUR_CODE};

pub const NOMX_LANGUAGE: &str = "nomx";

/// A .nomx code block — backed by nom-editor buffer (Wave B)
/// Uses affine:code flavour with language="nomx"
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
        assert_eq!(NomxBlock::flavour(), "affine:code");
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
}
