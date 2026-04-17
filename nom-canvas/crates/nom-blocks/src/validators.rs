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

    /// Validator flags blocks whose entity.kind is not registered in the dict —
    /// the enforcement path for the blueprint's "NomtuRef is non-optional" invariant.
    #[test]
    fn validator_rejects_missing_nomtu() {
        let dict = StubDictReader::new();
        // An entity with an unrecognised kind simulates a block not backed by a real DB entry
        let block = BlockModel::new("b1", NomtuRef::new("?", "unknown_word", "nonexistent_kind"), "affine:note");
        let errors = validate_block(&block, &dict);
        assert!(!errors.is_empty(), "Validator must reject block with unknown nomtu kind");
        assert_eq!(errors[0].severity, Severity::Error);
        assert!(errors[0].message.contains("nonexistent_kind"));
    }

    #[test]
    fn validation_error_with_label_and_footer() {
        let err = ValidationError::error(0..5, "test error")
            .with_label(Label { span: 0..3, message: "here".into() })
            .with_footer("hint: fix the kind");
        assert_eq!(err.labels.len(), 1);
        assert_eq!(err.footers.len(), 1);
        assert_eq!(err.footers[0], "hint: fix the kind");
        assert_eq!(err.severity, Severity::Error);
    }

    #[test]
    fn slot_shape_validator_warns_unknown_slot() {
        let dict = StubDictReader::new(); // default shapes: only "input" and "output"
        let mut block = BlockModel::new("b2", NomtuRef::new("e2", "summarize", "verb"), "affine:paragraph");
        block.set_slot("nonexistent_slot", crate::slot::SlotValue::Bool(true));
        let errors = SlotShapeValidator.validate(&block, &dict);
        // "nonexistent_slot" is not in the default stub shapes → should produce a warning
        assert!(!errors.is_empty());
        assert_eq!(errors[0].severity, Severity::Warning);
        assert!(errors[0].message.contains("nonexistent_slot"));
    }

    #[test]
    fn slot_shape_validator_passes_known_slot() {
        use crate::dict_reader::ClauseShape;
        let dict = StubDictReader::new()
            .with_shapes("verb", vec![
                ClauseShape { name: "output".into(), grammar_shape: "text".into(), is_required: true, description: String::new() },
            ]);
        let mut block = BlockModel::new("b3", NomtuRef::new("e3", "fetch", "verb"), "affine:paragraph");
        block.set_slot("output", crate::slot::SlotValue::Text("result".into()));
        let errors = SlotShapeValidator.validate(&block, &dict);
        assert!(errors.is_empty(), "known slot must not produce warnings: {:?}", errors.iter().map(|e| &e.message).collect::<Vec<_>>());
    }

    #[test]
    fn validation_warning_severity() {
        let w = ValidationError::warning(0..4, "mild issue");
        assert_eq!(w.severity, Severity::Warning);
        assert_eq!(w.message, "mild issue");
    }

    /// validate_block returns no errors when block has no slots set
    #[test]
    fn validate_block_no_slots_no_errors() {
        let dict = StubDictReader::new();
        let block = BlockModel::new("b1", NomtuRef::new("e1", "plan", "concept"), "affine:note");
        let errors = validate_block(&block, &dict);
        assert!(errors.is_empty(), "block with no slots and known kind must validate clean");
    }

    /// Span range boundaries are preserved in ValidationError
    #[test]
    fn validation_error_span_preserved() {
        let err = ValidationError::error(5..20, "span test");
        assert_eq!(err.span.start, 5u32);
        assert_eq!(err.span.end, 20u32);
    }

    /// Multiple labels can be attached to a single ValidationError
    #[test]
    fn validation_error_multiple_labels() {
        let err = ValidationError::error(0..1, "multi")
            .with_label(Label { span: 0..1, message: "label 1".into() })
            .with_label(Label { span: 0..1, message: "label 2".into() });
        assert_eq!(err.labels.len(), 2);
        assert_eq!(err.labels[0].message, "label 1");
        assert_eq!(err.labels[1].message, "label 2");
    }

    /// GrammarDerivationValidator passes when kind is known
    #[test]
    fn grammar_derivation_validator_passes_known_kind() {
        let dict = StubDictReader::new();
        let block = BlockModel::new("b1", NomtuRef::new("e1", "fetch", "verb"), "affine:paragraph");
        let errors = GrammarDerivationValidator.validate(&block, &dict);
        assert!(errors.is_empty());
    }

    /// GrammarDerivationValidator error message includes the unknown kind name
    #[test]
    fn grammar_derivation_validator_error_includes_kind_name() {
        let dict = StubDictReader::new();
        let block = BlockModel::new("b1", NomtuRef::new("e1", "foo", "spooky_kind"), "affine:note");
        let errors = GrammarDerivationValidator.validate(&block, &dict);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("spooky_kind"));
    }
}
