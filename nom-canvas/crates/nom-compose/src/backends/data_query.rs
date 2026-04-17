//! Data query backend (natural language → grounded SQL/Cypher).
//!
//! Consumes a `SemanticModel` + natural-language query; emits a grounded
//! query string + optional visualization hint.  Pipeline stages + vector
//! retrieval live in runtime crates; this module is the spec + stages enum.
#![deny(unsafe_code)]

use crate::backend_trait::{CompositionBackend, ComposeSpec, ComposeOutput, ComposeError, InterruptFlag, ProgressSink};
use crate::kind::NomKind;
use crate::semantic::SemanticModel;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryIntent { Query, Chart, Insight }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryLanguage { Sql, Cypher, PromQL }

#[derive(Clone, Debug, PartialEq)]
pub struct QuerySpec {
    pub natural_language: String,
    pub model: SemanticModel,
    pub target_language: QueryLanguage,
    pub max_correction_iterations: u8,
    pub intent: Option<QueryIntent>,
}

impl QuerySpec {
    pub fn new(natural_language: impl Into<String>, model: SemanticModel) -> Self {
        Self {
            natural_language: natural_language.into(),
            model,
            target_language: QueryLanguage::Sql,
            max_correction_iterations: 3,
            intent: None,
        }
    }
    pub fn with_language(mut self, lang: QueryLanguage) -> Self { self.target_language = lang; self }
    pub fn with_intent(mut self, intent: QueryIntent) -> Self { self.intent = Some(intent); self }
    pub fn with_max_iterations(mut self, n: u8) -> Self { self.max_correction_iterations = n; self }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PipelineStage {
    IntentClassification,
    VectorRetrieval,
    GroundedGeneration,
    CorrectionLoop,
    Execution,
}

impl PipelineStage {
    pub const ORDER: &'static [PipelineStage] = &[
        Self::IntentClassification,
        Self::VectorRetrieval,
        Self::GroundedGeneration,
        Self::CorrectionLoop,
        Self::Execution,
    ];
    pub fn index(self) -> usize {
        match self {
            Self::IntentClassification => 0,
            Self::VectorRetrieval => 1,
            Self::GroundedGeneration => 2,
            Self::CorrectionLoop => 3,
            Self::Execution => 4,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::IntentClassification => "intent",
            Self::VectorRetrieval => "retrieval",
            Self::GroundedGeneration => "generation",
            Self::CorrectionLoop => "correction",
            Self::Execution => "execution",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueryResult {
    pub intent: QueryIntent,
    pub generated_query: String,
    pub correction_iterations: u8,
    pub stage_durations_ms: Vec<(PipelineStage, u64)>,
}

#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("natural_language query must not be empty")]
    EmptyQuery,
    #[error("semantic model is empty — at least one entity required")]
    EmptyModel,
    #[error("max_correction_iterations must be 1..=10; got {0}")]
    InvalidMaxIterations(u8),
}

pub fn validate(spec: &QuerySpec) -> Result<(), QueryError> {
    if spec.natural_language.trim().is_empty() { return Err(QueryError::EmptyQuery); }
    if spec.model.entities.is_empty() { return Err(QueryError::EmptyModel); }
    if spec.max_correction_iterations == 0 || spec.max_correction_iterations > 10 {
        return Err(QueryError::InvalidMaxIterations(spec.max_correction_iterations));
    }
    Ok(())
}

pub struct StubDataQueryBackend;

impl CompositionBackend for StubDataQueryBackend {
    fn kind(&self) -> NomKind { NomKind::DataQuery }
    fn name(&self) -> &str { "stub-data-query" }
    fn compose(&self, _spec: &ComposeSpec, _progress: &dyn ProgressSink, _interrupt: &InterruptFlag) -> Result<ComposeOutput, ComposeError> {
        Ok(ComposeOutput { bytes: b"SELECT 1".to_vec(), mime_type: "text/plain".to_string(), cost_cents: 0 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::{SemanticEntity, SemanticModel};

    fn model_with_entity() -> SemanticModel {
        let mut m = SemanticModel::new();
        m.add_entity(SemanticEntity::new("Order", "orders", "orders entity")).unwrap();
        m
    }

    #[test]
    fn new_defaults_sql_max3_no_intent() {
        let spec = QuerySpec::new("show me orders", model_with_entity());
        assert_eq!(spec.target_language, QueryLanguage::Sql);
        assert_eq!(spec.max_correction_iterations, 3);
        assert!(spec.intent.is_none());
    }

    #[test]
    fn builder_chain_sets_all_fields() {
        let spec = QuerySpec::new("revenue by region", model_with_entity())
            .with_language(QueryLanguage::Cypher)
            .with_intent(QueryIntent::Chart)
            .with_max_iterations(5);
        assert_eq!(spec.target_language, QueryLanguage::Cypher);
        assert_eq!(spec.intent, Some(QueryIntent::Chart));
        assert_eq!(spec.max_correction_iterations, 5);
    }

    #[test]
    fn pipeline_stage_order_has_5_entries() {
        assert_eq!(PipelineStage::ORDER.len(), 5);
    }

    #[test]
    fn pipeline_stage_indices_are_0_to_4() {
        assert_eq!(PipelineStage::IntentClassification.index(), 0);
        assert_eq!(PipelineStage::VectorRetrieval.index(), 1);
        assert_eq!(PipelineStage::GroundedGeneration.index(), 2);
        assert_eq!(PipelineStage::CorrectionLoop.index(), 3);
        assert_eq!(PipelineStage::Execution.index(), 4);
    }

    #[test]
    fn pipeline_stage_names_are_str() {
        assert_eq!(PipelineStage::IntentClassification.name(), "intent");
        assert_eq!(PipelineStage::VectorRetrieval.name(), "retrieval");
        assert_eq!(PipelineStage::GroundedGeneration.name(), "generation");
        assert_eq!(PipelineStage::CorrectionLoop.name(), "correction");
        assert_eq!(PipelineStage::Execution.name(), "execution");
    }

    #[test]
    fn query_result_fields_accessible() {
        let r = QueryResult {
            intent: QueryIntent::Insight,
            generated_query: "SELECT * FROM orders".into(),
            correction_iterations: 1,
            stage_durations_ms: vec![(PipelineStage::Execution, 42)],
        };
        assert_eq!(r.intent, QueryIntent::Insight);
        assert_eq!(r.correction_iterations, 1);
        assert_eq!(r.stage_durations_ms.len(), 1);
    }

    #[test]
    fn validate_ok_with_non_empty_model() {
        let spec = QuerySpec::new("show me orders", model_with_entity());
        assert!(validate(&spec).is_ok());
    }

    #[test]
    fn validate_empty_nl_returns_empty_query() {
        let spec = QuerySpec::new("   ", model_with_entity());
        assert!(matches!(validate(&spec), Err(QueryError::EmptyQuery)));
    }

    #[test]
    fn validate_empty_model_returns_empty_model() {
        let spec = QuerySpec::new("show me orders", SemanticModel::new());
        assert!(matches!(validate(&spec), Err(QueryError::EmptyModel)));
    }

    #[test]
    fn validate_invalid_max_iter_zero() {
        let spec = QuerySpec::new("show me orders", model_with_entity())
            .with_max_iterations(0);
        assert!(matches!(validate(&spec), Err(QueryError::InvalidMaxIterations(0))));
    }

    #[test]
    fn validate_invalid_max_iter_eleven() {
        let spec = QuerySpec::new("show me orders", model_with_entity())
            .with_max_iterations(11);
        assert!(matches!(validate(&spec), Err(QueryError::InvalidMaxIterations(11))));
    }

    #[test]
    fn stub_backend_kind_and_name() {
        let b = StubDataQueryBackend;
        assert_eq!(b.kind(), NomKind::DataQuery);
        assert_eq!(b.name(), "stub-data-query");
    }
}
