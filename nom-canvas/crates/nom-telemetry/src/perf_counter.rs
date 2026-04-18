use std::collections::HashMap;

// ---------------------------------------------------------------------------
// CounterKind
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum CounterKind {
    Increment,
    Gauge,
    Histogram,
    Rate,
}

impl CounterKind {
    pub fn is_cumulative(&self) -> bool {
        matches!(self, CounterKind::Increment)
    }

    pub fn unit_suffix(&self) -> &'static str {
        match self {
            CounterKind::Increment => "/total",
            CounterKind::Gauge => "",
            CounterKind::Histogram => "/sample",
            CounterKind::Rate => "/sec",
        }
    }
}

// ---------------------------------------------------------------------------
// PerfCounter
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct PerfCounter {
    pub name: String,
    pub kind: CounterKind,
    pub value: i64,
}

impl PerfCounter {
    pub fn new(name: impl Into<String>, kind: CounterKind) -> Self {
        Self { name: name.into(), kind, value: 0 }
    }

    pub fn increment(&mut self, delta: i64) {
        self.value += delta;
    }

    pub fn reset(&mut self) {
        self.value = 0;
    }

    pub fn label(&self) -> String {
        format!("{}{}", self.name, self.kind.unit_suffix())
    }
}

// ---------------------------------------------------------------------------
// CounterSnapshot
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CounterSnapshot {
    pub name: String,
    pub value: i64,
    pub timestamp_ms: u64,
}

impl CounterSnapshot {
    pub fn delta_from(&self, prev: &CounterSnapshot) -> i64 {
        self.value - prev.value
    }
}

// ---------------------------------------------------------------------------
// CounterRegistry
// ---------------------------------------------------------------------------

pub struct CounterRegistry {
    pub counters: HashMap<String, PerfCounter>,
}

impl CounterRegistry {
    pub fn new() -> Self {
        Self { counters: HashMap::new() }
    }

    pub fn register(&mut self, c: PerfCounter) {
        self.counters.insert(c.name.clone(), c);
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut PerfCounter> {
        self.counters.get_mut(name)
    }

    pub fn snapshot_all(&self, timestamp_ms: u64) -> Vec<CounterSnapshot> {
        let mut snapshots: Vec<CounterSnapshot> = self
            .counters
            .values()
            .map(|c| CounterSnapshot {
                name: c.name.clone(),
                value: c.value,
                timestamp_ms,
            })
            .collect();
        snapshots.sort_by(|a, b| a.name.cmp(&b.name));
        snapshots
    }
}

impl Default for CounterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// RateCalculator
// ---------------------------------------------------------------------------

pub struct RateCalculator {
    pub snapshots: Vec<CounterSnapshot>,
}

impl RateCalculator {
    pub fn new() -> Self {
        Self { snapshots: Vec::new() }
    }

    pub fn add_snapshot(&mut self, s: CounterSnapshot) {
        self.snapshots.push(s);
    }

    pub fn rate_per_sec(&self, name: &str) -> f64 {
        let relevant: Vec<&CounterSnapshot> =
            self.snapshots.iter().filter(|s| s.name == name).collect();
        if relevant.len() < 2 {
            return 0.0;
        }
        let first = relevant[0];
        let last = relevant[relevant.len() - 1];
        let time_diff_ms = last.timestamp_ms.saturating_sub(first.timestamp_ms);
        if time_diff_ms == 0 {
            return 0.0;
        }
        (last.value - first.value) as f64 / (time_diff_ms as f64 / 1000.0)
    }
}

impl Default for RateCalculator {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod perf_counter_tests {
    use super::*;

    #[test]
    fn kind_is_cumulative() {
        assert!(CounterKind::Increment.is_cumulative());
        assert!(!CounterKind::Gauge.is_cumulative());
        assert!(!CounterKind::Histogram.is_cumulative());
        assert!(!CounterKind::Rate.is_cumulative());
    }

    #[test]
    fn kind_unit_suffix_rate() {
        assert_eq!(CounterKind::Rate.unit_suffix(), "/sec");
    }

    #[test]
    fn counter_increment_and_value() {
        let mut c = PerfCounter::new("reqs", CounterKind::Increment);
        c.increment(5);
        c.increment(3);
        assert_eq!(c.value, 8);
    }

    #[test]
    fn counter_reset() {
        let mut c = PerfCounter::new("reqs", CounterKind::Gauge);
        c.increment(42);
        c.reset();
        assert_eq!(c.value, 0);
    }

    #[test]
    fn counter_label_format() {
        let c = PerfCounter::new("latency", CounterKind::Histogram);
        assert_eq!(c.label(), "latency/sample");
    }

    #[test]
    fn snapshot_delta_from() {
        let prev = CounterSnapshot { name: "x".into(), value: 10, timestamp_ms: 1000 };
        let curr = CounterSnapshot { name: "x".into(), value: 35, timestamp_ms: 2000 };
        assert_eq!(curr.delta_from(&prev), 25);
    }

    #[test]
    fn registry_snapshot_all_sorted() {
        let mut reg = CounterRegistry::new();
        reg.register(PerfCounter::new("zebra", CounterKind::Gauge));
        reg.register(PerfCounter::new("alpha", CounterKind::Gauge));
        reg.register(PerfCounter::new("mango", CounterKind::Gauge));
        let snaps = reg.snapshot_all(0);
        let names: Vec<&str> = snaps.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "mango", "zebra"]);
    }

    #[test]
    fn registry_snapshot_count() {
        let mut reg = CounterRegistry::new();
        reg.register(PerfCounter::new("a", CounterKind::Increment));
        reg.register(PerfCounter::new("b", CounterKind::Increment));
        reg.register(PerfCounter::new("c", CounterKind::Increment));
        assert_eq!(reg.snapshot_all(0).len(), 3);
    }

    #[test]
    fn rate_calculator_rate_per_sec() {
        let mut calc = RateCalculator::new();
        calc.add_snapshot(CounterSnapshot { name: "hits".into(), value: 100, timestamp_ms: 0 });
        calc.add_snapshot(CounterSnapshot { name: "hits".into(), value: 300, timestamp_ms: 2000 });
        // (300 - 100) / (2000 / 1000) = 200 / 2 = 100.0
        assert!((calc.rate_per_sec("hits") - 100.0).abs() < f64::EPSILON);
    }
}
