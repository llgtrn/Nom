#![deny(unsafe_code)]

use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// EventKind
// ---------------------------------------------------------------------------

/// All observable event categories emitted by NomCanvas subsystems.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventKind {
    /// A user action on the canvas surface (pan, zoom, select, drag, …).
    CanvasAction { action: String },
    /// A call into the Nom compiler, with elapsed wall-time in milliseconds.
    CompilerInvoke { duration_ms: u64 },
    /// A RAG/vector retrieval query with the requested result count.
    RagQuery { top_k: usize },
    /// An error with a numeric code and human-readable message.
    Error { code: u32, message: String },
    /// Emitted once when a session is established.
    SessionStart,
    /// Emitted once when a session is torn down.
    SessionEnd,
}

// ---------------------------------------------------------------------------
// TelemetryEvent
// ---------------------------------------------------------------------------

/// A single telemetry observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryEvent {
    pub kind: EventKind,
    /// Wall-clock timestamp in milliseconds since the Unix epoch.
    pub timestamp_ms: u64,
    /// Opaque identifier that groups events belonging to one user session.
    pub session_id: u64,
}

impl TelemetryEvent {
    /// Convenience constructor.
    pub fn new(kind: EventKind, timestamp_ms: u64, session_id: u64) -> Self {
        Self { kind, timestamp_ms, session_id }
    }
}

// ---------------------------------------------------------------------------
// TelemetrySink trait
// ---------------------------------------------------------------------------

/// Destination that consumes telemetry events.
///
/// Implementors decide what to do with each event (ignore, log, forward to a
/// remote collector, store in memory for tests, …).
pub trait TelemetrySink {
    fn record(&self, event: TelemetryEvent);
}

// ---------------------------------------------------------------------------
// NullSink — discards every event
// ---------------------------------------------------------------------------

/// No-op sink.  Use in release builds or whenever telemetry is disabled.
pub struct NullSink;

impl TelemetrySink for NullSink {
    #[inline]
    fn record(&self, _event: TelemetryEvent) {}
}

// ---------------------------------------------------------------------------
// InMemorySink — captures events for testing
// ---------------------------------------------------------------------------

/// Thread-safe sink that accumulates events in a `Vec` for later inspection.
#[derive(Clone)]
pub struct InMemorySink {
    events: Arc<Mutex<Vec<TelemetryEvent>>>,
}

impl InMemorySink {
    /// Create a new, empty in-memory sink.
    pub fn new() -> Self {
        Self { events: Arc::new(Mutex::new(Vec::new())) }
    }

    /// Return a cloned snapshot of all recorded events.
    pub fn events(&self) -> Vec<TelemetryEvent> {
        self.events.lock().expect("InMemorySink mutex poisoned").clone()
    }

    /// Return the number of events recorded so far.
    pub fn count(&self) -> usize {
        self.events.lock().expect("InMemorySink mutex poisoned").len()
    }
}

impl Default for InMemorySink {
    fn default() -> Self {
        Self::new()
    }
}

impl TelemetrySink for InMemorySink {
    fn record(&self, event: TelemetryEvent) {
        self.events.lock().expect("InMemorySink mutex poisoned").push(event);
    }
}

// ---------------------------------------------------------------------------
// Telemetry — global-style coordinator
// ---------------------------------------------------------------------------

/// Wraps a `TelemetrySink` and provides a convenient `emit` method.
///
/// Typically held behind an `Arc` so multiple canvas subsystems can share one
/// coordinator without coordination overhead on the call site.
pub struct Telemetry {
    sink: Arc<dyn TelemetrySink + Send + Sync>,
}

impl Telemetry {
    /// Create a new coordinator backed by the given sink.
    pub fn new(sink: Arc<dyn TelemetrySink + Send + Sync>) -> Self {
        Self { sink }
    }

