// ---------------------------------------------------------------------------
// metrics_deep — HistogramBucket, Histogram, SpanEvent, SpanTracer
// ---------------------------------------------------------------------------

/// A single histogram bucket with an upper bound.
#[derive(Debug, Clone)]
pub struct HistogramBucket {
    pub upper_bound: f64,
    pub count: u64,
}

impl HistogramBucket {
    pub fn new(upper_bound: f64) -> Self {
        Self { upper_bound, count: 0 }
    }

    /// Increment count if `value` falls within this bucket's upper bound.
    pub fn add(&mut self, value: f64) {
        if value <= self.upper_bound {
            self.count += 1;
        }
    }
}

/// A histogram that tracks observations across a set of buckets.
#[derive(Debug, Clone)]
pub struct Histogram {
    pub name: String,
    pub buckets: Vec<HistogramBucket>,
    pub sum: f64,
    pub count: u64,
}

impl Histogram {
    /// Create a histogram with one bucket per bound in `bounds`.
    pub fn new(name: impl Into<String>, bounds: Vec<f64>) -> Self {
        let buckets = bounds.into_iter().map(HistogramBucket::new).collect();
        Self { name: name.into(), buckets, sum: 0.0, count: 0 }
    }

    /// Record a single observation.
    pub fn observe(&mut self, value: f64) {
        self.sum += value;
        self.count += 1;
        for bucket in &mut self.buckets {
            bucket.add(value);
        }
    }

    /// Mean of all observations; 0.0 when no observations have been recorded.
    pub fn mean(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.sum / self.count as f64
        }
    }

    /// Upper bound of the first bucket whose cumulative count is >= count/2.
    /// Returns 0.0 if there are no buckets or no observations.
    pub fn p50_bucket(&self) -> f64 {
        if self.count == 0 || self.buckets.is_empty() {
            return 0.0;
        }
        let half = self.count / 2;
        for bucket in &self.buckets {
            if bucket.count >= half {
                return bucket.upper_bound;
            }
        }
        // Fallback: return last bucket upper bound
        self.buckets.last().map(|b| b.upper_bound).unwrap_or(0.0)
    }
}

// ---------------------------------------------------------------------------
// SpanEvent
// ---------------------------------------------------------------------------

/// A single traced span with a name, timestamps, and key-value attributes.
#[derive(Debug, Clone)]
pub struct SpanEvent {
    pub name: String,
    pub start_ns: u64,
    pub end_ns: u64,
    pub attributes: Vec<(String, String)>,
}

impl SpanEvent {
    pub fn new(name: impl Into<String>, start_ns: u64, end_ns: u64) -> Self {
        Self { name: name.into(), start_ns, end_ns, attributes: Vec::new() }
    }

    /// Wall-clock duration of this span in nanoseconds.
    pub fn duration_ns(&self) -> u64 {
        self.end_ns.saturating_sub(self.start_ns)
    }

    /// Attach a key-value attribute to this span.
    pub fn add_attr(&mut self, key: impl Into<String>, val: impl Into<String>) {
        self.attributes.push((key.into(), val.into()));
    }
}

// ---------------------------------------------------------------------------
// SpanTracer
// ---------------------------------------------------------------------------

/// Collects a sequence of spans for analysis.
#[derive(Debug, Default)]
pub struct SpanTracer {
    pub spans: Vec<SpanEvent>,
}

impl SpanTracer {
    pub fn new() -> Self {
        Self { spans: Vec::new() }
    }

    /// Append a span to the trace.
    pub fn record(&mut self, span: SpanEvent) {
        self.spans.push(span);
    }

    /// Sum of all span durations in nanoseconds.
    pub fn total_duration_ns(&self) -> u64 {
        self.spans.iter().map(|s| s.duration_ns()).sum()
    }

    /// The span with the longest duration, or `None` if no spans have been recorded.
    pub fn slowest_span(&self) -> Option<&SpanEvent> {
        self.spans.iter().max_by_key(|s| s.duration_ns())
    }

    /// All spans whose name matches `name`.
    pub fn spans_by_name(&self, name: &str) -> Vec<&SpanEvent> {
        self.spans.iter().filter(|s| s.name == name).collect()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod metrics_deep_tests {
    use super::*;

    #[test]
    fn histogram_bucket_add_increments_for_in_range() {
        let mut b = HistogramBucket::new(10.0);
        b.add(5.0);
        b.add(10.0);
        b.add(11.0); // out of range — should not increment
        assert_eq!(b.count, 2);
    }

    #[test]
    fn histogram_observe_increments_count() {
        let mut h = Histogram::new("req_latency", vec![1.0, 5.0, 10.0]);
        h.observe(3.0);
        h.observe(7.0);
        assert_eq!(h.count, 2);
    }

    #[test]
    fn histogram_mean_calculation() {
        let mut h = Histogram::new("sizes", vec![100.0, 200.0]);
        h.observe(50.0);
        h.observe(150.0);
        // mean = (50 + 150) / 2 = 100.0
        assert!((h.mean() - 100.0).abs() < 1e-9);
    }

    #[test]
    fn histogram_p50_bucket_returns_correct_upper_bound() {
        let mut h = Histogram::new("latency", vec![1.0, 5.0, 10.0, 50.0]);
        // Observe 4 values: 0.5, 0.8, 3.0, 20.0
        // Bucket counts after observation:
        //   [<=1.0]: 2  (0.5, 0.8)
        //   [<=5.0]: 3  (0.5, 0.8, 3.0)
        //   [<=10.0]: 3
        //   [<=50.0]: 4
        // count=4, half=2; first bucket where cumulative count >= 2 is <=1.0 (count=2)
        h.observe(0.5);
        h.observe(0.8);
        h.observe(3.0);
        h.observe(20.0);
        assert!((h.p50_bucket() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn span_event_duration_ns() {
        let s = SpanEvent::new("parse", 100, 350);
        assert_eq!(s.duration_ns(), 250);
    }

    #[test]
    fn span_event_add_attr() {
        let mut s = SpanEvent::new("compile", 0, 1000);
        s.add_attr("lang", "nom");
        s.add_attr("version", "1.0");
        assert_eq!(s.attributes.len(), 2);
        assert_eq!(s.attributes[0], ("lang".to_string(), "nom".to_string()));
    }

    #[test]
    fn span_tracer_slowest_span() {
        let mut tracer = SpanTracer::new();
        tracer.record(SpanEvent::new("fast", 0, 100));
        tracer.record(SpanEvent::new("slow", 0, 5000));
        tracer.record(SpanEvent::new("medium", 0, 500));
        let slowest = tracer.slowest_span().expect("should have a slowest span");
        assert_eq!(slowest.name, "slow");
    }

    #[test]
    fn span_tracer_spans_by_name_filter() {
        let mut tracer = SpanTracer::new();
        tracer.record(SpanEvent::new("parse", 0, 100));
        tracer.record(SpanEvent::new("compile", 0, 200));
        tracer.record(SpanEvent::new("parse", 0, 150));
        let found = tracer.spans_by_name("parse");
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn span_tracer_total_duration_ns() {
        let mut tracer = SpanTracer::new();
        tracer.record(SpanEvent::new("a", 0, 100));
        tracer.record(SpanEvent::new("b", 200, 400));
        tracer.record(SpanEvent::new("c", 500, 600));
        // 100 + 200 + 100 = 400
        assert_eq!(tracer.total_duration_ns(), 400);
    }
}
