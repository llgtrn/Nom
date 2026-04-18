//! MECE category validator and Dream score gate for concept/app production-readiness.
//!
//! `MeceValidator` checks that a set of categories is Mutually Exclusive
//! (no item appears in more than one category) and that no category is empty.
//!
//! `DreamScore` measures how close an app or concept is to production-ready,
//! returning a weighted score out of 100. Scores ≥ 95 pass the EPIC gate.

use std::collections::HashMap;

// ── MECE ──────────────────────────────────────────────────────────────────────

/// A named category holding a set of items.
#[derive(Debug, Clone)]
pub struct MeceCategory {
    /// Unique identifier for the category.
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// Items belonging to this category.
    pub items: Vec<String>,
}

impl MeceCategory {
    /// Create an empty category.
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            items: Vec::new(),
        }
    }

    /// Builder-style method to append one item.
    pub fn push_item(mut self, item: &str) -> Self {
        self.items.push(item.to_string());
        self
    }

    /// Number of items in the category.
    pub fn item_count(&self) -> usize {
        self.items.len()
    }
}

/// The kind of MECE violation detected.
#[derive(Debug, Clone, PartialEq)]
pub enum ViolationKind {
    /// An item appears in two or more categories (Mutually-Exclusive breach).
    Overlap,
    /// A category has zero items.
    Empty,
    /// An expected item is absent from every category.
    Missing,
}

/// One concrete MECE violation.
#[derive(Debug, Clone)]
pub struct MeceViolation {
    /// What kind of violation this is.
    pub kind: ViolationKind,
    /// The item or category name involved.
    pub item: String,
    /// Human-readable explanation.
    pub description: String,
}

/// Validates a collection of [`MeceCategory`] values for ME + CE properties.
pub struct MeceValidator {
    /// The categories under validation.
    pub categories: Vec<MeceCategory>,
}

impl MeceValidator {
    /// Create a validator with no categories.
    pub fn new() -> Self {
        Self {
            categories: Vec::new(),
        }
    }

    /// Builder-style method to append a category.
    pub fn add_category(mut self, cat: MeceCategory) -> Self {
        self.categories.push(cat);
        self
    }

    /// Run ME + empty-category checks and return all violations found.
    ///
    /// - `ViolationKind::Overlap`: an item string appears in ≥ 2 categories.
    /// - `ViolationKind::Empty`: a category contains zero items.
    pub fn validate(&self) -> Vec<MeceViolation> {
        let mut violations: Vec<MeceViolation> = Vec::new();

        // Empty-category check.
        for cat in &self.categories {
            if cat.items.is_empty() {
                violations.push(MeceViolation {
                    kind: ViolationKind::Empty,
                    item: cat.id.clone(),
                    description: format!("category '{}' has no items", cat.name),
                });
            }
        }

        // Overlap (ME) check: track which categories each item appears in.
        let mut item_to_categories: HashMap<&str, Vec<&str>> = HashMap::new();
        for cat in &self.categories {
            for item in &cat.items {
                item_to_categories
                    .entry(item.as_str())
                    .or_default()
                    .push(cat.id.as_str());
            }
        }

        let mut overlap_items: Vec<(&str, Vec<&str>)> = item_to_categories
            .into_iter()
            .filter(|(_, cats)| cats.len() > 1)
            .collect();

        // Sort for deterministic output.
        overlap_items.sort_by_key(|(item, _)| *item);

        for (item, cats) in overlap_items {
            violations.push(MeceViolation {
                kind: ViolationKind::Overlap,
                item: item.to_string(),
                description: format!(
                    "item '{}' appears in multiple categories: {}",
                    item,
                    cats.join(", ")
                ),
            });
        }

        violations
    }

    /// Return `true` when [`validate`] produces no violations.
    pub fn is_valid(&self) -> bool {
        self.validate().is_empty()
    }

    /// Total item count across all categories (including duplicates).
    pub fn total_items(&self) -> usize {
        self.categories.iter().map(|c| c.items.len()).sum()
    }

