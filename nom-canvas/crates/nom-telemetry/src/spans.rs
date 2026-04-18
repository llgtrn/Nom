/// The role of a span in distributed tracing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpanKind {
    Server,
    Client,
    Internal,
    Producer,
    Consumer,
}

/// A discrete event recorded within a span.
#[derive(Debug, Clone)]
pub struct SpanEvent {
    pub name: String,
    pub timestamp_ns: u64,
    pub attributes: Vec<(String, String)>,
}

impl SpanEvent {
    pub fn new(name: &str, ts: u64) -> Self {
        Self {
            name: name.to_string(),
            timestamp_ns: ts,
            attributes: Vec::new(),
        }
    }

    pub fn add_attribute(&mut self, key: &str, value: &str) {
        self.attributes.push((key.to_string(), value.to_string()));
    }

    pub fn attribute_count(&self) -> usize {
        self.attributes.len()
    }
}

/// An active (in-flight) distributed tracing span.
#[derive(Debug)]
pub struct ActiveSpan {
    pub trace_id: String,
    pub span_id: String,
    pub name: String,
    pub kind: SpanKind,
    pub events: Vec<SpanEvent>,
    pub start_ns: u64,
}

impl ActiveSpan {
    pub fn new(
        trace_id: &str,
        span_id: &str,
        name: &str,
        kind: SpanKind,
        start_ns: u64,
    ) -> Self {
        Self {
            trace_id: trace_id.to_string(),
            span_id: span_id.to_string(),
            name: name.to_string(),
            kind,
            events: Vec::new(),
            start_ns,
        }
    }

    pub fn add_event(&mut self, event: SpanEvent) {
        self.events.push(event);
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    pub fn duration_ns(&self, end_ns: u64) -> u64 {
        end_ns.saturating_sub(self.start_ns)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_event_new() {
        let ev = SpanEvent::new("cache.miss", 1_000_000);
        assert_eq!(ev.name, "cache.miss");
        assert_eq!(ev.timestamp_ns, 1_000_000);
        assert!(ev.attributes.is_empty());
    }

    #[test]
    fn span_event_attributes() {
        let mut ev = SpanEvent::new("db.query", 500);
        ev.add_attribute("table", "entries");
        ev.add_attribute("rows", "42");
        assert_eq!(ev.attribute_count(), 2);
        assert_eq!(ev.attributes[0], ("table".to_string(), "entries".to_string()));
        assert_eq!(ev.attributes[1], ("rows".to_string(), "42".to_string()));
    }

    #[test]
    fn active_span_new() {
        let span = ActiveSpan::new("trace-1", "span-1", "http.request", SpanKind::Server, 100);
        assert_eq!(span.trace_id, "trace-1");
        assert_eq!(span.span_id, "span-1");
        assert_eq!(span.name, "http.request");
        assert_eq!(span.kind, SpanKind::Server);
        assert_eq!(span.start_ns, 100);
        assert!(span.events.is_empty());
    }

    #[test]
    fn active_span_add_event() {
        let mut span = ActiveSpan::new("t2", "s2", "grpc.call", SpanKind::Client, 0);
        span.add_event(SpanEvent::new("retry", 10));
        assert_eq!(span.event_count(), 1);
    }

    #[test]
    fn active_span_event_count() {
        let mut span = ActiveSpan::new("t3", "s3", "internal.op", SpanKind::Internal, 0);
        assert_eq!(span.event_count(), 0);
        span.add_event(SpanEvent::new("step.1", 1));
        span.add_event(SpanEvent::new("step.2", 2));
        span.add_event(SpanEvent::new("step.3", 3));
        assert_eq!(span.event_count(), 3);
    }

    #[test]
    fn active_span_duration() {
        let span = ActiveSpan::new("t4", "s4", "producer.send", SpanKind::Producer, 1_000);
        assert_eq!(span.duration_ns(4_000), 3_000);
        assert_eq!(span.duration_ns(1_000), 0);
        // saturating: end before start
        assert_eq!(span.duration_ns(500), 0);
    }
}
