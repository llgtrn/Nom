#![deny(unsafe_code)]
use crate::block_model::BlockModel;

/// Span = Range<u32> byte offsets (exact yara-x type alias)
pub type Span = std::ops::Range<u32>;

#[derive(Clone, Debug, PartialEq)]
pub enum Severity { Error, Warning, Info }

#[derive(Clone, Debug)]
pub struct Label {
    pub span: Span,
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct ValidationError {
    pub span: Span,
    pub message: String,
    pub severity: Severity,
    pub labels: Vec<Label>,
    pub footers: Vec<String>,
}

impl ValidationError {
    pub fn error(span: Span, message: impl Into<String>) -> Self {
        Self { span, message: message.into(), severity: Severity::Error, labels: vec![], footers: vec![] }
    }
    pub fn warning(span: Span, message: impl Into<String>) -> Self {
        Self { span, message: message.into(), severity: Severity::Warning, labels: vec![], footers: vec![] }
    }
    pub fn with_label(mut self, label: Label) -> Self {
        self.labels.push(label);
        self
    }
    pub fn with_footer(mut self, footer: impl Into<String>) -> Self {
        self.footers.push(footer.into());
        self
    }
}

// Sealed trait pattern (yara-x style)
mod sealed { pub trait Sealed {} }
trait BlockValidatorInternal: sealed::Sealed {}
#[allow(private_bounds)]
pub trait BlockValidator: BlockValidatorInternal {
    fn validate(&self, block: &BlockModel, dict: &dyn crate::dict_reader::DictReader) -> Vec<ValidationError>;
}

pub struct GrammarDerivationValidator;
impl sealed::Sealed for GrammarDerivationValidator {}
impl BlockValidatorInternal for GrammarDerivationValidator {}
impl BlockValidator for GrammarDerivationValidator {
    fn validate(&self, block: &BlockModel, dict: &dyn crate::dict_reader::DictReader) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if !dict.is_known_kind(&block.entity.kind) {
            errors.push(ValidationError::error(
                0..block.entity.kind.len() as u32,
                format!("Unknown grammar kind: '{}'", block.entity.kind),
            ).with_footer(format!("Entity: {}", block.entity.word)));
        }
        errors
    }
}

pub struct SlotShapeValidator;
impl sealed::Sealed for SlotShapeValidator {}
impl BlockValidatorInternal for SlotShapeValidator {}
impl BlockValidator for SlotShapeValidator {
    fn validate(&self, block: &BlockModel, dict: &dyn crate::dict_reader::DictReader) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        let known_shapes = dict.clause_shapes_for(&block.entity.kind);
        for (slot_name, _) in &block.slots {
            if !known_shapes.iter().any(|s| &s.name == slot_name) {
                errors.push(ValidationError::warning(
                    0..slot_name.len() as u32,
                    format!("Slot '{}' not in clause_shapes for kind '{}'", slot_name, block.entity.kind),
                ));
            }
        }
        errors
    }
}

pub fn validate_block(block: &BlockModel, dict: &dyn crate::dict_reader::DictReader) -> Vec<ValidationError> {
    let mut all = Vec::new();
    all.extend(GrammarDerivationValidator.validate(block, dict));
    all.extend(SlotShapeValidator.validate(block, dict));
    all
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_model::NomtuRef;
    use crate::stub_dict::StubDictReader;

    #[test]
    fn validates_known_kind() {
        let dict = StubDictReader::new();
        let block = BlockModel::new("id1", NomtuRef::new("e1", "summarize", "verb"), "affine:paragraph");
        let errors = validate_block(&block, &dict);
        assert!(errors.is_empty(), "Expected no errors for known kind 'verb'");
    }

    #[test]
    fn rejects_unknown_kind() {
        let dict = StubDictReader::new();
        let block = BlockModel::new("id1", NomtuRef::new("e1", "xyz", "alien_kind_xyz"), "test");
        let errors = validate_block(&block, &dict);
        assert!(!errors.is_empty());
        assert_eq!(errors[0].severity, Severity::Error);
    }

    #[test]
    fn span_is_range_u32() {
        let span: Span = 0..10u32;
        assert_eq!(span.start, 0u32);
        assert_eq!(span.end, 10u32);
    }
}
