//! `nom bench regress` — regression checker for benchmark workloads.
//!
//! Compares actual measured timings against stored baselines and emits
//! `RegressAlert` entries for any workload that exceeds its tolerance
//! threshold.

/// Expected timing baseline for one workload.
#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkBaseline {
    /// Identifies the workload, e.g. `image_decode:photo_8k_srgb`.
    pub workload_key: String,
    /// Expected (baseline) timing in milliseconds.
    pub expected_ms: f64,
    /// Acceptable overshoot percentage. E.g. `10.0` means up to 10% over
    /// `expected_ms` is not a regression.
    pub tolerance_pct: f64,
}

impl BenchmarkBaseline {
    /// Create a new baseline.
    pub fn new(workload_key: impl Into<String>, expected_ms: f64, tolerance_pct: f64) -> Self {
        Self {
            workload_key: workload_key.into(),
            expected_ms,
            tolerance_pct,
        }
    }

    /// Maximum milliseconds before this run is flagged as a regression.
    ///
    /// `expected_ms * (1.0 + tolerance_pct / 100.0)`
    pub fn threshold_ms(&self) -> f64 {
        self.expected_ms * (1.0 + self.tolerance_pct / 100.0)
    }

    /// Returns `true` when `actual_ms` exceeds the tolerance threshold.
    pub fn is_regression(&self, actual_ms: f64) -> bool {
        actual_ms > self.threshold_ms()
    }
}

/// Describes one observed regression.
#[derive(Debug, Clone, PartialEq)]
pub struct RegressAlert {
    /// Which workload regressed.
    pub workload_key: String,
    /// The baseline (expected) millisecond value.
    pub baseline_ms: f64,
    /// The actual measured millisecond value.
    pub actual_ms: f64,
    /// `actual_ms / baseline_ms` — how many times slower the run was.
    pub ratio: f64,
}

impl RegressAlert {
    /// Construct an alert. `ratio` is computed as `actual_ms / baseline_ms`.
    pub fn new(workload_key: impl Into<String>, baseline_ms: f64, actual_ms: f64) -> Self {
        Self {
            workload_key: workload_key.into(),
            baseline_ms,
            actual_ms,
            ratio: actual_ms / baseline_ms,
        }
    }

    /// Human-readable severity label based on ratio.
    ///
    /// * `ratio > 2.0` → `"critical"`
    /// * `ratio > 1.5` → `"warning"`
    /// * otherwise     → `"info"`
    pub fn severity(&self) -> &str {
        if self.ratio > 2.0 {
            "critical"
        } else if self.ratio > 1.5 {
            "warning"
        } else {
            "info"
        }
    }
}

/// Holds a set of baselines and checks actual timings against them.
#[derive(Debug, Default)]
pub struct RegressionChecker {
    baselines: Vec<BenchmarkBaseline>,
}

impl RegressionChecker {
    /// Create an empty checker.
    pub fn new() -> Self {
        Self {
            baselines: Vec::new(),
        }
    }

    /// Register a baseline. Multiple baselines with different
    /// `workload_key` values may coexist; last-write wins if the key
    /// repeats.
    pub fn add_baseline(&mut self, baseline: BenchmarkBaseline) {
        if let Some(existing) = self
            .baselines
            .iter_mut()
            .find(|b| b.workload_key == baseline.workload_key)
        {
            *existing = baseline;
        } else {
            self.baselines.push(baseline);
        }
    }

    /// Check a single workload against its baseline.
    ///
    /// Returns `None` if:
    /// * no baseline exists for `workload_key`, or
    /// * `actual_ms` is within the tolerance threshold.
    ///
    /// Returns `Some(RegressAlert)` when the workload regressed.
    pub fn check(&self, workload_key: &str, actual_ms: f64) -> Option<RegressAlert> {
        let baseline = self.baselines.iter().find(|b| b.workload_key == workload_key)?;
        if baseline.is_regression(actual_ms) {
            Some(RegressAlert::new(workload_key, baseline.expected_ms, actual_ms))
        } else {
            None
        }
    }

