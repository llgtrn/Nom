//! MECE classifier depth and intent classification for nom-intent.

/// Labels for classified user intent.
#[derive(Debug, Clone, PartialEq)]
pub enum IntentLabel {
    Query,
    Command,
    Composition,
    Navigation,
    Configuration,
    Unknown,
}

/// Result of an intent classification.
#[derive(Debug, Clone)]
pub struct ClassifyResult {
    pub label: IntentLabel,
    pub confidence: f32,
    pub explanation: String,
}

impl ClassifyResult {
    pub fn new(label: IntentLabel, confidence: f32, explanation: impl Into<String>) -> Self {
        Self {
            label,
            confidence,
            explanation: explanation.into(),
        }
    }
}

/// Keyword-based intent classifier.
pub struct IntentClassifier;

impl IntentClassifier {
    pub fn new() -> Self {
        Self
    }

    pub fn classify(&self, input: &str) -> ClassifyResult {
        let lower = input.to_lowercase();
        if lower.contains("define") || lower.contains("make") {
            ClassifyResult::new(IntentLabel::Command, 0.9, "matched define/make")
        } else if lower.contains("what") || lower.contains("show") {
            ClassifyResult::new(IntentLabel::Query, 0.85, "matched what/show")
        } else if lower.contains("compose") || lower.contains("render") {
            ClassifyResult::new(IntentLabel::Composition, 0.88, "matched compose/render")
        } else if lower.contains("go to") || lower.contains("open") {
            ClassifyResult::new(IntentLabel::Navigation, 0.82, "matched go to/open")
        } else if lower.contains("set") || lower.contains("config") {
            ClassifyResult::new(IntentLabel::Configuration, 0.8, "matched set/config")
        } else {
            ClassifyResult::new(IntentLabel::Unknown, 0.5, "no keyword matched")
        }
    }

    pub fn is_confident(result: &ClassifyResult, threshold: f32) -> bool {
        result.confidence >= threshold
    }
}

impl Default for IntentClassifier {
    fn default() -> Self {
        Self::new()
    }
}

/// MECE partition: mutually exclusive, collectively exhaustive category set.
pub struct MecePartition {
    pub categories: Vec<String>,
}

impl MecePartition {
    pub fn new(categories: Vec<String>) -> Self {
        Self { categories }
    }

    /// Stub: mutually exclusive if there are no duplicate category names.
    pub fn is_mutually_exclusive(&self) -> bool {
        let mut seen = std::collections::HashSet::new();
        self.categories.iter().all(|c| seen.insert(c.as_str()))
    }

    /// Stub: collectively exhaustive if the number of categories matches `count`.
    pub fn is_collectively_exhaustive(&self, count: usize) -> bool {
        self.categories.len() == count
    }

    /// Stub: coverage score = categories.len() / 6.0, clamped to 1.0.
    pub fn coverage_score(&self) -> f32 {
        (self.categories.len() as f32 / 6.0).min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_command() {
        let c = IntentClassifier::new();
        let r = c.classify("define a new function");
        assert_eq!(r.label, IntentLabel::Command);
        assert!((r.confidence - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn classify_query() {
        let c = IntentClassifier::new();
        let r = c.classify("what is the current state");
        assert_eq!(r.label, IntentLabel::Query);
        assert!((r.confidence - 0.85).abs() < f32::EPSILON);
    }

    #[test]
    fn classify_composition() {
        let c = IntentClassifier::new();
        let r = c.classify("compose a new layout");
        assert_eq!(r.label, IntentLabel::Composition);
        assert!((r.confidence - 0.88).abs() < f32::EPSILON);
    }

    #[test]
    fn classify_unknown() {
        let c = IntentClassifier::new();
        let r = c.classify("blorp snorp");
        assert_eq!(r.label, IntentLabel::Unknown);
        assert!((r.confidence - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn is_confident_true() {
        let r = ClassifyResult::new(IntentLabel::Command, 0.9, "test");
        assert!(IntentClassifier::is_confident(&r, 0.8));
    }

    #[test]
    fn is_confident_false() {
        let r = ClassifyResult::new(IntentLabel::Unknown, 0.5, "test");
        assert!(!IntentClassifier::is_confident(&r, 0.8));
    }

    #[test]
    fn mece_no_duplicates() {
        let p = MecePartition::new(vec![
            "Query".into(),
            "Command".into(),
            "Composition".into(),
        ]);
        assert!(p.is_mutually_exclusive());

        let p_dup = MecePartition::new(vec!["Query".into(), "Query".into()]);
        assert!(!p_dup.is_mutually_exclusive());
    }

    #[test]
    fn mece_coverage_score() {
        let p6 = MecePartition::new(vec![
            "Query".into(),
            "Command".into(),
            "Composition".into(),
            "Navigation".into(),
            "Configuration".into(),
            "Unknown".into(),
        ]);
        assert!((p6.coverage_score() - 1.0).abs() < f32::EPSILON);

        let p3 = MecePartition::new(vec!["Query".into(), "Command".into(), "Composition".into()]);
        assert!((p3.coverage_score() - 0.5).abs() < f32::EPSILON);
    }
}
