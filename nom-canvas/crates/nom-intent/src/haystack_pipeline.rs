use std::collections::HashMap;

/// The category of work a pipeline component performs.
#[derive(Debug, Clone, PartialEq)]
pub enum ComponentType {
    Retriever,
    Ranker,
    Reader,
    Generator,
    Filter,
}

impl ComponentType {
    pub fn component_type_name(&self) -> &str {
        match self {
            ComponentType::Retriever => "Retriever",
            ComponentType::Ranker => "Ranker",
            ComponentType::Reader => "Reader",
            ComponentType::Generator => "Generator",
            ComponentType::Filter => "Filter",
        }
    }
}

/// A single pipeline stage with a name, type, and key-value config.
#[derive(Debug, Clone)]
pub struct HaystackComponent {
    pub name: String,
    pub component_type: ComponentType,
    pub config: HashMap<String, String>,
}

impl HaystackComponent {
    pub fn new(name: impl Into<String>, component_type: ComponentType) -> Self {
        Self {
            name: name.into(),
            component_type,
            config: HashMap::new(),
        }
    }

    pub fn set_config(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.config.insert(key.into(), value.into());
    }

    pub fn get_config(&self, key: &str) -> Option<&String> {
        self.config.get(key)
    }

    /// Returns true if this component is a retrieval stage (Retriever or Reader).
    pub fn is_retrieval_stage(&self) -> bool {
        matches!(
            self.component_type,
            ComponentType::Retriever | ComponentType::Reader
        )
    }
}

/// An ordered chain of HaystackComponents forming a complete pipeline.
#[derive(Debug, Default)]
pub struct ComponentPipeline {
    pub stages: Vec<HaystackComponent>,
}

impl ComponentPipeline {
    pub fn new() -> Self {
        Self { stages: Vec::new() }
    }

    pub fn add_stage(&mut self, stage: HaystackComponent) {
        self.stages.push(stage);
    }

    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }

    /// Returns references to all retrieval stages in order.
    pub fn retrieval_stages(&self) -> Vec<&HaystackComponent> {
        self.stages
            .iter()
            .filter(|s| s.is_retrieval_stage())
            .collect()
    }

    /// Returns stage names joined by "→".
    pub fn pipeline_signature(&self) -> String {
        self.stages
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>()
            .join("→")
    }
}

/// A single result produced by a pipeline, carrying content, a relevance score, and a source label.
#[derive(Debug, Clone)]
pub struct RankedResult {
    pub content: String,
    pub score: f32,
    pub source: String,
}

/// Ranks and filters a collection of RankedResults.
#[derive(Debug, Default)]
pub struct PipelineRanker;

impl PipelineRanker {
    pub fn new() -> Self {
        Self
    }

    /// Sorts results descending by score.
    pub fn rank(&self, mut results: Vec<RankedResult>) -> Vec<RankedResult> {
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    /// Ranks then returns the top-n results.
    pub fn top_n(&self, results: Vec<RankedResult>, n: usize) -> Vec<RankedResult> {
        self.rank(results).into_iter().take(n).collect()
    }
}

#[cfg(test)]
mod haystack_pipeline_tests {
    use super::*;

    #[test]
    fn component_type_name() {
        assert_eq!(ComponentType::Retriever.component_type_name(), "Retriever");
        assert_eq!(ComponentType::Ranker.component_type_name(), "Ranker");
        assert_eq!(ComponentType::Reader.component_type_name(), "Reader");
        assert_eq!(ComponentType::Generator.component_type_name(), "Generator");
        assert_eq!(ComponentType::Filter.component_type_name(), "Filter");
    }

    #[test]
    fn haystack_component_set_and_get_config() {
        let mut c = HaystackComponent::new("dense", ComponentType::Retriever);
        c.set_config("top_k", "10");
        assert_eq!(c.get_config("top_k"), Some(&"10".to_string()));
        assert_eq!(c.get_config("missing"), None);
    }

    #[test]
    fn haystack_component_is_retrieval_stage_true() {
        let r = HaystackComponent::new("r", ComponentType::Retriever);
        let rd = HaystackComponent::new("rd", ComponentType::Reader);
        assert!(r.is_retrieval_stage());
        assert!(rd.is_retrieval_stage());
    }

    #[test]
    fn haystack_component_is_retrieval_stage_false() {
        let ranker = HaystackComponent::new("ranker", ComponentType::Ranker);
        let gen = HaystackComponent::new("gen", ComponentType::Generator);
        let flt = HaystackComponent::new("flt", ComponentType::Filter);
        assert!(!ranker.is_retrieval_stage());
        assert!(!gen.is_retrieval_stage());
        assert!(!flt.is_retrieval_stage());
    }

    #[test]
    fn component_pipeline_add_and_count() {
        let mut p = ComponentPipeline::new();
        assert_eq!(p.stage_count(), 0);
        p.add_stage(HaystackComponent::new("s1", ComponentType::Retriever));
        p.add_stage(HaystackComponent::new("s2", ComponentType::Ranker));
        assert_eq!(p.stage_count(), 2);
    }

    #[test]
    fn component_pipeline_pipeline_signature() {
        let mut p = ComponentPipeline::new();
        p.add_stage(HaystackComponent::new("retrieve", ComponentType::Retriever));
        p.add_stage(HaystackComponent::new("rank", ComponentType::Ranker));
        p.add_stage(HaystackComponent::new("read", ComponentType::Reader));
        assert_eq!(p.pipeline_signature(), "retrieve→rank→read");
    }

    #[test]
    fn component_pipeline_retrieval_stages() {
        let mut p = ComponentPipeline::new();
        p.add_stage(HaystackComponent::new("r1", ComponentType::Retriever));
        p.add_stage(HaystackComponent::new("ranker", ComponentType::Ranker));
        p.add_stage(HaystackComponent::new("r2", ComponentType::Reader));
        let rs = p.retrieval_stages();
        assert_eq!(rs.len(), 2);
        assert_eq!(rs[0].name, "r1");
        assert_eq!(rs[1].name, "r2");
    }

    #[test]
    fn pipeline_ranker_rank_sorted() {
        let ranker = PipelineRanker::new();
        let results = vec![
            RankedResult { content: "a".into(), score: 0.5, source: "s".into() },
            RankedResult { content: "b".into(), score: 0.9, source: "s".into() },
            RankedResult { content: "c".into(), score: 0.3, source: "s".into() },
        ];
        let ranked = ranker.rank(results);
        assert_eq!(ranked[0].score, 0.9);
        assert_eq!(ranked[1].score, 0.5);
        assert_eq!(ranked[2].score, 0.3);
    }

    #[test]
    fn pipeline_ranker_top_n() {
        let ranker = PipelineRanker::new();
        let results = vec![
            RankedResult { content: "a".into(), score: 0.1, source: "s".into() },
            RankedResult { content: "b".into(), score: 0.8, source: "s".into() },
            RankedResult { content: "c".into(), score: 0.5, source: "s".into() },
            RankedResult { content: "d".into(), score: 0.95, source: "s".into() },
        ];
        let top2 = ranker.top_n(results, 2);
        assert_eq!(top2.len(), 2);
        assert_eq!(top2[0].score, 0.95);
        assert_eq!(top2[1].score, 0.8);
    }
}
