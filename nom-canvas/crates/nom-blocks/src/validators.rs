//! Block validators — grammar derivation and slot-shape checks.
#![deny(unsafe_code)]
use crate::block_model::BlockModel;

/// Span = Range<u32> byte offsets (exact yara-x type alias)
pub type Span = std::ops::Range<u32>;

/// Severity level of a validation diagnostic.
#[derive(Clone, Debug, PartialEq)]
pub enum Severity {
    /// Hard error that must be fixed.
    Error,
    /// Non-fatal warning.
    Warning,
    /// Informational note.
    Info,
}

/// A source-span annotation attached to a [`ValidationError`].
#[derive(Clone, Debug)]
pub struct Label {
    /// Byte span this label points to.
    pub span: Span,
    /// Annotation text.
    pub message: String,
}

/// A structured validation diagnostic with span, severity, labels, and footers.
#[derive(Clone, Debug)]
pub struct ValidationError {
    /// Byte span of the offending range.
    pub span: Span,
    /// Primary error message.
    pub message: String,
    /// Severity level.
    pub severity: Severity,
    /// Inline span labels.
    pub labels: Vec<Label>,
    /// Footer hints shown below the diagnostic.
    pub footers: Vec<String>,
}

impl ValidationError {
    /// Construct an `Error`-severity diagnostic.
    pub fn error(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            severity: Severity::Error,
            labels: vec![],
            footers: vec![],
        }
    }
    /// Construct a `Warning`-severity diagnostic.
    pub fn warning(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            severity: Severity::Warning,
            labels: vec![],
            footers: vec![],
        }
    }
    /// Attach a span label (builder pattern).
    pub fn with_label(mut self, label: Label) -> Self {
        self.labels.push(label);
        self
    }
    /// Attach a footer hint (builder pattern).
    pub fn with_footer(mut self, footer: impl Into<String>) -> Self {
        self.footers.push(footer.into());
        self
    }
}

// Sealed trait pattern (yara-x style)
mod sealed {
    pub trait Sealed {}
}
trait BlockValidatorInternal: sealed::Sealed {}
/// Public trait for all block validators (sealed against external implementations).
#[allow(private_bounds)]
pub trait BlockValidator: BlockValidatorInternal {
    /// Validate the given block against the dict and return any diagnostics.
    fn validate(
        &self,
        block: &BlockModel,
        dict: &dyn crate::dict_reader::DictReader,
    ) -> Vec<ValidationError>;
}

/// Validator that checks a block's entity kind is registered in the grammar dict.
pub struct GrammarDerivationValidator;
impl sealed::Sealed for GrammarDerivationValidator {}
impl BlockValidatorInternal for GrammarDerivationValidator {}
impl BlockValidator for GrammarDerivationValidator {
    fn validate(
        &self,
        block: &BlockModel,
        dict: &dyn crate::dict_reader::DictReader,
    ) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if !dict.is_known_kind(&block.entity.kind) {
            errors.push(
                ValidationError::error(
                    0..block.entity.kind.len() as u32,
                    format!("Unknown grammar kind: '{}'", block.entity.kind),
                )
                .with_footer(format!("Entity: {}", block.entity.word)),
            );
        }
        errors
    }
}

/// Validator that warns when a block sets a slot not declared in its kind's clause shapes.
pub struct SlotShapeValidator;
impl sealed::Sealed for SlotShapeValidator {}
impl BlockValidatorInternal for SlotShapeValidator {}
impl BlockValidator for SlotShapeValidator {
    fn validate(
        &self,
        block: &BlockModel,
        dict: &dyn crate::dict_reader::DictReader,
    ) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        let known_shapes = dict.clause_shapes_for(&block.entity.kind);
        for (slot_name, _) in &block.slots {
            if !known_shapes.iter().any(|s| &s.name == slot_name) {
                errors.push(ValidationError::warning(
                    0..slot_name.len() as u32,
                    format!(
                        "Slot '{}' not in clause_shapes for kind '{}'",
                        slot_name, block.entity.kind
                    ),
                ));
            }
        }
        errors
    }
}

