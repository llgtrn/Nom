//! Annotator trait + `Annotation` typed-key map (CoreNLP-inspired, doc 10 §E W1).
//!
//! Mines Stanford CoreNLP's `Annotator` interface ([src/edu/stanford/nlp/pipeline/
//! Annotator.java:54](https://github.com/stanfordnlp/CoreNLP)) — each stage of the
//! extraction pipeline declares the keys it **requires** upstream plus the keys it
//! **satisfies** (writes), and `annotate()` does the work.
//!
//! The contract:
//!
//! ```rust,ignore
//! pub trait Annotator {
//!     fn name(&self) -> &'static str;
//!     fn requires(&self) -> &'static [&'static str];
//!     fn requirements_satisfied(&self) -> &'static [&'static str];
//!     fn annotate(&self, ann: &mut Annotation) -> AnnotatorResult;
//! }
//! ```
//!
//! This fixes the current opaque ordering in `extract.rs` / `scan.rs` — downstream
//! code can ask the pipeline "what ran on this source?" and get a declarative answer
//! instead of a hardcoded call chain. Also lets the MECE validator + glass-box
//! report surface which stage emitted which atom.
//!
//! **Not adopted** from CoreNLP (per doc 10 §E "Don't adopt"): JVM runtime, Java-
//! properties-driven pipeline config (Nom uses hash-keyed `.nomtu` profiles),
//! statistical-ML taggers (LLM-as-oracle stays in `nom-intent` M8), constituency
//! parsing (dep parse is enough).

use std::collections::HashMap;

use thiserror::Error;

/// Errors from running an `Annotator`.
#[derive(Debug, Error)]
pub enum AnnotatorError {
    #[error("required key {key:?} missing (annotator {annotator} declared requires=[..,{key}])")]
    MissingRequirement { annotator: &'static str, key: &'static str },
    #[error("annotator {annotator} failed: {reason}")]
    Failed { annotator: &'static str, reason: String },
}

pub type AnnotatorResult = Result<(), AnnotatorError>;

/// Typed-key payload that flows through an `AnnotationPipeline`. Maps
/// CoreNLP's `CoreMap` / `Annotation` onto a simple `String -> Vec<u8>`
/// serialization so each stage writes self-describing bytes (typically JSON).
///
/// Design choice: `Vec<u8>` not `serde_json::Value` so stages can carry raw
/// bytes for media inputs without forcing a JSON round-trip. Stages that
/// want structured data just `serde_json::from_slice` their key.
#[derive(Debug, Default, Clone)]
pub struct Annotation {
    slots: HashMap<String, Vec<u8>>,
    /// Ordered list of annotator names that have run (audit trail).
    ran: Vec<&'static str>,
}

impl Annotation {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set<K: Into<String>>(&mut self, key: K, value: Vec<u8>) {
        self.slots.insert(key.into(), value);
    }

    pub fn get(&self, key: &str) -> Option<&[u8]> {
        self.slots.get(key).map(|v| v.as_slice())
    }

    pub fn has(&self, key: &str) -> bool {
        self.slots.contains_key(key)
    }

    /// Keys currently present, sorted for deterministic diagnostics.
    pub fn keys(&self) -> Vec<&str> {
        let mut k: Vec<&str> = self.slots.keys().map(|s| s.as_str()).collect();
        k.sort();
        k
    }

    /// Ordered audit trail of which annotators have run on this annotation.
    pub fn ran(&self) -> &[&'static str] {
        &self.ran
    }

    fn mark_ran(&mut self, name: &'static str) {
        self.ran.push(name);
    }
}

/// A pipeline stage. Mirrors CoreNLP's `Annotator` interface verbatim —
/// only `annotate()` does work; the other three methods declare the stage's
/// metadata so a pipeline can order stages and diagnose missing deps without
/// running them.
pub trait Annotator {
    /// Short stable name, used for error messages + audit trail.
    fn name(&self) -> &'static str;

    /// Keys this stage needs present on the `Annotation` before it can run.
    fn requires(&self) -> &'static [&'static str];

    /// Keys this stage writes to the `Annotation` on success.
    fn requirements_satisfied(&self) -> &'static [&'static str];

    /// Do the work. Precondition-checked by `AnnotationPipeline::run`; if
    /// `requires()` is non-empty the pipeline verifies all keys are present
    /// before calling this method.
    fn annotate(&self, ann: &mut Annotation) -> AnnotatorResult;
}

