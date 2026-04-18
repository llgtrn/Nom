// OpenTelemetry-style trace export: OTLP-compatible spans + Jaeger conversion.

// ---------------------------------------------------------------------------
// SpanStatus
// ---------------------------------------------------------------------------

/// The status of a completed span, following OTLP conventions.
#[derive(Debug, Clone, PartialEq)]
pub enum SpanStatus {
    Unset,
    Ok,
    Error,
}

impl SpanStatus {
    /// Returns the numeric status code (Unset=0, Ok=1, Error=2).
    pub fn status_code(&self) -> u8 {
        match self {
            SpanStatus::Unset => 0,
            SpanStatus::Ok => 1,
            SpanStatus::Error => 2,
        }
    }
}

// ---------------------------------------------------------------------------
// OpenTelemetrySpan
// ---------------------------------------------------------------------------

/// An OTLP-compatible span carrying timing, identity, and key-value attributes.
#[derive(Debug, Clone)]
pub struct OpenTelemetrySpan {
    pub trace_id: u64,
    pub span_id: u64,
    pub name: String,
    pub start_ns: u64,
    pub end_ns: u64,
    pub status: SpanStatus,
    pub attributes: Vec<(String, String)>,
}

impl OpenTelemetrySpan {
    /// Creates a new span with `Unset` status and no attributes.
    pub fn new(
        trace_id: u64,
        span_id: u64,
        name: impl Into<String>,
        start_ns: u64,
        end_ns: u64,
    ) -> Self {
        Self {
            trace_id,
            span_id,
            name: name.into(),
            start_ns,
            end_ns,
            status: SpanStatus::Unset,
            attributes: Vec::new(),
        }
    }

    /// Replaces the span's status.
    pub fn set_status(&mut self, status: SpanStatus) {
        self.status = status;
    }

    /// Appends a key-value attribute to the span.
    pub fn add_attribute(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.attributes.push((key.into(), value.into()));
    }

    /// Returns elapsed nanoseconds (end_ns − start_ns).
    pub fn duration_ns(&self) -> u64 {
        self.end_ns.saturating_sub(self.start_ns)
    }

    /// Returns the number of attributes attached to this span.
    pub fn attribute_count(&self) -> usize {
        self.attributes.len()
    }
}

// ---------------------------------------------------------------------------
// JaegerSpan
// ---------------------------------------------------------------------------

/// A Jaeger-compatible span representation.
#[derive(Debug, Clone)]
pub struct JaegerSpan {
    pub operation_name: String,
    pub trace_id_hex: String,
    pub span_id_hex: String,
    /// Duration in microseconds.
    pub duration_us: u64,
    pub tags: Vec<(String, String)>,
}

