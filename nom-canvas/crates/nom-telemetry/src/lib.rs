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
    /// W3C Trace Context trace-id (16 bytes = 32 lowercase hex chars).
    pub trace_id: [u8; 16],
    /// W3C Trace Context span-id (8 bytes = 16 lowercase hex chars).
    pub span_id: [u8; 8],
}

impl TelemetryEvent {
    /// Convenience constructor (trace_id and span_id default to all-zero).
    pub fn new(kind: EventKind, timestamp_ms: u64, session_id: u64) -> Self {
        Self {
            kind,
            timestamp_ms,
            session_id,
            trace_id: [0u8; 16],
            span_id: [0u8; 8],
        }
    }

    /// Convenience constructor with explicit W3C trace context fields.
    pub fn with_trace(
        kind: EventKind,
        timestamp_ms: u64,
        session_id: u64,
        trace_id: [u8; 16],
        span_id: [u8; 8],
    ) -> Self {
        Self { kind, timestamp_ms, session_id, trace_id, span_id }
    }

    /// Format as a W3C traceparent header value.
    ///
    /// Format: `"00-{trace_id_32hex}-{span_id_16hex}-01"` (sampled).
    pub fn traceparent(&self) -> String {
        let trace = self.trace_id.iter().map(|b| format!("{b:02x}")).collect::<String>();
        let span = self.span_id.iter().map(|b| format!("{b:02x}")).collect::<String>();
        format!("00-{trace}-{span}-01")
    }

    /// Parse a W3C traceparent header into `(trace_id, span_id, flags)`.
    ///
    /// Returns `None` if the header is malformed or the version is not `"00"`.
    pub fn parse_traceparent(s: &str) -> Option<([u8; 16], [u8; 8], u8)> {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 4 || parts[0] != "00" {
            return None;
        }
        let trace = hex_to_16(parts[1])?;
        let span = hex_to_8(parts[2])?;
        let flags = u8::from_str_radix(parts[3], 16).ok()?;
        Some((trace, span, flags))
    }
}

// ---------------------------------------------------------------------------
// Private hex helpers
// ---------------------------------------------------------------------------

fn hex_to_16(s: &str) -> Option<[u8; 16]> {
    if s.len() != 32 {
        return None;
    }
    let mut out = [0u8; 16];
    for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
        let hex = std::str::from_utf8(chunk).ok()?;
        out[i] = u8::from_str_radix(hex, 16).ok()?;
    }
    Some(out)
}