/// Ordered list of `Annotator`s sharing one `Annotation`. Mirrors CoreNLP's
/// `AnnotationPipeline` (AnnotationPipeline.java:27).
///
/// This week-1 wedge does NOT auto-order stages by their `requires()` /
/// `requirements_satisfied()` declarations — the pipeline runs them in
/// insertion order and errors if a stage's requires aren't satisfied yet.
/// Auto-ordering is a later slice once more annotators exist (today only
/// the pipeline shape + precondition check matter).
#[derive(Default)]
pub struct AnnotationPipeline {
    stages: Vec<Box<dyn Annotator>>,
}

impl AnnotationPipeline {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, stage: Box<dyn Annotator>) -> &mut Self {
        self.stages.push(stage);
        self
    }

    /// Number of stages in the pipeline.
    pub fn len(&self) -> usize {
        self.stages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.stages.is_empty()
    }

    /// Run every stage in insertion order on `ann`. Before each stage fires
    /// the pipeline verifies every `requires()` key is present on the
    /// annotation; if any is missing the stage is skipped and
    /// `AnnotatorError::MissingRequirement` is returned.
    pub fn run(&self, ann: &mut Annotation) -> AnnotatorResult {
        for stage in &self.stages {
            for req in stage.requires() {
                if !ann.has(req) {
                    return Err(AnnotatorError::MissingRequirement {
                        annotator: stage.name(),
                        key: req,
                    });
                }
            }
            stage.annotate(ann)?;
            ann.mark_ran(stage.name());
        }
        Ok(())
    }
}

// ── Concrete annotators wrapping existing nom-extract APIs ────────────

/// Wraps [`parse_and_extract`](crate::extract::parse_and_extract) as an
/// `Annotator` stage. Requires `source` (bytes = file contents, UTF-8)
/// and `language` (bytes = short language name, e.g. `rust`, `python`).
/// Satisfies `entities` (bytes = JSON array of `UirEntity`).
///
/// This closes CoreNLP W1b: the existing `extract_entities` pipeline is
/// now introspectable + composable via the `Annotator` trait, with the
/// same call semantics as the direct function.
pub struct ParseAndExtractAnnotator;