    /// Deduplicated item count across all categories.
    pub fn unique_items(&self) -> usize {
        let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for cat in &self.categories {
            for item in &cat.items {
                seen.insert(item.as_str());
            }
        }
        seen.len()
    }
}

impl Default for MeceValidator {
    fn default() -> Self {
        Self::new()
    }
}

// ── Dream score ───────────────────────────────────────────────────────────────

/// Weighted production-readiness score for an app or concept.
///
/// Each dimension is in the range `0.0..=1.0`. Call [`DreamScore::total`] for
/// the weighted aggregate (max 100.0). Scores ≥ [`DreamScore::EPIC_SCORE_THRESHOLD`]
/// satisfy the EPIC production-ready gate.
#[derive(Debug, Clone)]
pub struct DreamScore {
    /// Fraction of code paths covered by tests. Weight: 30.
    pub test_coverage: f32,
    /// Fraction of public API surface that is typed / safe. Weight: 25.
    pub type_safety: f32,
    /// Fraction of public items that have doc-comments. Weight: 20.
    pub doc_coverage: f32,
    /// 1.0 when zero foreign-brand names exist in the codebase. Weight: 15.
    pub no_foreign_names: f32,
    /// 1.0 when the codebase passes the linter with zero warnings. Weight: 10.
    pub clippy_clean: f32,
}

impl DreamScore {
    /// Score threshold that must be reached to pass the EPIC production gate.
    pub const EPIC_SCORE_THRESHOLD: f32 = 95.0;

    /// Create a score with all dimensions set to zero.
    pub fn new() -> Self {
        Self {
            test_coverage: 0.0,
            type_safety: 0.0,
            doc_coverage: 0.0,
            no_foreign_names: 0.0,
            clippy_clean: 0.0,
        }
    }

    /// Set the test-coverage dimension.
    pub fn with_test_coverage(mut self, v: f32) -> Self {
        self.test_coverage = v.clamp(0.0, 1.0);
        self
    }

    /// Set the type-safety dimension.
    pub fn with_type_safety(mut self, v: f32) -> Self {
        self.type_safety = v.clamp(0.0, 1.0);
        self
    }

    /// Set the doc-coverage dimension.
    pub fn with_doc_coverage(mut self, v: f32) -> Self {
        self.doc_coverage = v.clamp(0.0, 1.0);
        self
    }

    /// Set the no-foreign-names dimension (1.0 = zero violations).
    pub fn with_no_foreign_names(mut self, v: f32) -> Self {
        self.no_foreign_names = v.clamp(0.0, 1.0);
        self
    }

    /// Set the linter-clean dimension.
    pub fn with_clippy_clean(mut self, v: bool) -> Self {
        self.clippy_clean = if v { 1.0 } else { 0.0 };
        self
    }

    /// Weighted aggregate score (max 100.0).
    ///
    /// Weights: test_coverage×30 + type_safety×25 + doc_coverage×20
    ///          + no_foreign_names×15 + clippy_clean×10
    pub fn total(&self) -> f32 {
        self.test_coverage * 30.0
            + self.type_safety * 25.0
            + self.doc_coverage * 20.0
            + self.no_foreign_names * 15.0
            + self.clippy_clean * 10.0
    }

    /// Return `true` when the total meets the EPIC production gate.
    pub fn is_epic(&self) -> bool {
        self.total() >= Self::EPIC_SCORE_THRESHOLD
    }
}

impl Default for DreamScore {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── test 1: MeceCategory builder ─────────────────────────────────────────

    #[test]
    fn mece_category_new_and_push_item() {
        let cat = MeceCategory::new("cat-a", "Category A")
            .push_item("alpha")
            .push_item("beta");

        assert_eq!(cat.id, "cat-a");
        assert_eq!(cat.name, "Category A");
        assert_eq!(cat.item_count(), 2);
        assert!(cat.items.contains(&"alpha".to_string()));
        assert!(cat.items.contains(&"beta".to_string()));
    }