    /// Build a [`TelemetryEvent`] and forward it to the sink.
    pub fn emit(&self, kind: EventKind, timestamp_ms: u64, session_id: u64) {
        self.sink.record(TelemetryEvent::new(kind, timestamp_ms, session_id));
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_sink_does_not_panic() {
        let sink = NullSink;
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 1, 1));
        sink.record(TelemetryEvent::new(
            EventKind::CanvasAction { action: "pan".into() },
            2,
            1,
        ));
        sink.record(TelemetryEvent::new(
            EventKind::CompilerInvoke { duration_ms: 42 },
            3,
            1,
        ));
        sink.record(TelemetryEvent::new(EventKind::RagQuery { top_k: 5 }, 4, 1));
        sink.record(TelemetryEvent::new(
            EventKind::Error { code: 404, message: "not found".into() },
            5,
            1,
        ));
    }

    #[test]
    fn in_memory_sink_records_events() {
        let sink = InMemorySink::new();
        assert_eq!(sink.count(), 0);

        sink.record(TelemetryEvent::new(EventKind::SessionStart, 100, 42));
        sink.record(TelemetryEvent::new(
            EventKind::CanvasAction { action: "zoom".into() },
            101,
            42,
        ));
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 200, 42));

        assert_eq!(sink.count(), 3);

        let events = sink.events();
        assert_eq!(events[0].kind, EventKind::SessionStart);
        assert_eq!(events[0].timestamp_ms, 100);
        assert_eq!(events[0].session_id, 42);
        assert_eq!(
            events[1].kind,
            EventKind::CanvasAction { action: "zoom".into() }
        );
        assert_eq!(events[2].kind, EventKind::SessionEnd);
        assert_eq!(events[2].timestamp_ms, 200);
    }

    #[test]
    fn telemetry_emits_to_sink() {
        let inner = InMemorySink::new();
        // Clone shares the same Arc<Mutex<_>>, so we can observe via `inner`
        // after emitting through `telemetry`.
        let shared: Arc<dyn TelemetrySink + Send + Sync> = Arc::new(inner.clone());
        let telemetry = Telemetry::new(shared);

        telemetry.emit(EventKind::SessionStart, 0, 7);
        telemetry.emit(EventKind::CompilerInvoke { duration_ms: 150 }, 50, 7);
        telemetry.emit(EventKind::RagQuery { top_k: 10 }, 60, 7);
        telemetry.emit(
            EventKind::Error { code: 500, message: "internal".into() },
            70,
            7,
        );
        telemetry.emit(EventKind::SessionEnd, 999, 7);

        assert_eq!(inner.count(), 5);

        let events = inner.events();
        assert_eq!(events[0].kind, EventKind::SessionStart);
        assert_eq!(events[1].kind, EventKind::CompilerInvoke { duration_ms: 150 });
        assert_eq!(events[2].kind, EventKind::RagQuery { top_k: 10 });
        assert_eq!(
            events[3].kind,
            EventKind::Error { code: 500, message: "internal".into() }
        );
        assert_eq!(events[4].kind, EventKind::SessionEnd);
    }

    #[test]
    fn event_kinds_are_distinct() {
        let start = EventKind::SessionStart;
        let end = EventKind::SessionEnd;
        let action = EventKind::CanvasAction { action: "select".into() };
        let compile = EventKind::CompilerInvoke { duration_ms: 0 };
        let rag = EventKind::RagQuery { top_k: 1 };
        let err = EventKind::Error { code: 0, message: String::new() };

        assert_ne!(start, end);
        assert_ne!(start, action);
        assert_ne!(start, compile);
        assert_ne!(start, rag);
        assert_ne!(start, err);
        assert_ne!(end, action);
        assert_ne!(end, compile);
        assert_ne!(end, rag);
        assert_ne!(end, err);
        assert_ne!(action, compile);
        assert_ne!(action, rag);
        assert_ne!(action, err);
        assert_ne!(compile, rag);
        assert_ne!(compile, err);
        assert_ne!(rag, err);

        // Same-variant equality with identical field values.
        assert_eq!(
            EventKind::CanvasAction { action: "pan".into() },
            EventKind::CanvasAction { action: "pan".into() }
        );
        assert_ne!(
            EventKind::CanvasAction { action: "pan".into() },
            EventKind::CanvasAction { action: "zoom".into() }
        );
        assert_eq!(
            EventKind::CompilerInvoke { duration_ms: 10 },
            EventKind::CompilerInvoke { duration_ms: 10 }
        );
        assert_eq!(
            EventKind::RagQuery { top_k: 3 },
            EventKind::RagQuery { top_k: 3 }
        );
        assert_eq!(
            EventKind::Error { code: 1, message: "x".into() },
            EventKind::Error { code: 1, message: "x".into() }
        );
    }
}