    /// Check every `(workload_key, actual_ms)` pair and collect all alerts.
    pub fn check_all(&self, actuals: &[(String, f64)]) -> Vec<RegressAlert> {
        actuals
            .iter()
            .filter_map(|(key, ms)| self.check(key, *ms))
            .collect()
    }
}

#[cfg(test)]
mod regress_tests {
    use super::*;

    // ── BenchmarkBaseline ─────────────────────────────────────────────────────

    #[test]
    fn benchmark_baseline_threshold_ms() {
        let b = BenchmarkBaseline::new("sort:1M", 100.0, 10.0);
        // 100 * 1.10 = 110
        assert!((b.threshold_ms() - 110.0).abs() < 1e-9);
    }

    #[test]
    fn benchmark_baseline_is_regression_true() {
        let b = BenchmarkBaseline::new("sort:1M", 100.0, 10.0);
        // 111 > 110 → regression
        assert!(b.is_regression(111.0));
    }

    #[test]
    fn benchmark_baseline_is_regression_false() {
        let b = BenchmarkBaseline::new("sort:1M", 100.0, 10.0);
        // 110 == threshold → not a regression (must be strictly greater)
        assert!(!b.is_regression(110.0));
        // 105 < 110 → not a regression
        assert!(!b.is_regression(105.0));
    }

    // ── RegressAlert ──────────────────────────────────────────────────────────

    #[test]
    fn regress_alert_ratio_calculation() {
        let alert = RegressAlert::new("decode:8k", 50.0, 75.0);
        // 75 / 50 = 1.5
        assert!((alert.ratio - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn regress_alert_severity_critical() {
        let alert = RegressAlert::new("decode:8k", 50.0, 105.0); // ratio = 2.1
        assert_eq!(alert.severity(), "critical");
    }

    #[test]
    fn regress_alert_severity_warning() {
        let alert = RegressAlert::new("decode:8k", 50.0, 80.0); // ratio = 1.6
        assert_eq!(alert.severity(), "warning");
    }

    // ── RegressionChecker ─────────────────────────────────────────────────────

    #[test]
    fn regression_checker_no_alert_within_tolerance() {
        let mut checker = RegressionChecker::new();
        checker.add_baseline(BenchmarkBaseline::new("encode:mp3", 200.0, 5.0));
        // threshold = 210; actual = 205 → no alert
        assert!(checker.check("encode:mp3", 205.0).is_none());
    }

    #[test]
    fn regression_checker_alert_on_regression() {
        let mut checker = RegressionChecker::new();
        checker.add_baseline(BenchmarkBaseline::new("encode:mp3", 200.0, 5.0));
        // threshold = 210; actual = 250 → alert
        let alert = checker.check("encode:mp3", 250.0).expect("expected alert");
        assert_eq!(alert.workload_key, "encode:mp3");
        assert!((alert.baseline_ms - 200.0).abs() < f64::EPSILON);
        assert!((alert.actual_ms - 250.0).abs() < f64::EPSILON);
    }

    #[test]
    fn regression_checker_check_all_multiple() {
        let mut checker = RegressionChecker::new();
        checker.add_baseline(BenchmarkBaseline::new("a", 100.0, 0.0)); // threshold = 100
        checker.add_baseline(BenchmarkBaseline::new("b", 100.0, 0.0)); // threshold = 100
        checker.add_baseline(BenchmarkBaseline::new("c", 100.0, 0.0)); // threshold = 100

        let actuals = vec![
            ("a".to_string(), 99.0),  // ok
            ("b".to_string(), 101.0), // regression
            ("c".to_string(), 200.0), // regression
        ];
        let alerts = checker.check_all(&actuals);
        assert_eq!(alerts.len(), 2);
        assert!(alerts.iter().any(|a| a.workload_key == "b"));
        assert!(alerts.iter().any(|a| a.workload_key == "c"));
    }
}
