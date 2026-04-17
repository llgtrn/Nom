use crate::span::Span;

pub trait SpanExporter: Send + Sync {
    fn export(&self, span: Span);
    fn flush(&self);
}

pub struct InMemoryExporter {
    spans: parking_lot::Mutex<Vec<Span>>,
}

impl InMemoryExporter {
    pub fn new() -> Self {
        Self { spans: parking_lot::Mutex::new(Vec::new()) }
    }

    pub fn drain(&self) -> Vec<Span> {
        let mut guard = self.spans.lock();
        std::mem::take(&mut *guard)
    }
}

impl SpanExporter for InMemoryExporter {
    fn export(&self, span: Span) {
        self.spans.lock().push(span);
    }

    fn flush(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tier::TelemetryTier;

    #[test]
    fn export_stores_span() {
        let exp = InMemoryExporter::new();
        let s = Span::new("test", TelemetryTier::Ui);
        exp.export(s);
        assert_eq!(exp.drain().len(), 1);
    }

    #[test]
    fn drain_clears_storage() {
        let exp = InMemoryExporter::new();
        exp.export(Span::new("a", TelemetryTier::Ui));
        exp.export(Span::new("b", TelemetryTier::Ui));
        let _ = exp.drain();
        assert_eq!(exp.drain().len(), 0);
    }

    #[test]
    fn flush_does_not_clear() {
        let exp = InMemoryExporter::new();
        exp.export(Span::new("x", TelemetryTier::Background));
        exp.flush();
        assert_eq!(exp.drain().len(), 1);
    }

    #[test]
    fn exporter_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<InMemoryExporter>();
    }
}