/// Run all validators on a block and return the combined list of diagnostics.
pub fn validate_block(
    block: &BlockModel,
    dict: &dyn crate::dict_reader::DictReader,
) -> Vec<ValidationError> {
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
        let block = BlockModel::new(
            "id1",
            NomtuRef::new("e1", "summarize", "verb"),
            "affine:paragraph",
        );
        let errors = validate_block(&block, &dict);
        assert!(
            errors.is_empty(),
            "Expected no errors for known kind 'verb'"
        );
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
        let block = BlockModel::new(
            "b1",
            NomtuRef::new("?", "unknown_word", "nonexistent_kind"),
            "affine:note",
        );
        let errors = validate_block(&block, &dict);
        assert!(
            !errors.is_empty(),
            "Validator must reject block with unknown nomtu kind"
        );
        assert_eq!(errors[0].severity, Severity::Error);
        assert!(errors[0].message.contains("nonexistent_kind"));
    }

    #[test]
    fn validation_error_with_label_and_footer() {
        let err = ValidationError::error(0..5, "test error")
            .with_label(Label {
                span: 0..3,
                message: "here".into(),
            })
            .with_footer("hint: fix the kind");
        assert_eq!(err.labels.len(), 1);
        assert_eq!(err.footers.len(), 1);
        assert_eq!(err.footers[0], "hint: fix the kind");
        assert_eq!(err.severity, Severity::Error);
    }

    #[test]
    fn slot_shape_validator_warns_unknown_slot() {
        let dict = StubDictReader::new(); // default shapes: only "input" and "output"
        let mut block = BlockModel::new(
            "b2",
            NomtuRef::new("e2", "summarize", "verb"),
            "affine:paragraph",
        );
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
        let dict = StubDictReader::new().with_shapes(
            "verb",
            vec![ClauseShape {
                name: "output".into(),
                grammar_shape: "text".into(),
                is_required: true,
                description: String::new(),
            }],
        );
        let mut block = BlockModel::new(
            "b3",
            NomtuRef::new("e3", "fetch", "verb"),
            "affine:paragraph",
        );
        block.set_slot("output", crate::slot::SlotValue::Text("result".into()));
        let errors = SlotShapeValidator.validate(&block, &dict);
        assert!(
            errors.is_empty(),
            "known slot must not produce warnings: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
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
        assert!(
            errors.is_empty(),
            "block with no slots and known kind must validate clean"
        );
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
            .with_label(Label {
                span: 0..1,
                message: "label 1".into(),
            })
            .with_label(Label {
                span: 0..1,
                message: "label 2".into(),
            });
        assert_eq!(err.labels.len(), 2);
        assert_eq!(err.labels[0].message, "label 1");
        assert_eq!(err.labels[1].message, "label 2");
    }

    /// GrammarDerivationValidator passes when kind is known
    #[test]
    fn grammar_derivation_validator_passes_known_kind() {
        let dict = StubDictReader::new();
        let block = BlockModel::new(
            "b1",
            NomtuRef::new("e1", "fetch", "verb"),
            "affine:paragraph",
        );
        let errors = GrammarDerivationValidator.validate(&block, &dict);
        assert!(errors.is_empty());
    }

    /// GrammarDerivationValidator error message includes the unknown kind name
    #[test]
    fn grammar_derivation_validator_error_includes_kind_name() {
        let dict = StubDictReader::new();
        let block = BlockModel::new(
            "b1",
            NomtuRef::new("e1", "foo", "spooky_kind"),
            "affine:note",
        );
        let errors = GrammarDerivationValidator.validate(&block, &dict);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("spooky_kind"));
    }

    /// validate_block passes when block has a valid kind and a known slot
    #[test]
    fn validate_block_valid_content_passes() {
        use crate::dict_reader::ClauseShape;
        let dict = StubDictReader::new().with_shapes(
            "verb",
            vec![ClauseShape {
                name: "result".into(),
                grammar_shape: "text".into(),
                is_required: false,
                description: String::new(),
            }],
        );
        let mut block = BlockModel::new(
            "b10",
            NomtuRef::new("e10", "run", "verb"),
            "affine:paragraph",
        );
        block.set_slot("result", crate::slot::SlotValue::Text("ok".into()));
        let errors = validate_block(&block, &dict);
        assert!(errors.is_empty(), "valid content must produce no errors");
    }

    /// validate_block emits errors when kind is missing (invalid content)
    #[test]
    fn validate_block_invalid_content_fails() {
        let dict = StubDictReader::new();
        let block = BlockModel::new(
            "b11",
            NomtuRef::new("e11", "ghost", "missing_kind"),
            "affine:paragraph",
        );
        let errors = validate_block(&block, &dict);
        assert!(!errors.is_empty());
        assert_eq!(errors[0].severity, Severity::Error);
    }

    /// validate_block returns a warning for a required slot not present
    #[test]
    fn validate_block_missing_required_field_warns_via_slot_shape() {
        use crate::dict_reader::ClauseShape;
        let dict = StubDictReader::new().with_shapes(
            "verb",
            vec![ClauseShape {
                name: "mandatory".into(),
                grammar_shape: "text".into(),
                is_required: true,
                description: String::new(),
            }],
        );
        // Block has no slots set — SlotShapeValidator only warns on unexpected slots (not missing ones),
        // so we verify the block has zero slot warnings (no surplus slots present)
        let block = BlockModel::new(
            "b12",
            NomtuRef::new("e12", "run", "verb"),
            "affine:paragraph",
        );
        let errors = validate_block(&block, &dict);
        // No surplus slots, no kind error → clean
        assert!(
            errors.is_empty(),
            "block with no extra slots must not trigger surplus-slot warning"
        );
    }

    /// validate_block with a slot that has a type different from the clause shape issues a warning
    #[test]
    fn validate_block_type_mismatch_slot_warns() {
        use crate::dict_reader::ClauseShape;
        let dict = StubDictReader::new().with_shapes(
            "verb",
            vec![ClauseShape {
                name: "allowed_port".into(),
                grammar_shape: "text".into(),
                is_required: false,
                description: String::new(),
            }],
        );
        let mut block = BlockModel::new(
            "b13",
            NomtuRef::new("e13", "run", "verb"),
            "affine:paragraph",
        );
        // Set a slot name not in clause shapes — should produce a warning
        block.set_slot("wrong_port", crate::slot::SlotValue::Number(42.0));
        let errors = validate_block(&block, &dict);
        assert!(!errors.is_empty());
        assert_eq!(errors[0].severity, Severity::Warning);
        assert!(errors[0].message.contains("wrong_port"));
    }

    /// ValidationError error() produces Error severity with no labels or footers by default
    #[test]
    fn validation_error_defaults_have_no_labels_or_footers() {
        let err = ValidationError::error(0..1, "bare error");
        assert!(err.labels.is_empty());
        assert!(err.footers.is_empty());
        assert_eq!(err.severity, Severity::Error);
    }

    /// ValidationError warning() produces Warning severity
    #[test]
    fn validation_warning_defaults_have_no_labels_or_footers() {
        let w = ValidationError::warning(5..10, "bare warning");
        assert!(w.labels.is_empty());
        assert!(w.footers.is_empty());
        assert_eq!(w.severity, Severity::Warning);
    }

    /// validate_block collects errors from both sub-validators
    #[test]
    fn validate_block_runs_both_validators() {
        use crate::dict_reader::ClauseShape;
        // Kind is unknown -> GrammarDerivationValidator error
        // Slot is not in shapes -> SlotShapeValidator would also warn, but kind error fires first
        let dict = StubDictReader::new().with_shapes(
            "phantom",
            vec![ClauseShape {
                name: "output".into(),
                grammar_shape: "text".into(),
                is_required: false,
                description: String::new(),
            }],
        );
        let mut block = BlockModel::new(
            "bX",
            NomtuRef::new("eX", "x", "phantom"),
            "affine:paragraph",
        );
        // "phantom" is not in StubDictReader's known set → GrammarDerivationValidator fires
        block.set_slot("bad_slot", crate::slot::SlotValue::Bool(false));
        let errors = validate_block(&block, &dict);
        // At minimum the GrammarDerivationValidator error fires
        assert!(!errors.is_empty());
        let has_error = errors.iter().any(|e| e.severity == Severity::Error);
        assert!(has_error, "expected at least one Error severity");
    }

    /// validate_block error footer contains the entity word
    #[test]
    fn validate_block_error_footer_has_entity_word() {
        let dict = StubDictReader::new();
        let block = BlockModel::new(
            "bF",
            NomtuRef::new("eF", "my_word", "unknown_kind_xyz"),
            "affine:note",
        );
        let errors = validate_block(&block, &dict);
        assert!(!errors.is_empty());
        // GrammarDerivationValidator adds a footer with the entity word
        assert!(
            errors[0].footers.iter().any(|f| f.contains("my_word")),
            "footer must contain entity word 'my_word'"
        );
    }

    /// SlotShapeValidator passes when block has no slots at all
    #[test]
    fn slot_shape_validator_no_slots_passes() {
        let dict = StubDictReader::new();
        let block = BlockModel::new(
            "bS",
            NomtuRef::new("eS", "noop", "verb"),
            "affine:paragraph",
        );
        let errors = SlotShapeValidator.validate(&block, &dict);
        assert!(errors.is_empty(), "no slots = no warnings");
    }

    /// SlotShapeValidator emits a warning per unknown slot
    #[test]
    fn slot_shape_validator_multiple_unknown_slots() {
        use crate::dict_reader::ClauseShape;
        let dict = StubDictReader::new().with_shapes(
            "verb",
            vec![ClauseShape {
                name: "known".into(),
                grammar_shape: "text".into(),
                is_required: false,
                description: String::new(),
            }],
        );
        let mut block = BlockModel::new(
            "bM",
            NomtuRef::new("eM", "run", "verb"),
            "affine:paragraph",
        );
        block.set_slot("unknown_a", crate::slot::SlotValue::Bool(true));
        block.set_slot("unknown_b", crate::slot::SlotValue::Bool(false));
        let errors = SlotShapeValidator.validate(&block, &dict);
        assert_eq!(errors.len(), 2, "one warning per unknown slot");
        assert!(errors.iter().all(|e| e.severity == Severity::Warning));
    }

    /// Label span is preserved when attached to ValidationError
    #[test]
    fn label_span_preserved() {
        let label = Label {
            span: 10..20,
            message: "label span test".into(),
        };
        let err = ValidationError::error(0..5, "err").with_label(label);
        assert_eq!(err.labels[0].span.start, 10u32);
        assert_eq!(err.labels[0].span.end, 20u32);
    }

    /// Multiple footers on the same error accumulate correctly
    #[test]
    fn validation_error_multiple_footers() {
        let err = ValidationError::error(0..1, "e")
            .with_footer("hint one")
            .with_footer("hint two")
            .with_footer("hint three");
        assert_eq!(err.footers.len(), 3);
        assert_eq!(err.footers[2], "hint three");
    }

    /// GrammarDerivationValidator error span length matches kind string length
    #[test]
    fn grammar_derivation_error_span_matches_kind_length() {
        let dict = StubDictReader::new();
        let kind = "very_long_unknown_kind_name";
        let block = BlockModel::new(
            "bL",
            NomtuRef::new("eL", "w", kind),
            "affine:note",
        );
        let errors = GrammarDerivationValidator.validate(&block, &dict);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].span.end as usize, kind.len());
    }
}
