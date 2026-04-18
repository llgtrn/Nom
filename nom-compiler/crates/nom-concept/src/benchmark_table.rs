/// entry_benchmarks side-table — §5.12 typed side-table for query-hot bench data.
///
/// `EntryBenchmark` holds one benchmark run row.
/// `BenchmarkSideTable` is the in-memory typed side-table.
/// `BenchmarkAggregation` holds aggregated stats for a workload key.

// ── EntryBenchmark ────────────────────────────────────────────────────────────

/// One benchmark run row: run_id, platform, compiler_hash, workload_key, timing.
#[derive(Debug, Clone, PartialEq)]
pub struct EntryBenchmark {
    pub run_id: u64,
    pub platform: String,
    pub compiler_hash: u64,
    pub workload_key: String,
    pub timing_ms: f64,
    pub custom_counter: u64,
}

impl EntryBenchmark {
    /// Create a new benchmark row with `custom_counter` defaulting to 0.
    pub fn new(
        run_id: u64,
        platform: impl Into<String>,
        compiler_hash: u64,
        workload_key: impl Into<String>,
        timing_ms: f64,
    ) -> Self {
        Self {
            run_id,
            platform: platform.into(),
            compiler_hash,
            workload_key: workload_key.into(),
            timing_ms,
            custom_counter: 0,
        }
    }

    /// Builder-style setter for `custom_counter`.
    pub fn with_counter(mut self, counter: u64) -> Self {
        self.custom_counter = counter;
        self
    }
}

// ── BenchmarkSideTable ────────────────────────────────────────────────────────

/// Typed in-memory side-table of `EntryBenchmark` rows.
#[derive(Debug, Default)]
pub struct BenchmarkSideTable {
    rows: Vec<EntryBenchmark>,
}

impl BenchmarkSideTable {
    /// Create an empty side-table.
    pub fn new() -> Self {
        Self { rows: Vec::new() }
    }

    /// Append a row to the table.
    pub fn insert(&mut self, row: EntryBenchmark) {
        self.rows.push(row);
    }

    /// Return all rows whose `workload_key` matches `key`.
    pub fn query_workload<'a>(&'a self, key: &str) -> Vec<&'a EntryBenchmark> {
        self.rows
            .iter()
            .filter(|r| r.workload_key == key)
            .collect()
    }

    /// Total number of rows in the table.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Whether the table has no rows.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Return the last inserted row whose `workload_key` matches `key`, or
    /// `None` if no such row exists.
    pub fn latest_for_workload(&self, key: &str) -> Option<&EntryBenchmark> {
        self.rows.iter().rev().find(|r| r.workload_key == key)
    }
}

// ── BenchmarkAggregation ─────────────────────────────────────────────────────

/// Aggregated timing stats (min, max, avg) for a single workload key.
#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkAggregation {
    pub workload_key: String,
    pub min_ms: f64,
    pub max_ms: f64,
    pub avg_ms: f64,
    pub sample_count: usize,
}