    // ── test 2: valid validator (no overlaps, no empty categories) ────────────

    #[test]
    fn mece_validator_valid_no_violations() {
        let validator = MeceValidator::new()
            .add_category(
                MeceCategory::new("a", "Alpha")
                    .push_item("foo")
                    .push_item("bar"),
            )
            .add_category(
                MeceCategory::new("b", "Beta")
                    .push_item("baz")
                    .push_item("qux"),
            );

        let violations = validator.validate();
        assert!(
            violations.is_empty(),
            "expected no violations, got: {violations:?}"
        );
    }

    // ── test 3: overlap detected ──────────────────────────────────────────────

    #[test]
    fn mece_validator_overlap_detected() {
        let validator = MeceValidator::new()
            .add_category(MeceCategory::new("a", "Alpha").push_item("foo").push_item("bar"))
            .add_category(MeceCategory::new("b", "Beta").push_item("foo").push_item("baz"));

        let violations = validator.validate();
        let overlaps: Vec<&MeceViolation> = violations
            .iter()
            .filter(|v| v.kind == ViolationKind::Overlap)
            .collect();

        assert_eq!(overlaps.len(), 1, "expected exactly one overlap: {violations:?}");
        assert_eq!(overlaps[0].item, "foo");
    }

    // ── test 4: empty category detected ──────────────────────────────────────

    #[test]
    fn mece_validator_empty_detected() {
        let validator = MeceValidator::new()
            .add_category(MeceCategory::new("a", "Alpha").push_item("foo"))
            .add_category(MeceCategory::new("b", "Empty Beta"));

        let violations = validator.validate();
        let empties: Vec<&MeceViolation> = violations
            .iter()
            .filter(|v| v.kind == ViolationKind::Empty)
            .collect();

        assert_eq!(empties.len(), 1, "expected exactly one empty violation: {violations:?}");
        assert_eq!(empties[0].item, "b");
    }

    // ── test 5: is_valid on clean validator ───────────────────────────────────

    #[test]
    fn mece_is_valid_returns_true_for_clean_validator() {
        let validator = MeceValidator::new()
            .add_category(MeceCategory::new("x", "X").push_item("one"))
            .add_category(MeceCategory::new("y", "Y").push_item("two"));

        assert!(validator.is_valid());
    }

    // ── test 6: perfect dream score = 100.0 ──────────────────────────────────

    #[test]
    fn dream_score_total_perfect_is_100() {
        let score = DreamScore::new()
            .with_test_coverage(1.0)
            .with_type_safety(1.0)
            .with_doc_coverage(1.0)
            .with_no_foreign_names(1.0)
            .with_clippy_clean(true);

        let total = score.total();
        assert!(
            (total - 100.0).abs() < f32::EPSILON,
            "expected 100.0, got {total}"
        );
    }

    // ── test 7: is_epic threshold ─────────────────────────────────────────────

    #[test]
    fn dream_score_is_epic_threshold() {
        // Score of 100 → epic.
        let perfect = DreamScore::new()
            .with_test_coverage(1.0)
            .with_type_safety(1.0)
            .with_doc_coverage(1.0)
            .with_no_foreign_names(1.0)
            .with_clippy_clean(true);
        assert!(perfect.is_epic(), "perfect score must be epic");

        // Score of 0 → not epic.
        let zero = DreamScore::new();
        assert!(!zero.is_epic(), "zero score must not be epic");
    }

    // ── test 8: partial dream score ───────────────────────────────────────────

    #[test]
    fn dream_score_partial_test_coverage_only() {
        let score = DreamScore::new().with_test_coverage(0.5);

        // 0.5 * 30 = 15.0; all other dimensions are 0.
        let total = score.total();
        assert!(
            (total - 15.0).abs() < f32::EPSILON,
            "expected 15.0, got {total}"
        );
        assert!(!score.is_epic());
    }
}
