/// AI-invokes-compiler loop (verify→build→bench→flow) — §5.19 from the Nom roadmap.
///
/// The authoring workflow is a deterministic loop where the compiler acts as an oracle.
/// Each cycle advances through four stages; the loop repeats until all stages succeed.

// ---------------------------------------------------------------------------
// Stage
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompilerLoopStage {
    Verify,
    Build,
    Bench,
    Flow,
}

impl CompilerLoopStage {
    pub fn stage_name(&self) -> &str {
        match self {
            CompilerLoopStage::Verify => "verify",
            CompilerLoopStage::Build => "build",
            CompilerLoopStage::Bench => "bench",
            CompilerLoopStage::Flow => "flow",
        }
    }

    pub fn next(&self) -> Option<CompilerLoopStage> {
        match self {
            CompilerLoopStage::Verify => Some(CompilerLoopStage::Build),
            CompilerLoopStage::Build => Some(CompilerLoopStage::Bench),
            CompilerLoopStage::Bench => Some(CompilerLoopStage::Flow),
            CompilerLoopStage::Flow => None,
        }
    }
}

// ---------------------------------------------------------------------------
// LoopIteration
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct LoopIteration {
    pub stage: CompilerLoopStage,
    pub input: String,
    pub output: String,
    pub success: bool,
    pub duration_ms: u64,
}

impl LoopIteration {
    pub fn new(
        stage: CompilerLoopStage,
        input: impl Into<String>,
        success: bool,
        duration_ms: u64,
    ) -> Self {
        let output = if success { "ok" } else { "error" }.to_string();
        Self {
            stage,
            input: input.into(),
            output,
            success,
            duration_ms,
        }
    }
}

// ---------------------------------------------------------------------------
// CompilerLoopConfig
// ---------------------------------------------------------------------------

pub struct CompilerLoopConfig {
    pub max_iterations: u32,
    pub stop_on_failure: bool,
}

impl CompilerLoopConfig {
    pub fn default() -> Self {
        Self {
            max_iterations: 4,
            stop_on_failure: true,
        }
    }
}

// ---------------------------------------------------------------------------
// AiCompilerLoop
// ---------------------------------------------------------------------------

pub struct AiCompilerLoop {
    pub config: CompilerLoopConfig,
}

impl AiCompilerLoop {
    pub fn new(config: CompilerLoopConfig) -> Self {
        Self { config }
    }

    /// Simulate one full verify→build→bench→flow cycle.
    /// Each stage succeeds if `source` is non-empty.
    /// `duration_ms` = stage index * 10 (Verify=0, Build=10, Bench=20, Flow=30).
    pub fn simulate_cycle(source: &str) -> Vec<LoopIteration> {
        let stages = [
            CompilerLoopStage::Verify,
            CompilerLoopStage::Build,
            CompilerLoopStage::Bench,
            CompilerLoopStage::Flow,
        ];
        let success = !source.is_empty();
        stages
            .into_iter()
            .enumerate()
            .map(|(idx, stage)| {
                LoopIteration::new(stage, source, success, (idx as u64) * 10)
            })
            .collect()
    }

    /// Run up to `max_cycles` full cycles. Stops early when all 4 stages in a
    /// cycle succeed. Returns (history of cycles, reached_success).
    pub fn run_until_success(
        &self,
        source: &str,
        max_cycles: u32,
    ) -> (Vec<Vec<LoopIteration>>, bool) {
        let mut history = Vec::new();
        for _ in 0..max_cycles {
            let cycle = Self::simulate_cycle(source);
            let all_ok = cycle.iter().all(|it| it.success);
            history.push(cycle);
            if all_ok {
                return (history, true);
            }
            if self.config.stop_on_failure {
                return (history, false);
            }
        }
        (history, false)
    }

    /// Sum the `duration_ms` of every iteration in a slice.
    pub fn total_duration(iterations: &[LoopIteration]) -> u64 {
        iterations.iter().map(|it| it.duration_ms).sum()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod ai_compiler_loop_tests {
    use super::*;

    #[test]
    fn stage_next_chain() {
        assert_eq!(
            CompilerLoopStage::Verify.next(),
            Some(CompilerLoopStage::Build)
        );
        assert_eq!(
            CompilerLoopStage::Build.next(),
            Some(CompilerLoopStage::Bench)
        );
        assert_eq!(
            CompilerLoopStage::Bench.next(),
            Some(CompilerLoopStage::Flow)
        );
        assert_eq!(CompilerLoopStage::Flow.next(), None);
    }

    #[test]
    fn stage_name_correct() {
        assert_eq!(CompilerLoopStage::Verify.stage_name(), "verify");
        assert_eq!(CompilerLoopStage::Build.stage_name(), "build");
        assert_eq!(CompilerLoopStage::Bench.stage_name(), "bench");
        assert_eq!(CompilerLoopStage::Flow.stage_name(), "flow");
    }

    #[test]
    fn loop_iteration_new_output_ok_when_success() {
        let it = LoopIteration::new(CompilerLoopStage::Verify, "src", true, 5);
        assert_eq!(it.output, "ok");
        assert!(it.success);
    }

    #[test]
    fn simulate_cycle_returns_four_iterations() {
        let iters = AiCompilerLoop::simulate_cycle("hello");
        assert_eq!(iters.len(), 4);
    }

    #[test]
    fn simulate_cycle_all_succeed_non_empty_source() {
        let iters = AiCompilerLoop::simulate_cycle("hello");
        assert!(iters.iter().all(|it| it.success));
    }

    #[test]
    fn simulate_cycle_all_fail_empty_source() {
        let iters = AiCompilerLoop::simulate_cycle("");
        assert!(iters.iter().all(|it| !it.success));
    }

    #[test]
    fn run_until_success_reaches_success() {
        let lp = AiCompilerLoop::new(CompilerLoopConfig::default());
        let (history, ok) = lp.run_until_success("source code", 5);
        assert!(ok, "must reach success with non-empty source");
        assert_eq!(history.len(), 1, "success on first cycle");
    }

    #[test]
    fn total_duration_sums_correctly() {
        let iters = AiCompilerLoop::simulate_cycle("x");
        // durations: 0, 10, 20, 30 → sum = 60
        assert_eq!(AiCompilerLoop::total_duration(&iters), 60);
    }

    #[test]
    fn compiler_loop_config_default_values() {
        let cfg = CompilerLoopConfig::default();
        assert_eq!(cfg.max_iterations, 4);
        assert!(cfg.stop_on_failure);
    }
}
