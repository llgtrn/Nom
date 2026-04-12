//! `nom-bench` — compiler-integrated benchmarking per §5.13.
//!
//! A `BenchmarkRun` records one execution of a compiled closure on a
//! specific platform + workload. Runs are stored in the typed
//! `entry_benchmarks` side-table keyed by `(run_id, entry_hash,
//! platform, compiler_hash, workload_key)`. The §5.15 joint-optimization
//! solver consumes these rows to pick per-platform specializations.
//!
//! This crate is the Phase-5 §5.13 scaffold. The actual runner (launch
//! the compiled closure under a harness, sample latency, capture
//! custom counters) arrives incrementally — likely one runner type per
//! workload class (latency microbench, throughput burst, memory
//! allocation trace, …).

use thiserror::Error;

/// Benchmark target platform. Distinct from `nom_ux::Platform`:
/// this one is about **where the benchmark runs** (host CPU + OS +
/// feature set), not about UI runtime targets.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct BenchPlatform {
    /// Target triple, e.g. `x86_64-pc-windows-msvc`, `aarch64-apple-darwin`,
    /// `wasm32-unknown-unknown`.
    pub target_triple: String,
    /// Enabled CPU features, sorted. E.g. `["avx2", "bmi2", "sse4.2"]`.
    pub cpu_features: Vec<String>,
    /// Human-readable OS/kernel identifier, e.g. `Windows 11 24H2`.
    pub os_identifier: String,
}

/// One sample's timing summary. Nanosecond resolution.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TimingMoments {
    pub n_samples: u64,
    pub mean_ns: f64,
    pub stddev_ns: f64,
    pub p50_ns: u64,
    pub p95_ns: u64,
    pub p99_ns: u64,
    pub min_ns: u64,
    pub max_ns: u64,
}

impl TimingMoments {
    /// Zeroed moments for a run that produced no samples (e.g. early
    /// failure). Callers check `n_samples == 0` to treat as invalid.
    pub fn empty() -> Self {
        Self {
            n_samples: 0,
            mean_ns: 0.0,
            stddev_ns: 0.0,
            p50_ns: 0,
            p95_ns: 0,
            p99_ns: 0,
            min_ns: 0,
            max_ns: 0,
        }
    }
}

/// A single benchmark run. Becomes one `entry_benchmarks` row.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BenchmarkRun {
    /// Fresh UUID-ish id for this run.
    pub run_id: String,
    /// Hash of the benchmarked entry.
    pub entry_hash: String,
    /// Hash of the compiler used to produce the benchmarked artifact.
    /// Enables §5.15 to detect "same entry, different compiler" regressions.
    pub compiler_hash: String,
    pub platform: BenchPlatform,
    /// Workload descriptor — freeform key identifying which input this
    /// run used. Example: `image_decode:photo_8k_srgb` or
    /// `sort:sorted_1M_u64`. Same entry under different workloads
    /// becomes different rows.
    pub workload_key: String,
    pub timing: TimingMoments,
    /// Unix timestamp (seconds since epoch) when the run completed.
    pub completed_at_unix_s: i64,
    /// Custom counters: allocations, branch misses, cache references,
    /// anything the runner samples beyond wall-clock.
    pub custom_counters: serde_json::Value,
}

/// Errors produced by `nom-bench`.
#[derive(Debug, Error)]
pub enum BenchError {
    #[error("runner not yet implemented for workload class: {0}")]
    RunnerNotYetImplemented(String),
    #[error("entry has no buildable body for target {0:?}")]
    NoArtifact(BenchPlatform),
    #[error("benchmark budget exceeded (wall-clock limit hit)")]
    BudgetExceeded,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_timing_moments_has_zero_samples() {
        let t = TimingMoments::empty();
        assert_eq!(t.n_samples, 0);
        assert_eq!(t.p50_ns, 0);
        assert_eq!(t.p99_ns, 0);
    }

    #[test]
    fn benchmark_run_round_trips_through_json() {
        let run = BenchmarkRun {
            run_id: "run_abc".into(),
            entry_hash: "h_xyz".into(),
            compiler_hash: "c_123".into(),
            platform: BenchPlatform {
                target_triple: "x86_64-pc-windows-msvc".into(),
                cpu_features: vec!["avx2".into(), "bmi2".into()],
                os_identifier: "Windows 11".into(),
            },
            workload_key: "image_decode:photo_8k_srgb".into(),
            timing: TimingMoments {
                n_samples: 100,
                mean_ns: 1_234.5,
                stddev_ns: 12.3,
                p50_ns: 1_200,
                p95_ns: 1_400,
                p99_ns: 1_500,
                min_ns: 1_100,
                max_ns: 1_600,
            },
            completed_at_unix_s: 1_700_000_000,
            custom_counters: serde_json::json!({
                "allocations": 42,
                "cache_misses": 17,
            }),
        };
        let s = serde_json::to_string(&run).unwrap();
        let back: BenchmarkRun = serde_json::from_str(&s).unwrap();
        assert_eq!(run, back);
    }
}