impl Annotator for ParseAndExtractAnnotator {
    fn name(&self) -> &'static str {
        "parse_and_extract"
    }

    fn requires(&self) -> &'static [&'static str] {
        &["source", "language", "file_path"]
    }

    fn requirements_satisfied(&self) -> &'static [&'static str] {
        &["entities"]
    }

    fn annotate(&self, ann: &mut Annotation) -> AnnotatorResult {
        let source_bytes = ann.get("source").unwrap_or(b"");
        let language_bytes = ann.get("language").unwrap_or(b"");
        let file_path_bytes = ann.get("file_path").unwrap_or(b"");

        let source = std::str::from_utf8(source_bytes).map_err(|e| AnnotatorError::Failed {
            annotator: self.name(),
            reason: format!("source is not valid UTF-8: {e}"),
        })?;
        let language = std::str::from_utf8(language_bytes).map_err(|e| AnnotatorError::Failed {
            annotator: self.name(),
            reason: format!("language is not valid UTF-8: {e}"),
        })?;
        let file_path = std::str::from_utf8(file_path_bytes).map_err(|e| {
            AnnotatorError::Failed {
                annotator: self.name(),
                reason: format!("file_path is not valid UTF-8: {e}"),
            }
        })?;

        let entities = crate::extract::parse_and_extract(source, file_path, language)
            .map_err(|e| AnnotatorError::Failed {
                annotator: self.name(),
                reason: format!("parse_and_extract: {e:#}"),
            })?;

        let encoded = serde_json::to_vec(&entities).map_err(|e| AnnotatorError::Failed {
            annotator: self.name(),
            reason: format!("serialize entities: {e}"),
        })?;
        ann.set("entities", encoded);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TokenizeAnnotator;
    impl Annotator for TokenizeAnnotator {
        fn name(&self) -> &'static str {
            "tokenize"
        }
        fn requires(&self) -> &'static [&'static str] {
            &["source"]
        }
        fn requirements_satisfied(&self) -> &'static [&'static str] {
            &["tokens"]
        }
        fn annotate(&self, ann: &mut Annotation) -> AnnotatorResult {
            let src = ann.get("source").unwrap_or(b"");
            let tokens_count = std::str::from_utf8(src)
                .map(|s| s.split_whitespace().count())
                .unwrap_or(0);
            ann.set("tokens", tokens_count.to_string().into_bytes());
            Ok(())
        }
    }

    struct AtomsAnnotator;
    impl Annotator for AtomsAnnotator {
        fn name(&self) -> &'static str {
            "atoms"
        }
        fn requires(&self) -> &'static [&'static str] {
            &["tokens"]
        }
        fn requirements_satisfied(&self) -> &'static [&'static str] {
            &["atoms"]
        }
        fn annotate(&self, ann: &mut Annotation) -> AnnotatorResult {
            ann.set("atoms", b"[]".to_vec());
            Ok(())
        }
    }

    #[test]
    fn pipeline_runs_stages_in_order_when_deps_satisfied() {
        let mut ann = Annotation::new();
        ann.set("source", b"hello world foo".to_vec());
        let mut p = AnnotationPipeline::new();
        p.add(Box::new(TokenizeAnnotator));
        p.add(Box::new(AtomsAnnotator));
        p.run(&mut ann).expect("pipeline should succeed");
        assert_eq!(
            ann.get("tokens").unwrap(),
            b"3",
            "tokenize should count 3 whitespace tokens"
        );
        assert_eq!(ann.get("atoms").unwrap(), b"[]");
        assert_eq!(ann.ran(), &["tokenize", "atoms"]);
    }

    #[test]
    fn pipeline_errors_when_requirement_missing() {
        let mut ann = Annotation::new();
        // No "source" → tokenize can't run.
        let mut p = AnnotationPipeline::new();
        p.add(Box::new(TokenizeAnnotator));
        let err = p.run(&mut ann).expect_err("must fail with missing requirement");
        match err {
            AnnotatorError::MissingRequirement { annotator, key } => {
                assert_eq!(annotator, "tokenize");
                assert_eq!(key, "source");
            }
            _ => panic!("wrong error variant: {err:?}"),
        }
    }

    #[test]
    fn annotation_map_keys_are_sorted() {
        let mut ann = Annotation::new();
        ann.set("z", b"1".to_vec());
        ann.set("a", b"2".to_vec());
        ann.set("m", b"3".to_vec());
        assert_eq!(ann.keys(), vec!["a", "m", "z"]);
    }

    #[test]
    fn ran_audit_trail_records_order() {
        let mut ann = Annotation::new();
        ann.set("source", b"one two".to_vec());
        let mut p = AnnotationPipeline::new();
        p.add(Box::new(TokenizeAnnotator));
        p.add(Box::new(AtomsAnnotator));
        p.run(&mut ann).unwrap();
        assert_eq!(ann.ran(), &["tokenize", "atoms"]);
    }

    #[test]
    fn requires_and_satisfied_are_declarative() {
        let t = TokenizeAnnotator;
        let a = AtomsAnnotator;
        assert_eq!(t.requires(), &["source"]);
        assert_eq!(t.requirements_satisfied(), &["tokens"]);
        assert_eq!(a.requires(), &["tokens"]);
        assert_eq!(a.requirements_satisfied(), &["atoms"]);
        // AtomsAnnotator's requires overlap with TokenizeAnnotator's satisfied
        // → pipeline is valid when they run in that order.
        assert_eq!(t.requirements_satisfied()[0], a.requires()[0]);
    }

    // ── W1b: ParseAndExtractAnnotator wraps real nom-extract code ────────

    #[test]
    fn parse_and_extract_annotator_declares_contract() {
        let a = ParseAndExtractAnnotator;
        assert_eq!(a.name(), "parse_and_extract");
        assert_eq!(a.requires(), &["source", "language", "file_path"]);
        assert_eq!(a.requirements_satisfied(), &["entities"]);
    }

    #[test]
    fn parse_and_extract_annotator_rejects_missing_source() {
        let mut ann = Annotation::new();
        ann.set("language", b"rust".to_vec());
        ann.set("file_path", b"x.rs".to_vec());
        let mut p = AnnotationPipeline::new();
        p.add(Box::new(ParseAndExtractAnnotator));
        let err = p.run(&mut ann).expect_err("must fail without source");
        match err {
            AnnotatorError::MissingRequirement { annotator, key } => {
                assert_eq!(annotator, "parse_and_extract");
                assert_eq!(key, "source");
            }
            _ => panic!("wrong variant: {err:?}"),
        }
    }

    #[test]
    fn parse_and_extract_annotator_emits_entities_json() {
        let mut ann = Annotation::new();
        ann.set("source", b"fn greet(name: &str) -> String { name.to_string() }".to_vec());
        ann.set("language", b"rust".to_vec());
        ann.set("file_path", b"greet.rs".to_vec());

        let mut p = AnnotationPipeline::new();
        p.add(Box::new(ParseAndExtractAnnotator));
        p.run(&mut ann).expect("parse_and_extract should succeed on valid Rust");

        let entities_bytes = ann
            .get("entities")
            .expect("entities key must be written");
        let parsed: serde_json::Value =
            serde_json::from_slice(entities_bytes).expect("entities must be JSON");
        let arr = parsed.as_array().expect("entities must be a JSON array");
        assert!(!arr.is_empty(), "greet() should produce at least one entity");
        assert_eq!(ann.ran(), &["parse_and_extract"]);
    }
}
