/// Output format for metrics export.
#[derive(Debug, Clone, PartialEq)]
pub enum ExportFormat {
    Json,
    Prometheus,
    OpenTelemetry,
}

/// A single metric data point with optional labels.
#[derive(Debug, Clone)]
pub struct MetricRecord {
    pub name: String,
    pub value: f64,
    pub labels: Vec<(String, String)>,
    pub timestamp_ms: u64,
}

impl MetricRecord {
    /// Create a new record with the given name and value, no labels.
    pub fn new(name: &str, value: f64) -> Self {
        Self {
            name: name.to_string(),
            value,
            labels: Vec::new(),
            timestamp_ms: 0,
        }
    }

    /// Builder: attach a label key/value pair and return `self`.
    pub fn with_label(mut self, key: &str, value: &str) -> Self {
        self.labels.push((key.to_string(), value.to_string()));
        self
    }

    /// Number of labels attached to this record.
    pub fn label_count(&self) -> usize {
        self.labels.len()
    }
}

/// Collects [`MetricRecord`]s and serialises them in the chosen format.
pub struct MetricsExporter {
    pub format: ExportFormat,
    pub records: Vec<MetricRecord>,
}

impl MetricsExporter {
    /// Create a new exporter targeting `format`.
    pub fn new(format: ExportFormat) -> Self {
        Self { format, records: Vec::new() }
    }

    /// Append a record to the internal buffer.
    pub fn add_record(&mut self, record: MetricRecord) {
        self.records.push(record);
    }

    /// Return a stub serialisation of the current record count.
    pub fn export_stub(&self) -> String {
        let n = self.records.len();
        match self.format {
            ExportFormat::Json => format!("{{\"metrics\": {}}}", n),
            ExportFormat::Prometheus => {
                format!("# HELP metrics\nmetrics_count {}", n)
            }
            ExportFormat::OpenTelemetry => {
                format!("<metrics count=\"{}\"/>", n)
            }
        }
    }

    /// Number of records currently buffered.
    pub fn record_count(&self) -> usize {
        self.records.len()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metric_record_new() {
        let r = MetricRecord::new("cpu_usage", 42.5);
        assert_eq!(r.name, "cpu_usage");
        assert!((r.value - 42.5).abs() < f64::EPSILON);
        assert_eq!(r.label_count(), 0);
    }

    #[test]
    fn metric_with_label() {
        let r = MetricRecord::new("latency", 1.0)
            .with_label("host", "node-1")
            .with_label("env", "prod");
        assert_eq!(r.label_count(), 2);
        assert_eq!(r.labels[0], ("host".to_string(), "node-1".to_string()));
    }

    #[test]
    fn exporter_add_record() {
        let mut exp = MetricsExporter::new(ExportFormat::Json);
        assert_eq!(exp.record_count(), 0);
        exp.add_record(MetricRecord::new("m", 1.0));
        exp.add_record(MetricRecord::new("m", 2.0));
        assert_eq!(exp.record_count(), 2);
    }

    #[test]
    fn export_json_stub() {
        let mut exp = MetricsExporter::new(ExportFormat::Json);
        exp.add_record(MetricRecord::new("a", 1.0));
        exp.add_record(MetricRecord::new("b", 2.0));
        exp.add_record(MetricRecord::new("c", 3.0));
        assert_eq!(exp.export_stub(), "{\"metrics\": 3}");
    }

    #[test]
    fn export_prometheus_stub() {
        let mut exp = MetricsExporter::new(ExportFormat::Prometheus);
        exp.add_record(MetricRecord::new("x", 0.0));
        assert_eq!(exp.export_stub(), "# HELP metrics\nmetrics_count 1");
    }

    #[test]
    fn export_otel_stub() {
        let mut exp = MetricsExporter::new(ExportFormat::OpenTelemetry);
        exp.add_record(MetricRecord::new("y", 0.0));
        exp.add_record(MetricRecord::new("z", 0.0));
        assert_eq!(exp.export_stub(), "<metrics count=\"2\"/>");
    }
}