fn hex_to_8(s: &str) -> Option<[u8; 8]> {
    if s.len() != 16 {
        return None;
    }
    let mut out = [0u8; 8];
    for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
        let hex = std::str::from_utf8(chunk).ok()?;
        out[i] = u8::from_str_radix(hex, 16).ok()?;
    }
    Some(out)
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

    // -------------------------------------------------------------------------
    // W3C traceparent tests
    // -------------------------------------------------------------------------

    #[test]
    fn traceparent_format_correct() {
        let trace_id = [
            0x4b, 0xf9, 0x2f, 0x3b, 0x77, 0xb3, 0x41, 0x26,
            0xa8, 0x4c, 0x84, 0x35, 0x4e, 0x70, 0x5a, 0x9c,
        ];
        let span_id = [0x00, 0xf0, 0x67, 0xaa, 0x0b, 0xa9, 0x02, 0xb7];
        let event = TelemetryEvent::with_trace(
            EventKind::SessionStart,
            0,
            1,
            trace_id,
            span_id,
        );
        let tp = event.traceparent();
        // Must be "00-{32 hex}-{16 hex}-01"
        let parts: Vec<&str> = tp.split('-').collect();
        assert_eq!(parts.len(), 4);
        assert_eq!(parts[0], "00");
        assert_eq!(parts[1].len(), 32);
        assert_eq!(parts[2].len(), 16);
        assert_eq!(parts[3], "01");
        // Exact value check
        assert_eq!(tp, "00-4bf92f3b77b34126a84c84354e705a9c-00f067aa0ba902b7-01");
    }

    #[test]
    fn traceparent_parse_valid() {
        let header = "00-4bf92f3b77b34126a84c84354e705a9c-00f067aa0ba902b7-01";
        let result = TelemetryEvent::parse_traceparent(header);
        assert!(result.is_some());
        let (trace, span, flags) = result.unwrap();
        assert_eq!(trace[0], 0x4b);
        assert_eq!(trace[1], 0xf9);
        assert_eq!(span[0], 0x00);
        assert_eq!(span[1], 0xf0);
        assert_eq!(flags, 0x01);
    }

    #[test]
    fn traceparent_parse_invalid_returns_none() {
        // Wrong version prefix
        assert!(TelemetryEvent::parse_traceparent("ff-short-span-01").is_none());
        // Too few parts
        assert!(TelemetryEvent::parse_traceparent("00-4bf92f3b77b34126a84c84354e705a9c-01").is_none());
        // Trace ID too short (not 32 hex chars)
        assert!(TelemetryEvent::parse_traceparent("00-4bf9-00f067aa0ba902b7-01").is_none());
        // Span ID too short (not 16 hex chars)
        assert!(TelemetryEvent::parse_traceparent("00-4bf92f3b77b34126a84c84354e705a9c-00f0-01").is_none());
    }

    // -------------------------------------------------------------------------
    // New coverage tests
    // -------------------------------------------------------------------------

    #[test]
    fn telemetry_event_with_metadata_roundtrip() {
        // TelemetryEvent carries structured data via EventKind::Error (code + message).
        // Verify the key/value round-trips through construction and retrieval.
        let event = TelemetryEvent::new(
            EventKind::Error { code: 42, message: "context=canvas;user=7".into() },
            500,
            99,
        );
        match &event.kind {
            EventKind::Error { code, message } => {
                assert_eq!(*code, 42);
                assert_eq!(message, "context=canvas;user=7");
            }
            other => panic!("unexpected kind: {other:?}"),
        }
        assert_eq!(event.timestamp_ms, 500);
        assert_eq!(event.session_id, 99);
    }

    #[test]
    fn telemetry_event_clone_independence() {
        let mut original = TelemetryEvent::new(
            EventKind::CanvasAction { action: "pan".into() },
            100,
            1,
        );
        let clone = original.clone();

        // Mutate the original's timestamp after cloning.
        original.timestamp_ms = 9999;

        // Clone retains the original value.
        assert_eq!(clone.timestamp_ms, 100);
        assert_ne!(original.timestamp_ms, clone.timestamp_ms);
        // Kind is independent too.
        assert_eq!(clone.kind, EventKind::CanvasAction { action: "pan".into() });
    }

    #[test]
    fn telemetry_multiple_events_different_spans() {
        let span_a = [0x01u8, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let span_b = [0xA1u8, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7, 0xA8];
        let trace = [0u8; 16];

        let event_a = TelemetryEvent::with_trace(EventKind::SessionStart, 10, 1, trace, span_a);
        let event_b = TelemetryEvent::with_trace(EventKind::SessionEnd, 20, 1, trace, span_b);

        assert_ne!(event_a.span_id, event_b.span_id);
        assert_eq!(event_a.trace_id, event_b.trace_id);
    }

    #[test]
    fn telemetry_traceparent_consistent_with_ids() {
        let trace_id: [u8; 16] = [
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef,
            0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54, 0x32, 0x10,
        ];
        let span_id: [u8; 8] = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];

        let event = TelemetryEvent::with_trace(
            EventKind::SessionStart,
            0,
            1,
            trace_id,
            span_id,
        );
        let tp = event.traceparent();

        // Round-trip: parse should give back the same bytes.
        let (parsed_trace, parsed_span, flags) =
            TelemetryEvent::parse_traceparent(&tp).expect("valid traceparent");

        assert_eq!(parsed_trace, event.trace_id);
        assert_eq!(parsed_span, event.span_id);
        assert_eq!(flags, 0x01);
    }

    #[test]
    fn telemetry_event_timestamp_increases() {
        // Two events recorded at t=0 and t=1 must have non-decreasing timestamps.
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 5));
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 1, 5));

        let events = sink.events();
        assert_eq!(events.len(), 2);
        assert!(
            events[1].timestamp_ms >= events[0].timestamp_ms,
            "timestamps must be non-decreasing: {} < {}",
            events[1].timestamp_ms,
            events[0].timestamp_ms,
        );
    }
}
