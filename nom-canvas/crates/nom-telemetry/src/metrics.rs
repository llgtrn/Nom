// ---------------------------------------------------------------------------
// Counter
// ---------------------------------------------------------------------------

/// A monotonically increasing counter metric.
pub struct Counter {
    pub name: String,
    pub value: u64,
}

impl Counter {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            value: 0,
        }
    }

    pub fn increment(&mut self) {
        self.value += 1;
    }

    pub fn add(&mut self, delta: u64) {
        self.value += delta;
    }

    pub fn reset(&mut self) {
        self.value = 0;
    }

    pub fn value(&self) -> u64 {
        self.value
    }
}

// ---------------------------------------------------------------------------
// Histogram
// ---------------------------------------------------------------------------

/// Records a distribution of observed values.
pub struct Histogram {
    pub name: String,
    pub observations: Vec<f64>,
}

impl Histogram {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            observations: Vec::new(),
        }
    }

    pub fn record(&mut self, value: f64) {
        self.observations.push(value);
    }

    pub fn count(&self) -> usize {
        self.observations.len()
    }

    pub fn sum(&self) -> f64 {
        self.observations.iter().sum()
    }

    /// Mean of all observations, or `0.0` if no observations have been recorded.
    pub fn mean(&self) -> f64 {
        if self.observations.is_empty() {
            0.0
        } else {
            self.sum() / self.count() as f64
        }
    }

    /// Median (p50) of all observations, or `0.0` if no observations have been
    /// recorded.  Sorts a copy of the observations and picks the middle element.
    pub fn p50(&self) -> f64 {
        if self.observations.is_empty() {
            return 0.0;
        }
        let mut sorted = self.observations.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mid = sorted.len() / 2;
        if sorted.len().is_multiple_of(2) {
            (sorted[mid - 1] + sorted[mid]) / 2.0
        } else {
            sorted[mid]
        }
    }
}

// ---------------------------------------------------------------------------
// MetricsRegistry
// ---------------------------------------------------------------------------

/// Holds named counters and histograms.
#[derive(Default)]
pub struct MetricsRegistry {
    pub counters: Vec<Counter>,
    pub histograms: Vec<Histogram>,
}

impl MetricsRegistry {
    pub fn new() -> Self {
        Self {
            counters: Vec::new(),
            histograms: Vec::new(),
        }
    }

    /// Return a mutable reference to the named counter, creating it if absent.
    pub fn counter(&mut self, name: &str) -> &mut Counter {
        if let Some(pos) = self.counters.iter().position(|c| c.name == name) {
            return &mut self.counters[pos];
        }
        self.counters.push(Counter::new(name));
        self.counters.last_mut().expect("just pushed")
    }

    /// Return a mutable reference to the named histogram, creating it if absent.
    pub fn histogram(&mut self, name: &str) -> &mut Histogram {
        if let Some(pos) = self.histograms.iter().position(|h| h.name == name) {
            return &mut self.histograms[pos];
        }
        self.histograms.push(Histogram::new(name));
        self.histograms.last_mut().expect("just pushed")
    }

    /// Return the value of a named counter, or `0` if it does not exist.
    pub fn counter_value(&self, name: &str) -> u64 {
        self.counters
            .iter()
            .find(|c| c.name == name)
            .map(|c| c.value)
            .unwrap_or(0)
    }

    /// Return the total number of observations across all histograms.
    pub fn total_observations(&self) -> usize {
        self.histograms.iter().map(|h| h.count()).sum()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counter_new_starts_at_zero() {
        let c = Counter::new("requests");
        assert_eq!(c.value(), 0);
        assert_eq!(c.name, "requests");
    }

    #[test]
    fn counter_increment() {
        let mut c = Counter::new("hits");
        c.increment();
        assert_eq!(c.value(), 1);
        c.increment();
        assert_eq!(c.value(), 2);
    }

    #[test]
    fn counter_add() {
        let mut c = Counter::new("bytes");
        c.add(100);
        c.add(23);
        assert_eq!(c.value(), 123);
    }

    #[test]
    fn counter_reset() {
        let mut c = Counter::new("errors");
        c.add(99);
        c.reset();
        assert_eq!(c.value(), 0);
    }

    #[test]
    fn histogram_new_empty() {
        let h = Histogram::new("latency_ms");
        assert_eq!(h.count(), 0);
        assert_eq!(h.sum(), 0.0);
        assert_eq!(h.mean(), 0.0);
        assert_eq!(h.p50(), 0.0);
    }

    #[test]
    fn histogram_record_and_count() {
        let mut h = Histogram::new("latency_ms");
        h.record(10.0);
        h.record(20.0);
        h.record(30.0);
        assert_eq!(h.count(), 3);
        assert_eq!(h.sum(), 60.0);
    }

    #[test]
    fn histogram_mean_correct() {
        let mut h = Histogram::new("rtt");
        h.record(1.0);
        h.record(3.0);
        assert!((h.mean() - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn histogram_p50_odd_count() {
        let mut h = Histogram::new("resp");
        // Three values; median is middle after sort
        h.record(30.0);
        h.record(10.0);
        h.record(20.0);
        assert!((h.p50() - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn histogram_p50_even_count() {
        let mut h = Histogram::new("resp");
        h.record(10.0);
        h.record(20.0);
        h.record(30.0);
        h.record(40.0);
        // Median = (20+30)/2 = 25
        assert!((h.p50() - 25.0).abs() < f64::EPSILON);
    }

    #[test]
    fn registry_counter_get_or_create() {
        let mut reg = MetricsRegistry::new();
        reg.counter("req").increment();
        reg.counter("req").increment();
        assert_eq!(reg.counter_value("req"), 2);
        assert_eq!(reg.counter_value("missing"), 0);
    }

    #[test]
    fn registry_histogram_get_or_create() {
        let mut reg = MetricsRegistry::new();
        reg.histogram("lat").record(5.0);
        reg.histogram("lat").record(15.0);
        assert_eq!(reg.total_observations(), 2);
    }

    #[test]
    fn registry_total_observations_multiple_histograms() {
        let mut reg = MetricsRegistry::new();
        reg.histogram("a").record(1.0);
        reg.histogram("b").record(2.0);
        reg.histogram("b").record(3.0);
        assert_eq!(reg.total_observations(), 3);
    }

    #[test]
    fn registry_default_is_empty() {
        let reg = MetricsRegistry::default();
        assert!(reg.counters.is_empty());
        assert!(reg.histograms.is_empty());
    }
}
