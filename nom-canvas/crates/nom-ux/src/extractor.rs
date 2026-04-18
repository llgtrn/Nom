#[derive(Debug, Clone, PartialEq)]
pub enum CorpusSource {
    Motion,
    Dioxus,
    ToolJet,
    DeerFlow,
}

impl CorpusSource {
    pub fn canonical_name(&self) -> &str {
        match self {
            CorpusSource::Motion => "motion",
            CorpusSource::Dioxus => "dioxus",
            CorpusSource::ToolJet => "tooljet",
            CorpusSource::DeerFlow => "deerflow",
        }
    }

    pub fn pattern_kind(&self) -> &str {
        match self {
            CorpusSource::Motion => "web_animation",
            CorpusSource::Dioxus => "cross_platform_ui",
            CorpusSource::ToolJet => "app_composition",
            CorpusSource::DeerFlow => "middleware_flow",
        }
    }
}

#[derive(Debug, Clone)]
pub struct UxPattern {
    pub source: CorpusSource,
    pub pattern_name: String,
    pub description: String,
}

impl UxPattern {
    pub fn new(
        source: CorpusSource,
        pattern_name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            source,
            pattern_name: pattern_name.into(),
            description: description.into(),
        }
    }
}

pub struct UxExtractor {
    patterns: Vec<UxPattern>,
}

impl UxExtractor {
    pub fn new() -> Self {
        Self {
            patterns: Self::seed_patterns(),
        }
    }

    pub fn seed_patterns() -> Vec<UxPattern> {
        vec![
            // Motion — web animation
            UxPattern::new(
                CorpusSource::Motion,
                "spring_animation",
                "Physics-based spring curves for natural motion with configurable stiffness and damping",
            ),
            UxPattern::new(
                CorpusSource::Motion,
                "gesture_recognizer",
                "Unified gesture detection layer mapping pointer/touch events to drag, pan, and tap intents",
            ),
            UxPattern::new(
                CorpusSource::Motion,
                "layout_animation",
                "Automatic interpolation of layout changes between render frames without manual keyframes",
            ),
            // Dioxus — cross-platform UI
            UxPattern::new(
                CorpusSource::Dioxus,
                "reactive_component",
                "Closure-captured signals that re-render only the subtree whose dependencies changed",
            ),
            UxPattern::new(
                CorpusSource::Dioxus,
                "virtual_dom_reconcile",
                "Diff-and-patch pass comparing old and new VNode trees to emit minimal platform mutations",
            ),
            UxPattern::new(
                CorpusSource::Dioxus,
                "platform_target",
                "Single component tree compiled to web, desktop, or mobile via renderer substitution",
            ),
            // ToolJet — app composition
            UxPattern::new(
                CorpusSource::ToolJet,
                "widget_composition",
                "Drag-and-drop palette of atomic widgets assembled into complex app layouts at runtime",
            ),
            UxPattern::new(
                CorpusSource::ToolJet,
                "data_source_binding",
                "Declarative bindings connecting widget properties to query results or REST/DB sources",
            ),
            UxPattern::new(
                CorpusSource::ToolJet,
                "query_engine",
                "Pluggable query runner dispatching SQL, REST, GraphQL, and custom transformations",
            ),
            // DeerFlow — middleware flow
            UxPattern::new(
                CorpusSource::DeerFlow,
                "middleware_chain",
                "Ordered interceptor stack wrapping each flow step for logging, auth, and retry",
            ),
            UxPattern::new(
                CorpusSource::DeerFlow,
                "flow_artifact",
                "Immutable record capturing inputs, outputs, and timing for a completed flow execution",
            ),
            UxPattern::new(
                CorpusSource::DeerFlow,
                "step_instrumentation",
                "Per-step probe points emitting structured events consumed by observability backends",
            ),
        ]
    }

    pub fn patterns_for(&self, source: &CorpusSource) -> Vec<&UxPattern> {
        self.patterns
            .iter()
            .filter(|p| &p.source == source)
            .collect()
    }

    pub fn all_patterns(&self) -> &[UxPattern] {
        &self.patterns
    }
}

impl Default for UxExtractor {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ExtractionReport {
    pub patterns: Vec<UxPattern>,
    pub source_count: usize,
}

impl ExtractionReport {
    pub fn from_extractor(extractor: &UxExtractor) -> Self {
        let patterns = extractor.all_patterns().to_vec();
        let mut seen = std::collections::HashSet::new();
        for p in &patterns {
            seen.insert(p.source.canonical_name());
        }
        Self {
            source_count: seen.len(),
            patterns,
        }
    }

    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }
}

#[cfg(test)]
mod extractor_tests {
    use super::*;

    #[test]
    fn corpus_source_canonical_name() {
        assert_eq!(CorpusSource::Motion.canonical_name(), "motion");
        assert_eq!(CorpusSource::Dioxus.canonical_name(), "dioxus");
        assert_eq!(CorpusSource::ToolJet.canonical_name(), "tooljet");
        assert_eq!(CorpusSource::DeerFlow.canonical_name(), "deerflow");
    }

    #[test]
    fn corpus_source_pattern_kind() {
        assert_eq!(CorpusSource::Motion.pattern_kind(), "web_animation");
        assert_eq!(CorpusSource::Dioxus.pattern_kind(), "cross_platform_ui");
        assert_eq!(CorpusSource::ToolJet.pattern_kind(), "app_composition");
        assert_eq!(CorpusSource::DeerFlow.pattern_kind(), "middleware_flow");
    }

    #[test]
    fn ux_pattern_fields() {
        let p = UxPattern::new(CorpusSource::Motion, "spring_animation", "A spring curve");
        assert_eq!(p.pattern_name, "spring_animation");
        assert_eq!(p.description, "A spring curve");
        assert_eq!(p.source, CorpusSource::Motion);
    }

    #[test]
    fn seed_patterns_returns_at_least_12() {
        let patterns = UxExtractor::seed_patterns();
        assert!(patterns.len() >= 12, "expected >= 12 patterns, got {}", patterns.len());
    }

    #[test]
    fn patterns_for_filters_by_source() {
        let extractor = UxExtractor::new();
        let motion = extractor.patterns_for(&CorpusSource::Motion);
        for p in &motion {
            assert_eq!(p.source, CorpusSource::Motion);
        }
        let dioxus = extractor.patterns_for(&CorpusSource::Dioxus);
        for p in &dioxus {
            assert_eq!(p.source, CorpusSource::Dioxus);
        }
    }

    #[test]
    fn patterns_for_motion_returns_3() {
        let extractor = UxExtractor::new();
        let motion = extractor.patterns_for(&CorpusSource::Motion);
        assert_eq!(motion.len(), 3);
    }

    #[test]
    fn all_patterns_count_matches_seed() {
        let extractor = UxExtractor::new();
        let seed_count = UxExtractor::seed_patterns().len();
        assert_eq!(extractor.all_patterns().len(), seed_count);
    }

    #[test]
    fn extraction_report_source_count_is_4() {
        let extractor = UxExtractor::new();
        let report = ExtractionReport::from_extractor(&extractor);
        assert_eq!(report.source_count, 4);
    }

    #[test]
    fn extraction_report_pattern_count_matches_seed() {
        let extractor = UxExtractor::new();
        let report = ExtractionReport::from_extractor(&extractor);
        let seed_count = UxExtractor::seed_patterns().len();
        assert_eq!(report.pattern_count(), seed_count);
    }
}
