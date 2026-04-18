//! Span aggregation with P95 latency calculation.
//!
//! Provides [`SpanSample`], [`P95Calculator`], [`SpanAggregator`], and
//! [`TraceReport`] for collecting per-span measurements and computing
//! percentile summaries.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// SpanSample
// ---------------------------------------------------------------------------

/// A single span measurement: a named operation and its duration.
#[derive(Debug, Clone)]
pub struct SpanSample {
    pub name: String,
    pub duration_ms: f64,
}

impl SpanSample {
    pub fn new(name: impl Into<String>, duration_ms: f64) -> Self {
        Self {
            name: name.into(),
            duration_ms,
        }
    }
}

// ---------------------------------------------------------------------------
// P95Calculator
// ---------------------------------------------------------------------------

/// Stateless helper that computes P95 latency from a slice of durations.
pub struct P95Calculator;

impl P95Calculator {
    /// Returns the P95 value from `samples`, or `0.0` if the slice is empty.
    ///
    /// Implementation: sort a local copy, then return the element at index
    /// `floor(0.95 * len - 1)` (clamped to a valid index).
    pub fn compute(samples: &[f64]) -> f64 {
        if samples.is_empty() {
            return 0.0;
        }
        let mut sorted = samples.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let idx = ((0.95_f64 * sorted.len() as f64) - 1.0)
            .floor()
            .max(0.0) as usize;
        let idx = idx.min(sorted.len() - 1);
        sorted[idx]
    }
}

// ---------------------------------------------------------------------------
// SpanAggregator
// ---------------------------------------------------------------------------

/// Collects [`SpanSample`] measurements and computes per-span percentiles.
pub struct SpanAggregator {
    buckets: HashMap<String, Vec<f64>>,
}

impl SpanAggregator {
    pub fn new() -> Self {
        Self {
            buckets: HashMap::new(),
        }
    }

    /// Record one span measurement.
    pub fn record(&mut self, sample: SpanSample) {
        self.buckets
            .entry(sample.name)
            .or_default()
            .push(sample.duration_ms);
    }

    /// Return all recorded durations for the given span name, or an empty
    /// slice if the span has not been recorded.
    pub fn samples_for(&self, name: &str) -> &[f64] {
        self.buckets
            .get(name)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    /// Return the P95 latency for the given span name.
    pub fn p95_for(&self, name: &str) -> f64 {
        P95Calculator::compute(self.samples_for(name))
    }

    /// Return all span names that have been recorded.
    pub fn span_names(&self) -> Vec<&String> {
        self.buckets.keys().collect()
    }

    /// Return the total number of samples recorded across all spans.
    pub fn total_samples(&self) -> usize {
        self.buckets.values().map(Vec::len).sum()
    }
}

impl Default for SpanAggregator {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// TraceReport
// ---------------------------------------------------------------------------

/// A summary report built from a [`SpanAggregator`].
///
/// Each entry is `(span_name, p95_ms, sample_count)`.
pub struct TraceReport {
    pub entries: Vec<(String, f64, usize)>,
}

impl TraceReport {
    /// Build a report from all spans in `agg`.
    pub fn from_aggregator(agg: &SpanAggregator) -> Self {
        let mut entries: Vec<(String, f64, usize)> = agg
            .span_names()
            .into_iter()
            .map(|name| {
                let p95 = agg.p95_for(name);
                let count = agg.samples_for(name).len();
                (name.clone(), p95, count)
            })
            .collect();
        // Deterministic order for reproducible tests.
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        Self { entries }
    }

    /// Number of spans in the report.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` when the report contains no spans.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Look up a span by name and return `(p95_ms, sample_count)`, or `None`.
    pub fn find(&self, name: &str) -> Option<(f64, usize)> {
        self.entries
            .iter()
            .find(|(n, _, _)| n == name)
            .map(|(_, p95, count)| (*p95, *count))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod span_aggregator_tests {
    use super::*;

    // --- P95Calculator ---

    #[test]
    fn p95_calculator_empty_returns_zero() {
        assert_eq!(P95Calculator::compute(&[]), 0.0);
    }

    #[test]
    fn p95_calculator_single_sample() {
        // With one element, index = floor(0.95 * 1 - 1) = floor(-0.05) = 0 (clamped).
        assert_eq!(P95Calculator::compute(&[42.0]), 42.0);
    }

    #[test]
    fn p95_calculator_p95_of_100_samples() {
        // 1.0 .. 100.0 — P95 index = floor(0.95 * 100 - 1) = floor(94) = 94 → value 95.0
        let samples: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let p95 = P95Calculator::compute(&samples);
        assert_eq!(p95, 95.0);
    }

    // --- SpanAggregator ---

    #[test]
    fn span_aggregator_record_and_total() {
        let mut agg = SpanAggregator::new();
        agg.record(SpanSample::new("compile", 10.0));
        agg.record(SpanSample::new("compile", 20.0));
        agg.record(SpanSample::new("render", 5.0));
        assert_eq!(agg.total_samples(), 3);
    }

    #[test]
    fn span_aggregator_p95_for_known_span() {
        let mut agg = SpanAggregator::new();
        for i in 1..=100 {
            agg.record(SpanSample::new("query", i as f64));
        }
        assert_eq!(agg.p95_for("query"), 95.0);
    }

    #[test]
    fn span_aggregator_span_names_count() {
        let mut agg = SpanAggregator::new();
        agg.record(SpanSample::new("a", 1.0));
        agg.record(SpanSample::new("b", 2.0));
        agg.record(SpanSample::new("a", 3.0));
        assert_eq!(agg.span_names().len(), 2);
    }

    // --- TraceReport ---

    #[test]
    fn trace_report_from_aggregator_len() {
        let mut agg = SpanAggregator::new();
        agg.record(SpanSample::new("x", 1.0));
        agg.record(SpanSample::new("y", 2.0));
        let report = TraceReport::from_aggregator(&agg);
        assert_eq!(report.len(), 2);
    }

    #[test]
    fn trace_report_find_existing() {
        let mut agg = SpanAggregator::new();
        for i in 1..=20 {
            agg.record(SpanSample::new("db_query", i as f64));
        }
        let report = TraceReport::from_aggregator(&agg);
        let (p95, count) = report.find("db_query").expect("span must exist");
        assert_eq!(count, 20);
        // index = floor(0.95 * 20 - 1) = floor(18) = 18 → sorted[18] = 19.0
        assert_eq!(p95, 19.0);
    }

    #[test]
    fn trace_report_find_missing() {
        let agg = SpanAggregator::new();
        let report = TraceReport::from_aggregator(&agg);
        assert!(report.find("nonexistent").is_none());
    }
}
