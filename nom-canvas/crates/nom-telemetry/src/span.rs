use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

pub type TraceId = [u8; 16];
pub type SpanId = [u8; 8];

static COUNTER: AtomicU64 = AtomicU64::new(1);

fn next_id() -> u64 {
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

fn new_trace_id() -> TraceId {
    let a = next_id();
    let b = next_id();
    let mut id = [0u8; 16];
    id[..8].copy_from_slice(&a.to_le_bytes());
    id[8..].copy_from_slice(&b.to_le_bytes());
    id
}

fn new_span_id() -> SpanId {
    let v = next_id();
    v.to_le_bytes()
}

#[derive(Debug, Clone)]
pub struct SpanContext {
    pub trace_id: TraceId,
    pub span_id: SpanId,
    pub sampled: bool,
}

#[derive(Debug, Clone)]
pub struct Span {
    pub context: SpanContext,
    pub name: String,
    pub tier: crate::tier::TelemetryTier,
    pub parent: Option<SpanId>,
    pub attributes: Vec<(String, String)>,
    pub start_micros: u64,
    pub end_micros: Option<u64>,
}

fn now_micros() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_micros() as u64)
        .unwrap_or(0)
}

impl Span {
    pub fn new(name: impl Into<String>, tier: crate::tier::TelemetryTier) -> Self {
        Self {
            context: SpanContext {
                trace_id: new_trace_id(),
                span_id: new_span_id(),
                sampled: true,
            },
            name: name.into(),
            tier,
            parent: None,
            attributes: Vec::new(),
            start_micros: now_micros(),
            end_micros: None,
        }
    }

    pub fn with_attribute(mut self, k: impl Into<String>, v: impl Into<String>) -> Self {
        self.attributes.push((k.into(), v.into()));
        self
    }

    pub fn child_of(parent: &Span) -> Self {
        Self {
            context: SpanContext {
                trace_id: parent.context.trace_id,
                span_id: new_span_id(),
                sampled: parent.context.sampled,
            },
            name: String::new(),
            tier: parent.tier,
            parent: Some(parent.context.span_id),
            attributes: Vec::new(),
            start_micros: now_micros(),
            end_micros: None,
        }
    }

    pub fn finish(&mut self) {
        self.end_micros = Some(now_micros());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tier::TelemetryTier;

    #[test]
    fn new_span_has_no_end() {
        let s = Span::new("test", TelemetryTier::Ui);
        assert!(s.end_micros.is_none());
        assert!(s.start_micros > 0);
    }

    #[test]
    fn finish_sets_end_micros() {
        let mut s = Span::new("test", TelemetryTier::Interactive);
        s.finish();
        assert!(s.end_micros.is_some());
    }

    #[test]
    fn with_attribute_stores_kv() {
        let s = Span::new("test", TelemetryTier::Background)
            .with_attribute("key", "val");
        assert_eq!(s.attributes.len(), 1);
        assert_eq!(s.attributes[0], ("key".into(), "val".into()));
    }

    #[test]
    fn child_inherits_trace_id() {
        let parent = Span::new("parent", TelemetryTier::Ui);
        let child = Span::child_of(&parent);
        assert_eq!(child.context.trace_id, parent.context.trace_id);
        assert_ne!(child.context.span_id, parent.context.span_id);
        assert_eq!(child.parent, Some(parent.context.span_id));
    }

    #[test]
    fn child_inherits_sampled_flag() {
        let mut parent = Span::new("parent", TelemetryTier::Ui);
        parent.context.sampled = false;
        let child = Span::child_of(&parent);
        assert!(!child.context.sampled);
    }

    #[test]
    fn span_ids_are_unique() {
        let a = Span::new("a", TelemetryTier::Ui);
        let b = Span::new("b", TelemetryTier::Ui);
        assert_ne!(a.context.span_id, b.context.span_id);
    }
}