impl JaegerSpan {
    /// Converts an [`OpenTelemetrySpan`] into a [`JaegerSpan`].
    ///
    /// - Trace/span IDs are formatted as lowercase 16-digit hex strings.
    /// - Duration is converted from nanoseconds to microseconds (÷ 1000).
    /// - Attributes are copied verbatim as tags.
    pub fn from_otel(span: &OpenTelemetrySpan) -> JaegerSpan {
        JaegerSpan {
            operation_name: span.name.clone(),
            trace_id_hex: format!("{:016x}", span.trace_id),
            span_id_hex: format!("{:016x}", span.span_id),
            duration_us: span.duration_ns() / 1000,
            tags: span.attributes.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// TraceExporter
// ---------------------------------------------------------------------------

/// Exports [`OpenTelemetrySpan`]s by converting them to [`JaegerSpan`]s.
pub struct TraceExporter {
    exported_count: u64,
}

impl TraceExporter {
    /// Creates a new exporter with zero exports recorded.
    pub fn new() -> Self {
        Self { exported_count: 0 }
    }

    /// Converts a single span and increments the internal counter.
    pub fn export_span(&mut self, span: &OpenTelemetrySpan) -> JaegerSpan {
        self.exported_count += 1;
        JaegerSpan::from_otel(span)
    }

    /// Returns the total number of spans exported so far.
    pub fn exported_count(&self) -> u64 {
        self.exported_count
    }

    /// Converts a slice of spans and returns all results.
    pub fn export_batch(&mut self, spans: &[OpenTelemetrySpan]) -> Vec<JaegerSpan> {
        spans.iter().map(|s| self.export_span(s)).collect()
    }
}

impl Default for TraceExporter {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod trace_export_tests {
    use super::*;

    #[test]
    fn span_status_status_code() {
        assert_eq!(SpanStatus::Unset.status_code(), 0);
        assert_eq!(SpanStatus::Ok.status_code(), 1);
        assert_eq!(SpanStatus::Error.status_code(), 2);
    }

    #[test]
    fn otel_span_duration_ns() {
        let span = OpenTelemetrySpan::new(1, 2, "op", 1_000, 4_000);
        assert_eq!(span.duration_ns(), 3_000);
    }

    #[test]
    fn otel_span_add_attribute() {
        let mut span = OpenTelemetrySpan::new(1, 2, "op", 0, 10);
        span.add_attribute("env", "prod");
        span.add_attribute("version", "1.2.3");
        assert_eq!(span.attribute_count(), 2);
        assert_eq!(span.attributes[0], ("env".to_string(), "prod".to_string()));
        assert_eq!(
            span.attributes[1],
            ("version".to_string(), "1.2.3".to_string())
        );
    }

    #[test]
    fn otel_span_set_status() {
        let mut span = OpenTelemetrySpan::new(1, 2, "op", 0, 10);
        assert_eq!(span.status, SpanStatus::Unset);
        span.set_status(SpanStatus::Error);
        assert_eq!(span.status, SpanStatus::Error);
        span.set_status(SpanStatus::Ok);
        assert_eq!(span.status, SpanStatus::Ok);
    }

    #[test]
    fn jaeger_span_from_otel_duration() {
        // 5_000 ns → 5 µs
        let span = OpenTelemetrySpan::new(0xABCD, 0x1234, "fetch", 0, 5_000);
        let jaeger = JaegerSpan::from_otel(&span);
        assert_eq!(jaeger.duration_us, 5);
    }

    #[test]
    fn jaeger_span_from_otel_tags_count() {
        let mut span = OpenTelemetrySpan::new(1, 2, "op", 0, 100);
        span.add_attribute("a", "1");
        span.add_attribute("b", "2");
        span.add_attribute("c", "3");
        let jaeger = JaegerSpan::from_otel(&span);
        assert_eq!(jaeger.tags.len(), 3);
    }

    #[test]
    fn trace_exporter_export_span_count() {
        let mut exporter = TraceExporter::new();
        let span = OpenTelemetrySpan::new(1, 2, "op", 0, 1_000);
        let jaeger = exporter.export_span(&span);
        assert_eq!(jaeger.operation_name, "op");
        assert_eq!(exporter.exported_count(), 1);
    }

    #[test]
    fn trace_exporter_export_batch_count() {
        let mut exporter = TraceExporter::new();
        let spans = vec![
            OpenTelemetrySpan::new(1, 1, "a", 0, 100),
            OpenTelemetrySpan::new(1, 2, "b", 100, 200),
            OpenTelemetrySpan::new(1, 3, "c", 200, 300),
        ];
        let results = exporter.export_batch(&spans);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn trace_exporter_exported_count_increments() {
        let mut exporter = TraceExporter::new();
        assert_eq!(exporter.exported_count(), 0);
        let spans = vec![
            OpenTelemetrySpan::new(1, 1, "x", 0, 50),
            OpenTelemetrySpan::new(1, 2, "y", 50, 100),
        ];
        exporter.export_batch(&spans);
        assert_eq!(exporter.exported_count(), 2);
        exporter.export_span(&OpenTelemetrySpan::new(1, 3, "z", 100, 150));
        assert_eq!(exporter.exported_count(), 3);
    }
}
