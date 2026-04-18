//! Integration tests: CompilePipeline + CorpusOrchestrator coverage.

use nom_concept::ingest::{CorpusBatchResult, CorpusEcosystem, CorpusOrchestrator};
use nom_concept::{CompileError, CompileInput, CompilePipeline};

// ── CompilePipeline tests ─────────────────────────────────────────────────────

/// 1. Empty source string returns Err at the Parse stage.
#[test]
fn pipeline_compile_empty_source() {
    let pipeline = CompilePipeline::new();
    let input = CompileInput { source: "".into(), entry_name: "x".into() };
    let result = pipeline.compile(input);
    assert!(result.is_err(), "empty source must fail");
}

/// 2. Whitespace-only source returns Err (trim check in pipeline).
#[test]
fn pipeline_compile_whitespace_only() {
    let pipeline = CompilePipeline::new();
    let input = CompileInput { source: "   \t\n  ".into(), entry_name: "ws".into() };
    let result = pipeline.compile(input);
    assert!(result.is_err(), "whitespace-only source must fail");
}

/// 3. A valid short source compiles successfully.
#[test]
fn pipeline_compile_short_source() {
    let pipeline = CompilePipeline::new();
    let input = CompileInput { source: "x = 1".into(), entry_name: "x".into() };
    let result = pipeline.compile(input);
    assert!(result.is_ok(), "short valid source must compile: {:?}", result.err());
    let out = result.unwrap();
    assert!(!out.ir_text.is_empty(), "ir_text must not be empty");
}

/// 4. Successful compilation output has non-empty binary_bytes.
#[test]
fn pipeline_compile_output_has_bytes() {
    let pipeline = CompilePipeline::new();
    let input = CompileInput { source: "define foo that 42".into(), entry_name: "foo".into() };
    let out = pipeline.compile(input).unwrap();
    assert!(!out.binary_bytes.is_empty(), "binary_bytes must be non-empty");
}

/// 5. stage_names() returns stages in the documented order.
#[test]
fn pipeline_compile_stage_order() {
    let names = CompilePipeline::stage_names();
    assert!(names.len() >= 3, "must have at least 3 stage names");
    // parse always comes before the codegen step
    let parse_pos = names.iter().position(|&n| n == "parse").expect("parse stage present");
    let codegen_pos = names.iter().position(|&n| n == "codegen").expect("codegen stage present");
    assert!(parse_pos < codegen_pos, "parse must precede codegen in stage order");
}

/// 6. A compile error carries a non-empty message field.
#[test]
fn pipeline_error_contains_message() {
    let pipeline = CompilePipeline::new();
    let input = CompileInput { source: "".into(), entry_name: "empty".into() };
    let err: CompileError = pipeline.compile(input).unwrap_err();
    assert!(!err.message.is_empty(), "error message must not be empty");
}

// ── CorpusOrchestrator tests ──────────────────────────────────────────────────

/// 7. plan_batches() returns exactly 4 batches.
#[test]
fn corpus_plan_batches_count() {
    let orch = CorpusOrchestrator::new(400);
    let batches = orch.plan_batches();
    assert_eq!(batches.len(), 4, "plan_batches must return 4 batches");
}

/// 8. Each batch returned by plan_batches() has max_entries > 0.
#[test]
fn corpus_batch_has_entries() {
    let orch = CorpusOrchestrator::new(400);
    let batches = orch.plan_batches();
    for batch in &batches {
        assert!(batch.max_entries > 0, "each batch must have max_entries > 0");
    }
}

/// 9. The 4 planned batches cover 4 distinct ecosystems.
#[test]
fn corpus_batch_ecosystems_distinct() {
    let orch = CorpusOrchestrator::new(400);
    let batches = orch.plan_batches();
    let ecosystems: Vec<&CorpusEcosystem> = batches.iter().map(|b| &b.ecosystem).collect();
    // All 4 expected variants must be present.
    assert!(ecosystems.contains(&&CorpusEcosystem::PyPi), "PyPi must be present");
    assert!(ecosystems.contains(&&CorpusEcosystem::GitHub), "GitHub must be present");
    assert!(ecosystems.contains(&&CorpusEcosystem::RustCrates), "RustCrates must be present");
    assert!(ecosystems.contains(&&CorpusEcosystem::NpmPackages), "NpmPackages must be present");
    // All 4 are distinct (no duplicates).
    let mut seen = std::collections::HashSet::new();
    for eco in &ecosystems {
        let name = eco.name();
        assert!(seen.insert(name), "duplicate ecosystem: {}", name);
    }
}

/// 10. A CorpusBatchResult with 10 processed repos and no errors has success_rate 1.0
///     and the repos_processed total equals 10.
#[test]
fn corpus_batch_result_can_succeed() {
    let result = CorpusBatchResult {
        ecosystem: "pypi".into(),
        repos_processed: 10,
        entries_ingested: 100,
        errors: vec![],
    };
    assert_eq!(result.repos_processed, 10, "repos_processed must be 10");
    let rate = result.success_rate();
    assert!(
        (rate - 1.0).abs() < f64::EPSILON,
        "success_rate must be 1.0 when errors is empty, got {}",
        rate
    );
}