impl BenchmarkAggregation {
    /// Compute aggregated stats from a slice of row references.
    ///
    /// Returns `None` when `rows` is empty.
    pub fn compute(key: &str, rows: &[&EntryBenchmark]) -> Option<Self> {
        if rows.is_empty() {
            return None;
        }
        let mut min_ms = f64::MAX;
        let mut max_ms = f64::MIN;
        let mut sum = 0.0_f64;
        for r in rows {
            if r.timing_ms < min_ms {
                min_ms = r.timing_ms;
            }
            if r.timing_ms > max_ms {
                max_ms = r.timing_ms;
            }
            sum += r.timing_ms;
        }
        let sample_count = rows.len();
        Some(Self {
            workload_key: key.to_string(),
            min_ms,
            max_ms,
            avg_ms: sum / sample_count as f64,
            sample_count,
        })
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod benchmark_table_tests {
    use super::*;

    #[test]
    fn entry_benchmark_new_fields() {
        let b = EntryBenchmark::new(1, "linux", 0xdeadbeef, "sort_1k", 12.5);
        assert_eq!(b.run_id, 1);
        assert_eq!(b.platform, "linux");
        assert_eq!(b.compiler_hash, 0xdeadbeef);
        assert_eq!(b.workload_key, "sort_1k");
        assert!((b.timing_ms - 12.5).abs() < f64::EPSILON);
        assert_eq!(b.custom_counter, 0);
    }

    #[test]
    fn entry_benchmark_with_counter() {
        let b = EntryBenchmark::new(2, "win", 0, "parse_big", 5.0).with_counter(42);
        assert_eq!(b.custom_counter, 42);
        // Other fields unchanged
        assert_eq!(b.run_id, 2);
        assert_eq!(b.workload_key, "parse_big");
    }

    #[test]
    fn benchmark_side_table_insert_and_len() {
        let mut table = BenchmarkSideTable::new();
        assert_eq!(table.len(), 0);
        table.insert(EntryBenchmark::new(1, "linux", 0, "w1", 1.0));
        table.insert(EntryBenchmark::new(2, "linux", 0, "w2", 2.0));
        assert_eq!(table.len(), 2);
    }

    #[test]
    fn benchmark_side_table_query_workload_filters() {
        let mut table = BenchmarkSideTable::new();
        table.insert(EntryBenchmark::new(1, "linux", 0, "alpha", 1.0));
        table.insert(EntryBenchmark::new(2, "linux", 0, "beta", 2.0));
        table.insert(EntryBenchmark::new(3, "linux", 0, "alpha", 3.0));

        let alpha = table.query_workload("alpha");
        assert_eq!(alpha.len(), 2);
        assert!(alpha.iter().all(|r| r.workload_key == "alpha"));

        let beta = table.query_workload("beta");
        assert_eq!(beta.len(), 1);
        assert_eq!(beta[0].run_id, 2);
    }

    #[test]
    fn benchmark_side_table_latest_for_workload() {
        let mut table = BenchmarkSideTable::new();
        table.insert(EntryBenchmark::new(10, "mac", 0, "load", 5.0));
        table.insert(EntryBenchmark::new(11, "mac", 0, "other", 3.0));
        table.insert(EntryBenchmark::new(12, "mac", 0, "load", 7.0));

        let latest = table.latest_for_workload("load").expect("should find one");
        assert_eq!(latest.run_id, 12);
    }

    #[test]
    fn benchmark_side_table_query_returns_empty_for_unknown() {
        let mut table = BenchmarkSideTable::new();
        table.insert(EntryBenchmark::new(1, "linux", 0, "known", 1.0));
        let result = table.query_workload("unknown");
        assert!(result.is_empty());
    }

    #[test]
    fn benchmark_aggregation_compute_empty_returns_none() {
        let result = BenchmarkAggregation::compute("empty_key", &[]);
        assert!(result.is_none());
    }

    #[test]
    fn benchmark_aggregation_compute_stats() {
        let b1 = EntryBenchmark::new(1, "linux", 0, "w", 10.0);
        let b2 = EntryBenchmark::new(2, "linux", 0, "w", 20.0);
        let b3 = EntryBenchmark::new(3, "linux", 0, "w", 30.0);
        let rows = vec![&b1, &b2, &b3];
        let agg = BenchmarkAggregation::compute("w", &rows).expect("non-empty");
        assert_eq!(agg.workload_key, "w");
        assert!((agg.min_ms - 10.0).abs() < f64::EPSILON);
        assert!((agg.max_ms - 30.0).abs() < f64::EPSILON);
        assert_eq!(agg.sample_count, 3);
    }

    #[test]
    fn benchmark_aggregation_avg_correct() {
        let b1 = EntryBenchmark::new(1, "linux", 0, "avg_test", 4.0);
        let b2 = EntryBenchmark::new(2, "linux", 0, "avg_test", 8.0);
        let rows = vec![&b1, &b2];
        let agg = BenchmarkAggregation::compute("avg_test", &rows).expect("non-empty");
        assert!((agg.avg_ms - 6.0).abs() < f64::EPSILON);
    }
}
