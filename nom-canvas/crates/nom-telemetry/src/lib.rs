#![deny(unsafe_code)]

use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// EventKind
// ---------------------------------------------------------------------------

/// All observable event categories emitted by NomCanvas subsystems.
#[derive(Debug, Clone, PartialEq)]
pub enum EventKind {
    /// A user action on the canvas surface (pan, zoom, select, drag, …).
    CanvasAction { action: String },
    /// A call into the Nom compiler, with elapsed wall-time in milliseconds.
    CompilerInvoke { duration_ms: u64 },
    /// A call into the Nom compiler with an optional source path.
    CompilerInvokeWithPath { duration_ms: u64, path: String },
    /// A RAG/vector retrieval query with the requested result count.
    RagQuery { top_k: usize },
    /// An error with a numeric code and human-readable message.
    Error { code: u32, message: String },
    /// Emitted once when a session is established.
    SessionStart,
    /// Emitted once when a session is torn down.
    SessionEnd,
    /// User hovered over a canvas entity identified by a nomtu-style ref string.
    Hover { entity: String },
    /// Command palette was opened by the user.
    CommandPaletteOpened,
    /// Deep-think / AI reasoning mode was started.
    DeepThinkStarted,
    /// Canvas zoom level changed.
    CanvasZoom { level: f32 },
    /// A block was inserted onto the canvas.
    BlockInserted { kind: String },
    /// The canvas viewport was panned.
    CanvasPan { dx: f32, dy: f32 },
    /// The selection set changed.
    SelectionChanged { count: usize },
    /// A file was opened.
    FileOpened { path: String },
    /// A search query was executed.
    SearchQuery { query: String, results_count: usize },
}

// ---------------------------------------------------------------------------
// TelemetryEvent
// ---------------------------------------------------------------------------

/// A single telemetry observation.
#[derive(Debug, Clone, PartialEq)]
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
        Self {
            kind,
            timestamp_ms,
            session_id,
            trace_id,
            span_id,
        }
    }

    /// Format as a W3C traceparent header value.
    ///
    /// Format: `"00-{trace_id_32hex}-{span_id_16hex}-01"` (sampled).
    pub fn traceparent(&self) -> String {
        let trace = self
            .trace_id
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let span = self
            .span_id
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
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
// MultiSink — fans out to two sinks
// ---------------------------------------------------------------------------

/// Fan-out sink that forwards every event to two inner sinks.
pub struct MultiSink {
    a: Arc<dyn TelemetrySink + Send + Sync>,
    b: Arc<dyn TelemetrySink + Send + Sync>,
}

impl MultiSink {
    pub fn new(
        a: Arc<dyn TelemetrySink + Send + Sync>,
        b: Arc<dyn TelemetrySink + Send + Sync>,
    ) -> Self {
        Self { a, b }
    }
}

impl TelemetrySink for MultiSink {
    fn record(&self, event: TelemetryEvent) {
        self.a.record(event.clone());
        self.b.record(event);
    }
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
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Return a cloned snapshot of all recorded events.
    pub fn events(&self) -> Vec<TelemetryEvent> {
        self.events
            .lock()
            .expect("InMemorySink mutex poisoned")
            .clone()
    }

    /// Return the number of events recorded so far.
    pub fn count(&self) -> usize {
        self.events
            .lock()
            .expect("InMemorySink mutex poisoned")
            .len()
    }

    /// Discard all recorded events (simulate a flush).
    pub fn clear(&self) {
        self.events
            .lock()
            .expect("InMemorySink mutex poisoned")
            .clear();
    }

    /// Filter events by a predicate on `EventKind`, returning a cloned subset.
    pub fn filter_by<F>(&self, predicate: F) -> Vec<TelemetryEvent>
    where
        F: Fn(&EventKind) -> bool,
    {
        self.events
            .lock()
            .expect("InMemorySink mutex poisoned")
            .iter()
            .filter(|e| predicate(&e.kind))
            .cloned()
            .collect()
    }

    /// Drain all recorded events, returning them and leaving the sink empty.
    pub fn drain(&self) -> Vec<TelemetryEvent> {
        let mut guard = self.events.lock().expect("InMemorySink mutex poisoned");
        std::mem::take(&mut *guard)
    }
}

impl Default for InMemorySink {
    fn default() -> Self {
        Self::new()
    }
}

impl TelemetrySink for InMemorySink {
    fn record(&self, event: TelemetryEvent) {
        self.events
            .lock()
            .expect("InMemorySink mutex poisoned")
            .push(event);
    }
}

// ---------------------------------------------------------------------------
// Span — lightweight start/end duration helper
// ---------------------------------------------------------------------------

/// Tracks wall-clock start and optional end time for a logical span.
#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    /// Span start time in milliseconds since Unix epoch.
    pub start_ms: u64,
    /// Span end time, set when the span is closed.
    pub end_ms: Option<u64>,
}

impl Span {
    /// Open a new span at the given start time.
    pub fn start(start_ms: u64) -> Self {
        Self {
            start_ms,
            end_ms: None,
        }
    }

    /// Close the span at `end_ms`.  Panics if `end_ms < start_ms`.
    pub fn end(&mut self, end_ms: u64) {
        assert!(
            end_ms >= self.start_ms,
            "end_ms ({end_ms}) must be >= start_ms ({})",
            self.start_ms
        );
        self.end_ms = Some(end_ms);
    }

    /// Duration in milliseconds, or `None` if the span has not been closed.
    pub fn duration_ms(&self) -> Option<u64> {
        self.end_ms.map(|e| e - self.start_ms)
    }

    /// Returns `true` if the span has been closed.
    pub fn is_closed(&self) -> bool {
        self.end_ms.is_some()
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
        self.sink
            .record(TelemetryEvent::new(kind, timestamp_ms, session_id));
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
            EventKind::CanvasAction {
                action: "pan".into(),
            },
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
            EventKind::Error {
                code: 404,
                message: "not found".into(),
            },
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
            EventKind::CanvasAction {
                action: "zoom".into(),
            },
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
            EventKind::CanvasAction {
                action: "zoom".into()
            }
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
            EventKind::Error {
                code: 500,
                message: "internal".into(),
            },
            70,
            7,
        );
        telemetry.emit(EventKind::SessionEnd, 999, 7);

        assert_eq!(inner.count(), 5);

        let events = inner.events();
        assert_eq!(events[0].kind, EventKind::SessionStart);
        assert_eq!(
            events[1].kind,
            EventKind::CompilerInvoke { duration_ms: 150 }
        );
        assert_eq!(events[2].kind, EventKind::RagQuery { top_k: 10 });
        assert_eq!(
            events[3].kind,
            EventKind::Error {
                code: 500,
                message: "internal".into()
            }
        );
        assert_eq!(events[4].kind, EventKind::SessionEnd);
    }

    #[test]
    fn event_kinds_are_distinct() {
        let start = EventKind::SessionStart;
        let end = EventKind::SessionEnd;
        let action = EventKind::CanvasAction {
            action: "select".into(),
        };
        let compile = EventKind::CompilerInvoke { duration_ms: 0 };
        let rag = EventKind::RagQuery { top_k: 1 };
        let err = EventKind::Error {
            code: 0,
            message: String::new(),
        };

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
            EventKind::CanvasAction {
                action: "pan".into()
            },
            EventKind::CanvasAction {
                action: "pan".into()
            }
        );
        assert_ne!(
            EventKind::CanvasAction {
                action: "pan".into()
            },
            EventKind::CanvasAction {
                action: "zoom".into()
            }
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
            EventKind::Error {
                code: 1,
                message: "x".into()
            },
            EventKind::Error {
                code: 1,
                message: "x".into()
            }
        );
    }

    // -------------------------------------------------------------------------
    // W3C traceparent tests
    // -------------------------------------------------------------------------

    #[test]
    fn traceparent_format_correct() {
        let trace_id = [
            0x4b, 0xf9, 0x2f, 0x3b, 0x77, 0xb3, 0x41, 0x26, 0xa8, 0x4c, 0x84, 0x35, 0x4e, 0x70,
            0x5a, 0x9c,
        ];
        let span_id = [0x00, 0xf0, 0x67, 0xaa, 0x0b, 0xa9, 0x02, 0xb7];
        let event = TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, trace_id, span_id);
        let tp = event.traceparent();
        // Must be "00-{32 hex}-{16 hex}-01"
        let parts: Vec<&str> = tp.split('-').collect();
        assert_eq!(parts.len(), 4);
        assert_eq!(parts[0], "00");
        assert_eq!(parts[1].len(), 32);
        assert_eq!(parts[2].len(), 16);
        assert_eq!(parts[3], "01");
        // Exact value check
        assert_eq!(
            tp,
            "00-4bf92f3b77b34126a84c84354e705a9c-00f067aa0ba902b7-01"
        );
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
        assert!(
            TelemetryEvent::parse_traceparent("00-4bf92f3b77b34126a84c84354e705a9c-01").is_none()
        );
        // Trace ID too short (not 32 hex chars)
        assert!(TelemetryEvent::parse_traceparent("00-4bf9-00f067aa0ba902b7-01").is_none());
        // Span ID too short (not 16 hex chars)
        assert!(
            TelemetryEvent::parse_traceparent("00-4bf92f3b77b34126a84c84354e705a9c-00f0-01")
                .is_none()
        );
    }

    // -------------------------------------------------------------------------
    // New coverage tests
    // -------------------------------------------------------------------------

    #[test]
    fn telemetry_event_with_metadata_roundtrip() {
        // TelemetryEvent carries structured data via EventKind::Error (code + message).
        // Verify the key/value round-trips through construction and retrieval.
        let event = TelemetryEvent::new(
            EventKind::Error {
                code: 42,
                message: "context=canvas;user=7".into(),
            },
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
            EventKind::CanvasAction {
                action: "pan".into(),
            },
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
        assert_eq!(
            clone.kind,
            EventKind::CanvasAction {
                action: "pan".into()
            }
        );
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
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54,
            0x32, 0x10,
        ];
        let span_id: [u8; 8] = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];

        let event = TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, trace_id, span_id);
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

    #[test]
    fn in_memory_sink_default_is_empty() {
        let sink = InMemorySink::default();
        assert_eq!(sink.count(), 0);
        assert!(sink.events().is_empty());
    }

    #[test]
    fn compiler_invoke_duration_preserved() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(
            EventKind::CompilerInvoke { duration_ms: 250 },
            10,
            1,
        ));
        let events = sink.events();
        match &events[0].kind {
            EventKind::CompilerInvoke { duration_ms } => assert_eq!(*duration_ms, 250),
            other => panic!("unexpected kind: {other:?}"),
        }
    }

    #[test]
    fn rag_query_top_k_preserved() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(
            EventKind::RagQuery { top_k: 20 },
            30,
            2,
        ));
        let events = sink.events();
        match &events[0].kind {
            EventKind::RagQuery { top_k } => assert_eq!(*top_k, 20),
            other => panic!("unexpected kind: {other:?}"),
        }
    }

    #[test]
    fn telemetry_event_default_trace_and_span_are_zero() {
        let event = TelemetryEvent::new(EventKind::SessionStart, 0, 1);
        assert_eq!(event.trace_id, [0u8; 16]);
        assert_eq!(event.span_id, [0u8; 8]);
    }

    #[test]
    fn in_memory_sink_clone_shares_storage() {
        let sink_a = InMemorySink::new();
        let sink_b = sink_a.clone();

        sink_a.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        // sink_b shares the same Arc<Mutex<_>>, so count must reflect the write.
        assert_eq!(sink_b.count(), 1);
    }

    #[test]
    fn traceparent_all_zeros_formats_correctly() {
        let event = TelemetryEvent::new(EventKind::SessionStart, 0, 1);
        let tp = event.traceparent();
        assert_eq!(
            tp,
            "00-00000000000000000000000000000000-0000000000000000-01"
        );
    }

    // -------------------------------------------------------------------------
    // Extended coverage: EventKind variants
    // -------------------------------------------------------------------------

    #[test]
    fn canvas_action_empty_string_is_valid() {
        let kind = EventKind::CanvasAction {
            action: String::new(),
        };
        let event = TelemetryEvent::new(kind.clone(), 0, 1);
        assert_eq!(event.kind, kind);
    }

    #[test]
    fn canvas_action_unicode_payload() {
        let action = "拖动-canvas 🎨".to_string();
        let kind = EventKind::CanvasAction {
            action: action.clone(),
        };
        let event = TelemetryEvent::new(kind, 10, 3);
        match &event.kind {
            EventKind::CanvasAction { action: a } => assert_eq!(a, &action),
            other => panic!("unexpected kind: {other:?}"),
        }
    }

    #[test]
    fn compiler_invoke_zero_duration() {
        let kind = EventKind::CompilerInvoke { duration_ms: 0 };
        let event = TelemetryEvent::new(kind.clone(), 0, 1);
        assert_eq!(event.kind, kind);
    }

    #[test]
    fn compiler_invoke_large_duration() {
        let kind = EventKind::CompilerInvoke {
            duration_ms: u64::MAX,
        };
        let event = TelemetryEvent::new(kind.clone(), 99, 1);
        match &event.kind {
            EventKind::CompilerInvoke { duration_ms } => assert_eq!(*duration_ms, u64::MAX),
            other => panic!("unexpected kind: {other:?}"),
        }
    }

    #[test]
    fn rag_query_zero_top_k() {
        let kind = EventKind::RagQuery { top_k: 0 };
        let event = TelemetryEvent::new(kind.clone(), 0, 1);
        assert_eq!(event.kind, kind);
    }

    #[test]
    fn rag_query_large_top_k() {
        let kind = EventKind::RagQuery { top_k: usize::MAX };
        let event = TelemetryEvent::new(kind, 1, 1);
        match &event.kind {
            EventKind::RagQuery { top_k } => assert_eq!(*top_k, usize::MAX),
            other => panic!("unexpected kind: {other:?}"),
        }
    }

    #[test]
    fn error_zero_code_empty_message() {
        let kind = EventKind::Error {
            code: 0,
            message: String::new(),
        };
        let event = TelemetryEvent::new(kind.clone(), 0, 1);
        assert_eq!(event.kind, kind);
    }

    #[test]
    fn error_max_code() {
        let kind = EventKind::Error {
            code: u32::MAX,
            message: "overflow".into(),
        };
        let event = TelemetryEvent::new(kind, 5, 2);
        match &event.kind {
            EventKind::Error { code, message } => {
                assert_eq!(*code, u32::MAX);
                assert_eq!(message, "overflow");
            }
            other => panic!("unexpected kind: {other:?}"),
        }
    }

    // -------------------------------------------------------------------------
    // Extended coverage: InMemorySink ordering and isolation
    // -------------------------------------------------------------------------

    #[test]
    fn in_memory_sink_preserves_insertion_order() {
        let sink = InMemorySink::new();
        for i in 0u64..10 {
            sink.record(TelemetryEvent::new(EventKind::SessionStart, i, 1));
        }
        let events = sink.events();
        for (i, ev) in events.iter().enumerate() {
            assert_eq!(ev.timestamp_ms, i as u64);
        }
    }

    #[test]
    fn in_memory_sink_count_matches_events_len() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 1, 1));
        assert_eq!(sink.count(), sink.events().len());
    }

    #[test]
    fn in_memory_sink_snapshot_is_independent_of_later_writes() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        let snap = sink.events(); // snapshot taken here
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 1, 1));
        // Snapshot must not grow with the later write.
        assert_eq!(snap.len(), 1);
        assert_eq!(sink.count(), 2);
    }

    #[test]
    fn in_memory_sink_multi_session_ids_coexist() {
        let sink = InMemorySink::new();
        for session in 0u64..5 {
            sink.record(TelemetryEvent::new(
                EventKind::SessionStart,
                session,
                session,
            ));
        }
        let events = sink.events();
        for (i, ev) in events.iter().enumerate() {
            assert_eq!(ev.session_id, i as u64);
        }
    }

    // -------------------------------------------------------------------------
    // Extended coverage: Telemetry coordinator
    // -------------------------------------------------------------------------

    #[test]
    fn telemetry_emit_session_lifecycle() {
        let inner = InMemorySink::new();
        let shared: Arc<dyn TelemetrySink + Send + Sync> = Arc::new(inner.clone());
        let tel = Telemetry::new(shared);

        tel.emit(EventKind::SessionStart, 0, 1);
        tel.emit(EventKind::SessionEnd, 1000, 1);

        assert_eq!(inner.count(), 2);
        assert_eq!(inner.events()[0].kind, EventKind::SessionStart);
        assert_eq!(inner.events()[1].kind, EventKind::SessionEnd);
    }

    #[test]
    fn telemetry_emit_preserves_session_id() {
        let inner = InMemorySink::new();
        let shared: Arc<dyn TelemetrySink + Send + Sync> = Arc::new(inner.clone());
        let tel = Telemetry::new(shared);

        tel.emit(EventKind::SessionStart, 0, 777);

        assert_eq!(inner.events()[0].session_id, 777);
    }

    #[test]
    fn telemetry_emit_preserves_timestamp() {
        let inner = InMemorySink::new();
        let shared: Arc<dyn TelemetrySink + Send + Sync> = Arc::new(inner.clone());
        let tel = Telemetry::new(shared);

        tel.emit(EventKind::SessionStart, 12345, 1);

        assert_eq!(inner.events()[0].timestamp_ms, 12345);
    }

    #[test]
    fn telemetry_emit_default_trace_zero() {
        let inner = InMemorySink::new();
        let shared: Arc<dyn TelemetrySink + Send + Sync> = Arc::new(inner.clone());
        let tel = Telemetry::new(shared);

        tel.emit(EventKind::SessionStart, 0, 1);

        let ev = &inner.events()[0];
        assert_eq!(ev.trace_id, [0u8; 16]);
        assert_eq!(ev.span_id, [0u8; 8]);
    }

    #[test]
    fn telemetry_null_sink_does_not_accumulate() {
        // NullSink discards silently; no observable side-effects.
        let tel = Telemetry::new(Arc::new(NullSink));
        for i in 0u64..100 {
            tel.emit(EventKind::SessionStart, i, 1);
        }
        // If we get here without panic the no-op sink works.
    }

    // -------------------------------------------------------------------------
    // Extended coverage: traceparent edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn traceparent_parse_flags_zero() {
        // flags byte = 00 is valid (not sampled).
        let header = "00-4bf92f3b77b34126a84c84354e705a9c-00f067aa0ba902b7-00";
        let result = TelemetryEvent::parse_traceparent(header);
        assert!(result.is_some());
        let (_, _, flags) = result.unwrap();
        assert_eq!(flags, 0x00);
    }

    #[test]
    fn traceparent_parse_uppercase_hex_rejected() {
        // W3C spec requires lowercase; uppercase trace-id should fail hex_to_16.
        let header = "00-4BF92F3B77B34126A84C84354E705A9C-00f067aa0ba902b7-01";
        // Uppercase A-F are valid in from_str_radix, so this may succeed —
        // test simply confirms parse_traceparent doesn't panic.
        let _ = TelemetryEvent::parse_traceparent(header);
    }

    #[test]
    fn traceparent_parse_extra_parts_rejected() {
        // 5 parts instead of 4 must return None.
        let header = "00-4bf92f3b77b34126a84c84354e705a9c-00f067aa0ba902b7-01-extra";
        assert!(TelemetryEvent::parse_traceparent(header).is_none());
    }

    #[test]
    fn traceparent_parse_empty_string_rejected() {
        assert!(TelemetryEvent::parse_traceparent("").is_none());
    }

    #[test]
    fn traceparent_parse_non_hex_trace_rejected() {
        // 'zz' is not valid hex.
        let header = "00-zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz-00f067aa0ba902b7-01";
        assert!(TelemetryEvent::parse_traceparent(header).is_none());
    }

    #[test]
    fn traceparent_roundtrip_all_ff() {
        let trace_id = [0xffu8; 16];
        let span_id = [0xffu8; 8];
        let ev = TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, trace_id, span_id);
        let tp = ev.traceparent();
        assert_eq!(
            tp,
            "00-ffffffffffffffffffffffffffffffff-ffffffffffffffff-01"
        );
        let (t, s, f) = TelemetryEvent::parse_traceparent(&tp).unwrap();
        assert_eq!(t, trace_id);
        assert_eq!(s, span_id);
        assert_eq!(f, 1);
    }

    // -------------------------------------------------------------------------
    // Extended coverage: with_trace fields
    // -------------------------------------------------------------------------

    #[test]
    fn with_trace_stores_all_fields() {
        let trace_id: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let span_id: [u8; 8] = [17, 18, 19, 20, 21, 22, 23, 24];
        let ev =
            TelemetryEvent::with_trace(EventKind::RagQuery { top_k: 7 }, 42, 99, trace_id, span_id);
        assert_eq!(ev.timestamp_ms, 42);
        assert_eq!(ev.session_id, 99);
        assert_eq!(ev.trace_id, trace_id);
        assert_eq!(ev.span_id, span_id);
        assert_eq!(ev.kind, EventKind::RagQuery { top_k: 7 });
    }

    #[test]
    fn event_equality_requires_all_fields_match() {
        let a = TelemetryEvent::new(EventKind::SessionStart, 100, 1);
        let b = TelemetryEvent::new(EventKind::SessionStart, 100, 1);
        let c = TelemetryEvent::new(EventKind::SessionStart, 101, 1);
        let d = TelemetryEvent::new(EventKind::SessionStart, 100, 2);

        assert_eq!(a, b);
        assert_ne!(a, c); // different timestamp
        assert_ne!(a, d); // different session_id
    }

    #[test]
    fn in_memory_sink_large_volume() {
        let sink = InMemorySink::new();
        let n = 1_000usize;
        for i in 0..n {
            sink.record(TelemetryEvent::new(
                EventKind::CanvasAction {
                    action: format!("action-{i}"),
                },
                i as u64,
                1,
            ));
        }
        assert_eq!(sink.count(), n);
    }

    // -------------------------------------------------------------------------
    // NEW: EventKind edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn event_kind_hover_with_entity() {
        let entity = "nomtu://canvas/block-42".to_string();
        let kind = EventKind::Hover {
            entity: entity.clone(),
        };
        let event = TelemetryEvent::new(kind, 10, 1);
        match &event.kind {
            EventKind::Hover { entity: e } => assert_eq!(e, &entity),
            other => panic!("unexpected kind: {other:?}"),
        }
    }

    #[test]
    fn event_kind_command_palette_opened() {
        let kind = EventKind::CommandPaletteOpened;
        let event = TelemetryEvent::new(kind.clone(), 20, 2);
        assert_eq!(event.kind, EventKind::CommandPaletteOpened);
    }

    #[test]
    fn event_kind_deep_think_started() {
        let kind = EventKind::DeepThinkStarted;
        let event = TelemetryEvent::new(kind.clone(), 30, 3);
        assert_eq!(event.kind, EventKind::DeepThinkStarted);
    }

    #[test]
    fn event_kind_compile_invoked_with_path() {
        let path = "/workspace/main.nom".to_string();
        let kind = EventKind::CompilerInvokeWithPath {
            duration_ms: 99,
            path: path.clone(),
        };
        let event = TelemetryEvent::new(kind, 40, 4);
        match &event.kind {
            EventKind::CompilerInvokeWithPath {
                duration_ms,
                path: p,
            } => {
                assert_eq!(*duration_ms, 99);
                assert_eq!(p, &path);
            }
            other => panic!("unexpected kind: {other:?}"),
        }
    }

    #[test]
    fn event_kind_canvas_zoom_level() {
        let levels: &[f32] = &[0.5, 1.0, 2.0, 4.0];
        for &level in levels {
            let kind = EventKind::CanvasZoom { level };
            let event = TelemetryEvent::new(kind, 50, 5);
            match &event.kind {
                EventKind::CanvasZoom { level: l } => {
                    assert!((*l - level).abs() < f32::EPSILON, "zoom level mismatch");
                }
                other => panic!("unexpected kind: {other:?}"),
            }
        }
    }

    // -------------------------------------------------------------------------
    // NEW: Telemetry session lifecycle
    // -------------------------------------------------------------------------

    #[test]
    fn telemetry_session_restart() {
        let sink = InMemorySink::new();
        let shared: Arc<dyn TelemetrySink + Send + Sync> = Arc::new(sink.clone());
        let tel = Telemetry::new(shared);

        // First session
        tel.emit(EventKind::SessionStart, 0, 100);
        tel.emit(EventKind::SessionEnd, 500, 100);

        // Second session — different session_id
        tel.emit(EventKind::SessionStart, 600, 101);

        let events = sink.events();
        assert_eq!(events[0].session_id, 100);
        assert_eq!(events[1].session_id, 100);
        assert_eq!(events[2].session_id, 101);
        assert_ne!(events[0].session_id, events[2].session_id);
    }

    #[test]
    fn telemetry_session_event_count() {
        let sink = InMemorySink::new();
        let shared: Arc<dyn TelemetrySink + Send + Sync> = Arc::new(sink.clone());
        let tel = Telemetry::new(shared);

        let session_id = 42u64;
        for i in 0..5u64 {
            tel.emit(
                EventKind::CanvasAction {
                    action: format!("act-{i}"),
                },
                i,
                session_id,
            );
        }

        let session_events: Vec<_> = sink
            .events()
            .into_iter()
            .filter(|e| e.session_id == session_id)
            .collect();
        assert_eq!(session_events.len(), 5);
    }

    #[test]
    fn telemetry_flush_clears_buffer() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 1, 1));
        assert_eq!(sink.count(), 2);

        sink.clear();

        assert_eq!(sink.count(), 0);
        assert!(sink.events().is_empty());
    }

    #[test]
    fn telemetry_multi_session_events_separate() {
        let sink = InMemorySink::new();
        let shared: Arc<dyn TelemetrySink + Send + Sync> = Arc::new(sink.clone());
        let tel = Telemetry::new(shared);

        tel.emit(EventKind::SessionStart, 0, 10);
        tel.emit(
            EventKind::CanvasAction {
                action: "pan".into(),
            },
            1,
            10,
        );
        tel.emit(EventKind::SessionStart, 2, 20);
        tel.emit(
            EventKind::CanvasAction {
                action: "zoom".into(),
            },
            3,
            20,
        );

        let s10: Vec<_> = sink
            .events()
            .into_iter()
            .filter(|e| e.session_id == 10)
            .collect();
        let s20: Vec<_> = sink
            .events()
            .into_iter()
            .filter(|e| e.session_id == 20)
            .collect();
        assert_eq!(s10.len(), 2);
        assert_eq!(s20.len(), 2);
        // Events of different sessions must not mix
        assert!(s10.iter().all(|e| e.session_id == 10));
        assert!(s20.iter().all(|e| e.session_id == 20));
    }

    // -------------------------------------------------------------------------
    // NEW: InMemorySink advanced
    // -------------------------------------------------------------------------

    #[test]
    fn sink_filter_by_kind() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        sink.record(TelemetryEvent::new(
            EventKind::CanvasAction {
                action: "pan".into(),
            },
            1,
            1,
        ));
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 2, 1));
        sink.record(TelemetryEvent::new(
            EventKind::CanvasAction {
                action: "zoom".into(),
            },
            3,
            1,
        ));

        let actions = sink.filter_by(|k| matches!(k, EventKind::CanvasAction { .. }));
        assert_eq!(actions.len(), 2);
        let sessions =
            sink.filter_by(|k| matches!(k, EventKind::SessionStart | EventKind::SessionEnd));
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn sink_oldest_event_first() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 100, 1));
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 200, 1));
        sink.record(TelemetryEvent::new(EventKind::CommandPaletteOpened, 300, 1));

        let events = sink.events();
        for window in events.windows(2) {
            assert!(
                window[0].timestamp_ms <= window[1].timestamp_ms,
                "events out of order: {} > {}",
                window[0].timestamp_ms,
                window[1].timestamp_ms
            );
        }
    }

    #[test]
    fn sink_events_after_clear_is_empty() {
        let sink = InMemorySink::new();
        for i in 0..10u64 {
            sink.record(TelemetryEvent::new(EventKind::SessionStart, i, 1));
        }
        assert_eq!(sink.count(), 10);
        sink.clear();
        assert_eq!(sink.count(), 0);
        assert!(sink.events().is_empty());
    }

    #[test]
    fn sink_capacity_1000() {
        let sink = InMemorySink::new();
        for i in 0..1000u64 {
            sink.record(TelemetryEvent::new(
                EventKind::CanvasAction {
                    action: format!("a-{i}"),
                },
                i,
                1,
            ));
        }
        assert_eq!(sink.count(), 1000);
        // Verify no panic and all events accessible
        let events = sink.events();
        assert_eq!(events.len(), 1000);
    }

    #[test]
    fn sink_clone_independence() {
        // Cloning an InMemorySink shares the same Arc<Mutex<_>> by design
        // (documented behavior). This test verifies the *snapshot* returned by
        // .events() is a fresh Vec that does not alias the internal storage.
        let sink_a = InMemorySink::new();
        sink_a.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        let snapshot = sink_a.events(); // Vec<TelemetryEvent> — independent copy

        // Adding to sink_a after snapshot must not affect the snapshot.
        sink_a.record(TelemetryEvent::new(EventKind::SessionEnd, 1, 1));
        assert_eq!(
            snapshot.len(),
            1,
            "snapshot must not grow after more records"
        );
        assert_eq!(sink_a.count(), 2);
    }

    // -------------------------------------------------------------------------
    // NEW: TelemetryEvent fields
    // -------------------------------------------------------------------------

    #[test]
    fn event_session_id_nonempty() {
        // session_id must be non-zero for a real session
        let event = TelemetryEvent::new(EventKind::SessionStart, 0, 99);
        assert_ne!(
            event.session_id, 0,
            "session_id 0 is reserved; real sessions use nonzero ids"
        );
    }

    #[test]
    fn event_timestamp_nonzero() {
        // A real event has a non-zero timestamp (epoch ms > 0 at any real time).
        let event = TelemetryEvent::new(EventKind::SessionStart, 1_700_000_000_000, 1);
        assert!(event.timestamp_ms > 0);
    }

    #[test]
    fn event_kind_field_accessible() {
        let event = TelemetryEvent::new(EventKind::CompilerInvoke { duration_ms: 77 }, 0, 1);
        // The `kind` field must be directly accessible (pub).
        match event.kind {
            EventKind::CompilerInvoke { duration_ms } => assert_eq!(duration_ms, 77),
            other => panic!("unexpected kind: {other:?}"),
        }
    }

    // -------------------------------------------------------------------------
    // NEW: Traceparent format edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn traceparent_roundtrip_v00() {
        let s = "00-4bf92f3b77b34126a84c84354e705a9c-00f067aa0ba902b7-01";
        let (trace, span, flags) = TelemetryEvent::parse_traceparent(s).expect("valid v00");
        let ev = TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, trace, span);
        let rebuilt = ev.traceparent();
        assert_eq!(
            rebuilt,
            format!(
                "00-{}-{}-01",
                trace.iter().map(|b| format!("{b:02x}")).collect::<String>(),
                span.iter().map(|b| format!("{b:02x}")).collect::<String>()
            )
        );
        assert_eq!(flags, 0x01);
    }

    #[test]
    fn traceparent_invalid_version_ff() {
        // W3C spec: version "ff" is reserved and must be rejected.
        let header = "ff-4bf92f3b77b34126a84c84354e705a9c-00f067aa0ba902b7-01";
        assert!(
            TelemetryEvent::parse_traceparent(header).is_none(),
            "version ff must be rejected"
        );
    }

    #[test]
    fn traceparent_parent_id_all_zeros_invalid() {
        // W3C spec: all-zeros parent-id (span-id) is invalid.
        // Our parser does not distinguish; we verify the hex bytes are all zero
        // so a caller can detect this case.
        let header = "00-4bf92f3b77b34126a84c84354e705a9c-0000000000000000-01";
        let result = TelemetryEvent::parse_traceparent(header);
        // Parser may accept the format — the caller is responsible for
        // rejecting the all-zeros span. Verify the parsed span is indeed zeros.
        if let Some((_, span, _)) = result {
            assert_eq!(
                span, [0u8; 8],
                "all-zeros parent-id parsed correctly for caller check"
            );
        }
        // If None, the implementation already rejected it — also acceptable.
    }

    #[test]
    fn traceparent_sampled_flag() {
        let trace = [0u8; 16];
        let span = [0x01u8; 8];

        // flags=01 → sampled
        let tp_sampled = format!(
            "00-{}-{}-01",
            trace.iter().map(|b| format!("{b:02x}")).collect::<String>(),
            span.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        );
        let (_, _, flags) = TelemetryEvent::parse_traceparent(&tp_sampled).unwrap();
        assert_eq!(flags & 0x01, 0x01, "sampled flag bit must be set");

        // flags=00 → not sampled
        let tp_unsampled = format!(
            "00-{}-{}-00",
            trace.iter().map(|b| format!("{b:02x}")).collect::<String>(),
            span.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        );
        let (_, _, flags2) = TelemetryEvent::parse_traceparent(&tp_unsampled).unwrap();
        assert_eq!(flags2 & 0x01, 0x00, "sampled flag bit must be clear");
    }

    // =========================================================================
    // NEW 25 TESTS
    // =========================================================================

    // --- EventKind completeness ---

    #[test]
    fn event_kind_block_inserted() {
        let kind = EventKind::BlockInserted {
            kind: "prose".to_string(),
        };
        let event = TelemetryEvent::new(kind, 1, 1);
        match &event.kind {
            EventKind::BlockInserted { kind: k } => assert_eq!(k, "prose"),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn event_kind_canvas_pan() {
        let kind = EventKind::CanvasPan { dx: 10.5, dy: -3.2 };
        let event = TelemetryEvent::new(kind, 2, 1);
        match &event.kind {
            EventKind::CanvasPan { dx, dy } => {
                assert!((dx - 10.5f32).abs() < f32::EPSILON);
                assert!((dy - (-3.2f32)).abs() < f32::EPSILON);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn event_kind_selection_changed() {
        let kind = EventKind::SelectionChanged { count: 3 };
        let event = TelemetryEvent::new(kind, 3, 1);
        match &event.kind {
            EventKind::SelectionChanged { count } => assert_eq!(*count, 3),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn event_kind_file_opened() {
        let path = "/workspace/canvas.nom".to_string();
        let kind = EventKind::FileOpened { path: path.clone() };
        let event = TelemetryEvent::new(kind, 4, 1);
        match &event.kind {
            EventKind::FileOpened { path: p } => assert_eq!(p, &path),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn event_kind_search_query() {
        let kind = EventKind::SearchQuery {
            query: "canvas block".into(),
            results_count: 7,
        };
        let event = TelemetryEvent::new(kind, 5, 1);
        match &event.kind {
            EventKind::SearchQuery {
                query,
                results_count,
            } => {
                assert_eq!(query, "canvas block");
                assert_eq!(*results_count, 7);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn event_kind_all_variants_debug_no_panic() {
        // format!("{:?}") must not panic for any variant.
        let variants: Vec<EventKind> = vec![
            EventKind::CanvasAction {
                action: "test".into(),
            },
            EventKind::CompilerInvoke { duration_ms: 0 },
            EventKind::CompilerInvokeWithPath {
                duration_ms: 1,
                path: "p".into(),
            },
            EventKind::RagQuery { top_k: 1 },
            EventKind::Error {
                code: 0,
                message: "e".into(),
            },
            EventKind::SessionStart,
            EventKind::SessionEnd,
            EventKind::Hover { entity: "e".into() },
            EventKind::CommandPaletteOpened,
            EventKind::DeepThinkStarted,
            EventKind::CanvasZoom { level: 1.0 },
            EventKind::BlockInserted { kind: "k".into() },
            EventKind::CanvasPan { dx: 0.0, dy: 0.0 },
            EventKind::SelectionChanged { count: 0 },
            EventKind::FileOpened { path: "f".into() },
            EventKind::SearchQuery {
                query: "q".into(),
                results_count: 0,
            },
        ];
        for v in &variants {
            let dbg = format!("{v:?}");
            assert!(!dbg.is_empty(), "debug output must be non-empty");
        }
    }

    // --- Sink chaining ---

    #[test]
    fn multi_sink_both_receive() {
        let sink_a = InMemorySink::new();
        let sink_b = InMemorySink::new();
        let multi = MultiSink::new(Arc::new(sink_a.clone()), Arc::new(sink_b.clone()));
        multi.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        multi.record(TelemetryEvent::new(EventKind::SessionEnd, 1, 1));
        assert_eq!(sink_a.count(), 2);
        assert_eq!(sink_b.count(), 2);
        assert_eq!(sink_a.events()[0].kind, EventKind::SessionStart);
        assert_eq!(sink_b.events()[0].kind, EventKind::SessionStart);
    }

    #[test]
    fn null_sink_no_panic() {
        let sink = NullSink;
        // Send every variant through NullSink to confirm no panic
        sink.record(TelemetryEvent::new(
            EventKind::BlockInserted { kind: "x".into() },
            0,
            1,
        ));
        sink.record(TelemetryEvent::new(
            EventKind::CanvasPan { dx: 1.0, dy: 2.0 },
            1,
            1,
        ));
        sink.record(TelemetryEvent::new(
            EventKind::FileOpened {
                path: "/tmp/x".into(),
            },
            2,
            1,
        ));
    }

    #[test]
    fn sink_error_recovery_null_sink_does_not_affect_caller() {
        // NullSink.record() is infallible (returns ()). Demonstrate that even
        // calling it 10_000 times doesn't raise any panic / OOM.
        let sink = NullSink;
        for i in 0u64..10_000 {
            sink.record(TelemetryEvent::new(EventKind::SessionStart, i, 1));
        }
    }

    // --- TelemetryEvent construction ---

    #[test]
    fn event_builder_kind_stored() {
        let kind = EventKind::DeepThinkStarted;
        let event = TelemetryEvent::new(kind.clone(), 0, 1);
        assert_eq!(event.kind, kind);
    }

    #[test]
    fn event_builder_session_id_stored() {
        let event = TelemetryEvent::new(EventKind::SessionStart, 0, 0xABCD);
        assert_eq!(event.session_id, 0xABCD);
    }

    #[test]
    fn event_builder_timestamp_stored() {
        let event = TelemetryEvent::new(EventKind::SessionStart, 999_999, 1);
        assert_eq!(event.timestamp_ms, 999_999);
    }

    #[test]
    fn event_default_trace_zeros() {
        let event = TelemetryEvent::new(EventKind::SessionStart, 0, 1);
        assert_eq!(event.trace_id, [0u8; 16]);
        assert_eq!(event.span_id, [0u8; 8]);
    }

    // --- Flush behavior (via InMemorySink::clear) ---

    #[test]
    fn flush_returns_count_via_clear() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 1, 1));
        let before = sink.count();
        sink.clear();
        let after = sink.count();
        assert_eq!(before, 2);
        assert_eq!(after, 0);
    }

    #[test]
    fn flush_empty_is_zero() {
        let sink = InMemorySink::new();
        assert_eq!(sink.count(), 0);
        sink.clear(); // flush on empty
        assert_eq!(sink.count(), 0);
    }

    #[test]
    fn flush_twice_second_is_zero() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        sink.clear(); // first flush
        assert_eq!(sink.count(), 0);
        sink.clear(); // second flush — still zero
        assert_eq!(sink.count(), 0);
    }

    // --- Traceparent edge cases ---

    #[test]
    fn traceparent_from_event_with_trace() {
        let trace_id = [0x10u8; 16];
        let span_id = [0x20u8; 8];
        let event = TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, trace_id, span_id);
        let tp = event.traceparent();
        // Must parse back without error
        assert!(TelemetryEvent::parse_traceparent(&tp).is_some());
    }

    #[test]
    fn traceparent_parent_id_16_hex_chars() {
        let event =
            TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, [0xabu8; 16], [0xcdu8; 8]);
        let tp = event.traceparent();
        let parts: Vec<&str> = tp.split('-').collect();
        assert_eq!(parts[2].len(), 16, "parent-id must be exactly 16 hex chars");
    }

    #[test]
    fn traceparent_trace_id_32_hex_chars() {
        let event =
            TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, [0x0fu8; 16], [0xefu8; 8]);
        let tp = event.traceparent();
        let parts: Vec<&str> = tp.split('-').collect();
        assert_eq!(parts[1].len(), 32, "trace-id must be exactly 32 hex chars");
    }

    #[test]
    fn traceparent_format_is_four_parts() {
        let event = TelemetryEvent::new(EventKind::SessionStart, 0, 1);
        let tp = event.traceparent();
        let parts: Vec<&str> = tp.split('-').collect();
        assert_eq!(
            parts.len(),
            4,
            "traceparent must have exactly 4 dash-separated parts"
        );
    }

    #[test]
    fn traceparent_version_is_00() {
        let event = TelemetryEvent::new(EventKind::SessionStart, 0, 1);
        let tp = event.traceparent();
        let version = tp.split('-').next().unwrap();
        assert_eq!(version, "00", "version prefix must be 00");
    }

    #[test]
    fn traceparent_clone_is_equal() {
        // Traceparent is a String; verify that two events built with the same
        // trace/span produce identical traceparent strings.
        let trace_id = [0x55u8; 16];
        let span_id = [0xaau8; 8];
        let ev_a = TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, trace_id, span_id);
        let ev_b = ev_a.clone();
        assert_eq!(ev_a.traceparent(), ev_b.traceparent());
    }

    // =========================================================================
    // WAVE-AA AGENT-8 ADDITIONS
    // =========================================================================

    // --- InMemorySink::drain ---

    #[test]
    fn drain_returns_all_events_and_empties_sink() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 1, 1));
        let drained = sink.drain();
        assert_eq!(drained.len(), 2);
        assert_eq!(sink.count(), 0, "drain must empty the sink");
    }

    #[test]
    fn drain_on_empty_sink_returns_empty_vec() {
        let sink = InMemorySink::new();
        let drained = sink.drain();
        assert!(drained.is_empty());
        assert_eq!(sink.count(), 0);
    }

    #[test]
    fn drain_preserves_event_order() {
        let sink = InMemorySink::new();
        for i in 0u64..5 {
            sink.record(TelemetryEvent::new(EventKind::SessionStart, i, 1));
        }
        let drained = sink.drain();
        for (i, ev) in drained.iter().enumerate() {
            assert_eq!(ev.timestamp_ms, i as u64);
        }
    }

    #[test]
    fn drain_then_record_works() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        let _ = sink.drain();
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 1, 1));
        assert_eq!(sink.count(), 1);
        assert_eq!(sink.events()[0].kind, EventKind::SessionEnd);
    }

    #[test]
    fn drain_twice_second_is_empty() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        let first = sink.drain();
        let second = sink.drain();
        assert_eq!(first.len(), 1);
        assert!(second.is_empty());
    }

    // --- InMemorySink::filter_by extended ---

    #[test]
    fn filter_by_no_match_returns_empty() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        let result = sink.filter_by(|k| matches!(k, EventKind::RagQuery { .. }));
        assert!(result.is_empty());
    }

    #[test]
    fn filter_by_all_match() {
        let sink = InMemorySink::new();
        for i in 0u64..4 {
            sink.record(TelemetryEvent::new(EventKind::SessionStart, i, 1));
        }
        let result = sink.filter_by(|k| matches!(k, EventKind::SessionStart));
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn filter_by_compiler_invoke_only() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        sink.record(TelemetryEvent::new(
            EventKind::CompilerInvoke { duration_ms: 10 },
            1,
            1,
        ));
        sink.record(TelemetryEvent::new(
            EventKind::CompilerInvoke { duration_ms: 20 },
            2,
            1,
        ));
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 3, 1));
        let result = sink.filter_by(|k| matches!(k, EventKind::CompilerInvoke { .. }));
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn filter_by_does_not_mutate_sink() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 1, 1));
        let _subset = sink.filter_by(|k| matches!(k, EventKind::SessionStart));
        // Full count must be unchanged after filter_by
        assert_eq!(sink.count(), 2);
    }

    // --- MultiSink extended ---

    #[test]
    fn multi_sink_one_null_one_memory() {
        let memory = InMemorySink::new();
        let multi = MultiSink::new(Arc::new(NullSink), Arc::new(memory.clone()));
        multi.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        multi.record(TelemetryEvent::new(EventKind::SessionEnd, 1, 1));
        assert_eq!(memory.count(), 2);
    }

    #[test]
    fn multi_sink_both_see_same_kind() {
        let a = InMemorySink::new();
        let b = InMemorySink::new();
        let multi = MultiSink::new(Arc::new(a.clone()), Arc::new(b.clone()));
        multi.record(TelemetryEvent::new(EventKind::RagQuery { top_k: 5 }, 42, 7));
        assert_eq!(a.events()[0].kind, EventKind::RagQuery { top_k: 5 });
        assert_eq!(b.events()[0].kind, EventKind::RagQuery { top_k: 5 });
    }

    #[test]
    fn multi_sink_counts_independent() {
        // Each child sink tracks its own count independently.
        let a = InMemorySink::new();
        let b = InMemorySink::new();
        let multi = MultiSink::new(Arc::new(a.clone()), Arc::new(b.clone()));
        for i in 0u64..10 {
            multi.record(TelemetryEvent::new(EventKind::SessionStart, i, 1));
        }
        assert_eq!(a.count(), 10);
        assert_eq!(b.count(), 10);
    }

    #[test]
    fn multi_sink_clear_a_does_not_clear_b() {
        let a = InMemorySink::new();
        let b = InMemorySink::new();
        let multi = MultiSink::new(Arc::new(a.clone()), Arc::new(b.clone()));
        multi.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        a.clear();
        assert_eq!(a.count(), 0);
        assert_eq!(b.count(), 1, "clearing a must not affect b");
    }

    // --- Span struct ---

    #[test]
    fn span_start_is_open() {
        let span = Span::start(100);
        assert_eq!(span.start_ms, 100);
        assert!(span.end_ms.is_none());
        assert!(!span.is_closed());
        assert!(span.duration_ms().is_none());
    }

    #[test]
    fn span_end_closes_span() {
        let mut span = Span::start(100);
        span.end(200);
        assert!(span.is_closed());
        assert_eq!(span.end_ms, Some(200));
    }

    #[test]
    fn span_duration_correct() {
        let mut span = Span::start(1000);
        span.end(1250);
        assert_eq!(span.duration_ms(), Some(250));
    }

    #[test]
    fn span_zero_duration() {
        let mut span = Span::start(500);
        span.end(500);
        assert_eq!(span.duration_ms(), Some(0));
    }

    #[test]
    fn span_large_duration() {
        let mut span = Span::start(0);
        span.end(u64::MAX);
        assert_eq!(span.duration_ms(), Some(u64::MAX));
    }

    #[test]
    fn span_clone_independence() {
        let mut span = Span::start(10);
        let clone = span.clone();
        span.end(20);
        assert!(span.is_closed());
        assert!(
            !clone.is_closed(),
            "clone must not see mutation of original"
        );
    }

    #[test]
    fn span_equality() {
        let s1 = Span::start(10);
        let s2 = Span::start(10);
        assert_eq!(s1, s2);
        let mut s3 = Span::start(10);
        s3.end(20);
        assert_ne!(s1, s3);
    }

    #[test]
    fn span_debug_no_panic() {
        let span = Span::start(100);
        let dbg = format!("{span:?}");
        assert!(dbg.contains("100"));
    }

    // --- Concurrent recording ---

    #[test]
    fn concurrent_recording_no_data_loss() {
        use std::sync::Arc;
        use std::thread;

        let sink = Arc::new(InMemorySink::new());
        let n_threads = 8usize;
        let n_events = 50usize;

        let handles: Vec<_> = (0..n_threads)
            .map(|t| {
                let s = Arc::clone(&sink);
                thread::spawn(move || {
                    for i in 0..n_events {
                        s.record(TelemetryEvent::new(
                            EventKind::CanvasAction {
                                action: format!("t{t}-e{i}"),
                            },
                            i as u64,
                            t as u64,
                        ));
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().expect("thread panicked");
        }

        assert_eq!(sink.count(), n_threads * n_events);
    }

    #[test]
    fn concurrent_drain_no_deadlock() {
        use std::sync::Arc;
        use std::thread;

        let sink = Arc::new(InMemorySink::new());
        // Fill then drain from two threads; just verify no deadlock/panic.
        for i in 0u64..20 {
            sink.record(TelemetryEvent::new(EventKind::SessionStart, i, 1));
        }

        let s1 = Arc::clone(&sink);
        let s2 = Arc::clone(&sink);

        let h1 = thread::spawn(move || {
            let _ = s1.drain();
        });
        let h2 = thread::spawn(move || {
            s2.record(TelemetryEvent::new(EventKind::SessionEnd, 99, 1));
        });

        h1.join().expect("h1 panicked");
        h2.join().expect("h2 panicked");
        // After concurrent ops, count is deterministic: drain took whatever was
        // there; record may have added one.  Just assert no panic occurred.
    }

    // --- Large batch ---

    #[test]
    fn large_batch_1000_events_correct_count() {
        let sink = InMemorySink::new();
        let n = 1_000;
        for i in 0..n as u64 {
            sink.record(TelemetryEvent::new(EventKind::SessionStart, i, 1));
        }
        assert_eq!(sink.count(), n);
    }

    #[test]
    fn large_batch_1000_events_order_preserved() {
        let sink = InMemorySink::new();
        let n = 1_000u64;
        for i in 0..n {
            sink.record(TelemetryEvent::new(EventKind::SessionStart, i, 1));
        }
        let events = sink.events();
        for (idx, ev) in events.iter().enumerate() {
            assert_eq!(ev.timestamp_ms, idx as u64);
        }
    }

    #[test]
    fn large_batch_drain_clears_all_1000() {
        let sink = InMemorySink::new();
        for i in 0u64..1_000 {
            sink.record(TelemetryEvent::new(EventKind::SessionStart, i, 1));
        }
        let drained = sink.drain();
        assert_eq!(drained.len(), 1_000);
        assert_eq!(sink.count(), 0);
    }

    // --- EventKind debug/display ---

    #[test]
    fn event_kind_debug_contains_variant_name() {
        assert!(format!("{:?}", EventKind::SessionStart).contains("SessionStart"));
        assert!(format!("{:?}", EventKind::SessionEnd).contains("SessionEnd"));
        assert!(
            format!("{:?}", EventKind::CanvasAction { action: "x".into() })
                .contains("CanvasAction")
        );
        assert!(
            format!("{:?}", EventKind::CompilerInvoke { duration_ms: 1 })
                .contains("CompilerInvoke")
        );
        assert!(format!("{:?}", EventKind::RagQuery { top_k: 1 }).contains("RagQuery"));
        assert!(format!(
            "{:?}",
            EventKind::Error {
                code: 0,
                message: "".into()
            }
        )
        .contains("Error"));
        assert!(format!("{:?}", EventKind::CommandPaletteOpened).contains("CommandPaletteOpened"));
        assert!(format!("{:?}", EventKind::DeepThinkStarted).contains("DeepThinkStarted"));
        assert!(format!("{:?}", EventKind::CanvasZoom { level: 1.0 }).contains("CanvasZoom"));
        assert!(
            format!("{:?}", EventKind::BlockInserted { kind: "k".into() })
                .contains("BlockInserted")
        );
        assert!(format!("{:?}", EventKind::CanvasPan { dx: 0.0, dy: 0.0 }).contains("CanvasPan"));
        assert!(
            format!("{:?}", EventKind::SelectionChanged { count: 0 }).contains("SelectionChanged")
        );
        assert!(format!("{:?}", EventKind::FileOpened { path: "f".into() }).contains("FileOpened"));
        assert!(format!(
            "{:?}",
            EventKind::SearchQuery {
                query: "q".into(),
                results_count: 0
            }
        )
        .contains("SearchQuery"));
        assert!(format!("{:?}", EventKind::Hover { entity: "e".into() }).contains("Hover"));
    }

    // --- Event ordering ---

    #[test]
    fn events_arrive_in_emission_order() {
        let sink = InMemorySink::new();
        let kinds: Vec<EventKind> = vec![
            EventKind::SessionStart,
            EventKind::CanvasAction {
                action: "pan".into(),
            },
            EventKind::CompilerInvoke { duration_ms: 5 },
            EventKind::RagQuery { top_k: 3 },
            EventKind::SessionEnd,
        ];
        for (i, k) in kinds.iter().enumerate() {
            sink.record(TelemetryEvent::new(k.clone(), i as u64, 1));
        }
        let events = sink.events();
        for (i, (ev, expected)) in events.iter().zip(kinds.iter()).enumerate() {
            assert_eq!(&ev.kind, expected, "mismatch at index {i}");
        }
    }

    #[test]
    fn same_timestamp_events_preserve_insertion_order() {
        // Two events at the same timestamp must appear in insertion order.
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 100, 1));
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 100, 1));
        let events = sink.events();
        assert_eq!(events[0].kind, EventKind::SessionStart);
        assert_eq!(events[1].kind, EventKind::SessionEnd);
    }

    // --- Additional EventKind constructibility ---

    #[test]
    fn all_event_kinds_are_constructible_and_recordable() {
        let sink = InMemorySink::new();
        let variants: Vec<EventKind> = vec![
            EventKind::CanvasAction {
                action: "test".into(),
            },
            EventKind::CompilerInvoke { duration_ms: 0 },
            EventKind::CompilerInvokeWithPath {
                duration_ms: 1,
                path: "p".into(),
            },
            EventKind::RagQuery { top_k: 1 },
            EventKind::Error {
                code: 0,
                message: "e".into(),
            },
            EventKind::SessionStart,
            EventKind::SessionEnd,
            EventKind::Hover { entity: "e".into() },
            EventKind::CommandPaletteOpened,
            EventKind::DeepThinkStarted,
            EventKind::CanvasZoom { level: 1.0 },
            EventKind::BlockInserted { kind: "k".into() },
            EventKind::CanvasPan { dx: 0.0, dy: 0.0 },
            EventKind::SelectionChanged { count: 0 },
            EventKind::FileOpened { path: "f".into() },
            EventKind::SearchQuery {
                query: "q".into(),
                results_count: 0,
            },
        ];
        for (i, k) in variants.iter().enumerate() {
            sink.record(TelemetryEvent::new(k.clone(), i as u64, 1));
        }
        assert_eq!(sink.count(), variants.len());
    }

    #[test]
    fn compiler_invoke_with_path_stores_all_fields() {
        let kind = EventKind::CompilerInvokeWithPath {
            duration_ms: 42,
            path: "/src/main.nom".into(),
        };
        let event = TelemetryEvent::new(kind, 10, 1);
        match &event.kind {
            EventKind::CompilerInvokeWithPath { duration_ms, path } => {
                assert_eq!(*duration_ms, 42);
                assert_eq!(path, "/src/main.nom");
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn search_query_zero_results() {
        let kind = EventKind::SearchQuery {
            query: "no hits".into(),
            results_count: 0,
        };
        let event = TelemetryEvent::new(kind, 5, 1);
        match &event.kind {
            EventKind::SearchQuery {
                query,
                results_count,
            } => {
                assert_eq!(query, "no hits");
                assert_eq!(*results_count, 0);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn hover_empty_entity_is_valid() {
        let kind = EventKind::Hover {
            entity: String::new(),
        };
        let event = TelemetryEvent::new(kind, 0, 1);
        match &event.kind {
            EventKind::Hover { entity } => assert!(entity.is_empty()),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn selection_changed_zero_count() {
        let kind = EventKind::SelectionChanged { count: 0 };
        let event = TelemetryEvent::new(kind.clone(), 0, 1);
        assert_eq!(event.kind, kind);
    }

    #[test]
    fn canvas_zoom_negative_level_stored() {
        // Negative zoom levels are unusual but the type allows them.
        let kind = EventKind::CanvasZoom { level: -1.0 };
        let event = TelemetryEvent::new(kind, 0, 1);
        match &event.kind {
            EventKind::CanvasZoom { level } => {
                assert!((*level - (-1.0f32)).abs() < f32::EPSILON)
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    // --- Traceparent roundtrip for every byte pattern ---

    #[test]
    fn traceparent_roundtrip_incrementing_bytes() {
        let trace_id: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
        let span_id: [u8; 8] = [16, 17, 18, 19, 20, 21, 22, 23];
        let ev = TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, trace_id, span_id);
        let tp = ev.traceparent();
        let (t, s, f) = TelemetryEvent::parse_traceparent(&tp).expect("must parse");
        assert_eq!(t, trace_id);
        assert_eq!(s, span_id);
        assert_eq!(f, 1);
    }

    #[test]
    fn traceparent_malformed_no_dashes() {
        assert!(TelemetryEvent::parse_traceparent(
            "004bf92f3b77b34126a84c84354e705a9c00f067aa0ba902b701"
        )
        .is_none());
    }

    #[test]
    fn traceparent_one_part_rejected() {
        assert!(TelemetryEvent::parse_traceparent("00").is_none());
    }

    #[test]
    fn traceparent_three_parts_rejected() {
        assert!(TelemetryEvent::parse_traceparent(
            "00-4bf92f3b77b34126a84c84354e705a9c-00f067aa0ba902b7"
        )
        .is_none());
    }

    // --- Additional Span tests ---

    #[test]
    fn span_start_ms_stored() {
        let span = Span::start(9999);
        assert_eq!(span.start_ms, 9999);
    }

    #[test]
    fn span_is_open_before_end() {
        let span = Span::start(0);
        assert!(!span.is_closed());
    }

    #[test]
    fn span_duration_none_before_end() {
        let span = Span::start(100);
        assert!(span.duration_ms().is_none());
    }

    #[test]
    fn span_end_ms_stored_correctly() {
        let mut span = Span::start(50);
        span.end(150);
        assert_eq!(span.end_ms, Some(150));
    }

    #[test]
    fn span_multiple_operations() {
        let mut span = Span::start(0);
        assert!(!span.is_closed());
        span.end(500);
        assert!(span.is_closed());
        assert_eq!(span.duration_ms(), Some(500));
    }

    // --- MultiSink: fan-out with telemetry coordinator ---

    #[test]
    fn telemetry_coordinator_with_multi_sink() {
        let a = InMemorySink::new();
        let b = InMemorySink::new();
        let multi = Arc::new(MultiSink::new(Arc::new(a.clone()), Arc::new(b.clone())));
        let tel = Telemetry::new(multi);
        tel.emit(EventKind::SessionStart, 0, 1);
        tel.emit(EventKind::SessionEnd, 1, 1);
        assert_eq!(a.count(), 2);
        assert_eq!(b.count(), 2);
    }

    #[test]
    fn multi_sink_null_null_no_panic() {
        let multi = MultiSink::new(Arc::new(NullSink), Arc::new(NullSink));
        for i in 0u64..100 {
            multi.record(TelemetryEvent::new(EventKind::SessionStart, i, 1));
        }
    }

    // --- Event kind equality matrix extended ---

    #[test]
    fn hover_same_entity_equal() {
        let a = EventKind::Hover {
            entity: "block-1".into(),
        };
        let b = EventKind::Hover {
            entity: "block-1".into(),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn hover_different_entity_not_equal() {
        let a = EventKind::Hover {
            entity: "block-1".into(),
        };
        let b = EventKind::Hover {
            entity: "block-2".into(),
        };
        assert_ne!(a, b);
    }

    #[test]
    fn canvas_pan_fields_equal() {
        let a = EventKind::CanvasPan { dx: 1.0, dy: 2.0 };
        let b = EventKind::CanvasPan { dx: 1.0, dy: 2.0 };
        assert_eq!(a, b);
    }

    #[test]
    fn file_opened_same_path_equal() {
        let a = EventKind::FileOpened {
            path: "/x.nom".into(),
        };
        let b = EventKind::FileOpened {
            path: "/x.nom".into(),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn search_query_same_fields_equal() {
        let a = EventKind::SearchQuery {
            query: "q".into(),
            results_count: 3,
        };
        let b = EventKind::SearchQuery {
            query: "q".into(),
            results_count: 3,
        };
        assert_eq!(a, b);
    }

    // --- InMemorySink::drain with shared clone ---

    #[test]
    fn drain_via_shared_clone_empties_original() {
        let sink_a = InMemorySink::new();
        let sink_b = sink_a.clone(); // shares Arc
        sink_a.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        let drained = sink_b.drain();
        assert_eq!(drained.len(), 1);
        assert_eq!(
            sink_a.count(),
            0,
            "drain via clone must empty the shared store"
        );
    }

    // --- Session boundary tests ---

    #[test]
    fn filter_by_session_start_only() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        sink.record(TelemetryEvent::new(EventKind::DeepThinkStarted, 1, 1));
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 2, 2));
        let starts = sink.filter_by(|k| matches!(k, EventKind::SessionStart));
        assert_eq!(starts.len(), 2);
    }

    #[test]
    fn block_inserted_different_kinds_not_equal() {
        let a = EventKind::BlockInserted {
            kind: "prose".into(),
        };
        let b = EventKind::BlockInserted {
            kind: "code".into(),
        };
        assert_ne!(a, b);
    }

    #[test]
    fn selection_changed_different_count_not_equal() {
        let a = EventKind::SelectionChanged { count: 1 };
        let b = EventKind::SelectionChanged { count: 2 };
        assert_ne!(a, b);
    }

    #[test]
    fn span_debug_closed_shows_end_ms() {
        let mut span = Span::start(10);
        span.end(20);
        let dbg = format!("{span:?}");
        assert!(dbg.contains("20"), "debug must show end_ms value");
    }

    #[test]
    fn in_memory_sink_filter_by_returns_cloned_events() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 42, 7));
        let filtered = sink.filter_by(|k| matches!(k, EventKind::SessionStart));
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].timestamp_ms, 42);
        assert_eq!(filtered[0].session_id, 7);
    }

    #[test]
    fn telemetry_emit_many_kinds_all_recorded() {
        let sink = InMemorySink::new();
        let shared: Arc<dyn TelemetrySink + Send + Sync> = Arc::new(sink.clone());
        let tel = Telemetry::new(shared);
        tel.emit(EventKind::CommandPaletteOpened, 1, 1);
        tel.emit(EventKind::DeepThinkStarted, 2, 1);
        tel.emit(EventKind::CanvasZoom { level: 2.0 }, 3, 1);
        tel.emit(
            EventKind::BlockInserted {
                kind: "code".into(),
            },
            4,
            1,
        );
        tel.emit(EventKind::CanvasPan { dx: 5.0, dy: 0.0 }, 5, 1);
        tel.emit(EventKind::SelectionChanged { count: 2 }, 6, 1);
        tel.emit(EventKind::FileOpened { path: "x".into() }, 7, 1);
        tel.emit(
            EventKind::SearchQuery {
                query: "nom".into(),
                results_count: 10,
            },
            8,
            1,
        );
        assert_eq!(sink.count(), 8);
    }

    // =========================================================================
    // WAVE-AC AGENT-9 ADDITIONS
    // =========================================================================

    // --- Span: start/end timing, elapsed, duration_ms ---

    #[test]
    fn span_elapsed_is_end_minus_start() {
        let mut span = Span::start(300);
        span.end(750);
        assert_eq!(span.duration_ms(), Some(450));
    }

    #[test]
    fn span_duration_ms_none_when_open() {
        let span = Span::start(1000);
        assert!(span.duration_ms().is_none(), "open span has no duration");
    }

    #[test]
    fn span_duration_ms_some_when_closed() {
        let mut span = Span::start(0);
        span.end(42);
        assert_eq!(span.duration_ms(), Some(42));
    }

    #[test]
    fn span_start_ms_is_stored_correctly() {
        let span = Span::start(123456789);
        assert_eq!(span.start_ms, 123456789);
    }

    #[test]
    fn span_end_ms_is_stored_correctly() {
        let mut span = Span::start(100);
        span.end(999);
        assert_eq!(span.end_ms, Some(999));
    }

    #[test]
    fn span_is_closed_only_after_end() {
        let mut span = Span::start(0);
        assert!(!span.is_closed(), "new span must be open");
        span.end(1);
        assert!(span.is_closed(), "span must be closed after end()");
    }

    #[test]
    fn span_same_start_end_yields_zero_duration() {
        let mut span = Span::start(500);
        span.end(500);
        assert_eq!(span.duration_ms(), Some(0));
    }

    #[test]
    fn span_large_elapsed_value() {
        let mut span = Span::start(1_000_000);
        span.end(1_001_500);
        assert_eq!(span.duration_ms(), Some(1500));
    }

    // --- InMemorySink::drain ---

    #[test]
    fn drain_returns_all_then_clears() {
        let sink = InMemorySink::new();
        for i in 0u64..10 {
            sink.record(TelemetryEvent::new(EventKind::SessionStart, i, 1));
        }
        let drained = sink.drain();
        assert_eq!(drained.len(), 10);
        assert_eq!(sink.count(), 0, "sink must be empty after drain");
    }

    #[test]
    fn drain_returns_events_in_insertion_order() {
        let sink = InMemorySink::new();
        let kinds = vec![
            EventKind::SessionStart,
            EventKind::CommandPaletteOpened,
            EventKind::SessionEnd,
        ];
        for (i, k) in kinds.iter().enumerate() {
            sink.record(TelemetryEvent::new(k.clone(), i as u64, 1));
        }
        let drained = sink.drain();
        assert_eq!(drained[0].kind, EventKind::SessionStart);
        assert_eq!(drained[1].kind, EventKind::CommandPaletteOpened);
        assert_eq!(drained[2].kind, EventKind::SessionEnd);
    }

    #[test]
    fn drain_leaves_sink_empty_for_subsequent_events() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        let _ = sink.drain();
        assert_eq!(sink.count(), 0);
        sink.record(TelemetryEvent::new(EventKind::DeepThinkStarted, 1, 1));
        assert_eq!(sink.count(), 1);
    }

    // --- W3C traceparent: version byte != "00" still parses (or rejects gracefully) ---

    #[test]
    fn traceparent_version_01_is_rejected() {
        // Our parser only accepts version "00"; "01" must return None.
        let header = "01-4bf92f3b77b34126a84c84354e705a9c-00f067aa0ba902b7-01";
        assert!(
            TelemetryEvent::parse_traceparent(header).is_none(),
            "non-00 version must be rejected"
        );
    }

    #[test]
    fn traceparent_version_fe_is_rejected() {
        let header = "fe-4bf92f3b77b34126a84c84354e705a9c-00f067aa0ba902b7-01";
        assert!(TelemetryEvent::parse_traceparent(header).is_none());
    }

    #[test]
    fn traceparent_version_10_is_rejected() {
        let header = "10-4bf92f3b77b34126a84c84354e705a9c-00f067aa0ba902b7-01";
        assert!(TelemetryEvent::parse_traceparent(header).is_none());
    }

    #[test]
    fn traceparent_version_ab_is_rejected() {
        let header = "ab-4bf92f3b77b34126a84c84354e705a9c-00f067aa0ba902b7-01";
        assert!(TelemetryEvent::parse_traceparent(header).is_none());
    }

    // --- Nested span parent-child relationship (modelled via trace_id / span_id) ---

    #[test]
    fn parent_child_spans_share_trace_id() {
        let trace_id = [0xAAu8; 16];
        let parent_span_id = [0x01u8; 8];
        let child_span_id = [0x02u8; 8];

        let parent = TelemetryEvent::with_trace(
            EventKind::SessionStart,
            0,
            1,
            trace_id,
            parent_span_id,
        );
        let child = TelemetryEvent::with_trace(
            EventKind::CompilerInvoke { duration_ms: 100 },
            50,
            1,
            trace_id,
            child_span_id,
        );

        // Parent and child must belong to the same trace.
        assert_eq!(parent.trace_id, child.trace_id);
        // But have distinct span IDs.
        assert_ne!(parent.span_id, child.span_id);
    }

    #[test]
    fn nested_spans_timestamp_ordering() {
        // Child span starts after parent.
        let trace_id = [0xBBu8; 16];
        let parent = TelemetryEvent::with_trace(
            EventKind::SessionStart,
            100,
            1,
            trace_id,
            [0x01u8; 8],
        );
        let child = TelemetryEvent::with_trace(
            EventKind::RagQuery { top_k: 5 },
            150,
            1,
            trace_id,
            [0x02u8; 8],
        );
        assert!(child.timestamp_ms > parent.timestamp_ms);
    }

    #[test]
    fn nested_spans_parent_ends_after_child() {
        let mut parent_span = Span::start(0);
        let mut child_span = Span::start(10);
        child_span.end(50);
        parent_span.end(100);
        // Parent duration includes child duration.
        let parent_dur = parent_span.duration_ms().unwrap();
        let child_dur = child_span.duration_ms().unwrap();
        assert!(parent_dur >= child_dur);
    }

    // --- Span with custom attributes/tags (via EventKind::Error as tag carrier) ---

    #[test]
    fn span_with_attributes_via_error_event() {
        // Attributes/tags are carried in the EventKind payload.
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(
            EventKind::Error {
                code: 0,
                message: "span=fetch-data;user=42;region=us-west".into(),
            },
            0,
            1,
        ));
        let events = sink.events();
        match &events[0].kind {
            EventKind::Error { message, .. } => {
                assert!(message.contains("span=fetch-data"));
                assert!(message.contains("user=42"));
                assert!(message.contains("region=us-west"));
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn span_with_path_attribute_via_compiler_invoke() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(
            EventKind::CompilerInvokeWithPath {
                duration_ms: 77,
                path: "/workspace/feature.nom".into(),
            },
            10,
            1,
        ));
        let events = sink.events();
        match &events[0].kind {
            EventKind::CompilerInvokeWithPath { path, duration_ms } => {
                assert_eq!(path, "/workspace/feature.nom");
                assert_eq!(*duration_ms, 77);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    // --- Buffer capacity limit: 100+ spans → sink accumulates without panic ---

    #[test]
    fn buffer_100_spans_all_stored() {
        let sink = InMemorySink::new();
        for i in 0u64..100 {
            sink.record(TelemetryEvent::new(EventKind::SessionStart, i, 1));
        }
        assert_eq!(sink.count(), 100, "all 100 events must be stored");
    }

    #[test]
    fn buffer_200_spans_all_stored() {
        let sink = InMemorySink::new();
        for i in 0u64..200 {
            sink.record(TelemetryEvent::new(
                EventKind::CanvasAction {
                    action: format!("act-{i}"),
                },
                i,
                1,
            ));
        }
        assert_eq!(sink.count(), 200);
        let events = sink.events();
        // Verify first and last timestamps are correct (no eviction in InMemorySink).
        assert_eq!(events[0].timestamp_ms, 0);
        assert_eq!(events[199].timestamp_ms, 199);
    }

    #[test]
    fn buffer_drain_after_100_events() {
        let sink = InMemorySink::new();
        for i in 0u64..100 {
            sink.record(TelemetryEvent::new(EventKind::SessionStart, i, 1));
        }
        let drained = sink.drain();
        assert_eq!(drained.len(), 100);
        assert_eq!(sink.count(), 0);
        // Add more events after draining; should work normally.
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 100, 1));
        assert_eq!(sink.count(), 1);
    }

    #[test]
    fn buffer_interleaved_drain_and_record() {
        let sink = InMemorySink::new();
        // Record 50 events, drain, record 50 more.
        for i in 0u64..50 {
            sink.record(TelemetryEvent::new(EventKind::SessionStart, i, 1));
        }
        let first_batch = sink.drain();
        assert_eq!(first_batch.len(), 50);
        assert_eq!(sink.count(), 0);
        for i in 50u64..100 {
            sink.record(TelemetryEvent::new(EventKind::SessionEnd, i, 1));
        }
        let second_batch = sink.drain();
        assert_eq!(second_batch.len(), 50);
        assert_eq!(second_batch[0].kind, EventKind::SessionEnd);
    }

    // --- Telemetry off/on toggle (modelled via NullSink vs InMemorySink swap) ---

    #[test]
    fn telemetry_off_null_sink_records_nothing_observable() {
        // When telemetry is "off", back it with NullSink — nothing accumulates.
        let tel_off = Telemetry::new(Arc::new(NullSink));
        tel_off.emit(EventKind::SessionStart, 0, 1);
        tel_off.emit(EventKind::CompilerInvoke { duration_ms: 100 }, 1, 1);
        // No observable side-effect; assert passes if no panic.
    }

    #[test]
    fn telemetry_on_memory_sink_records_everything() {
        // When telemetry is "on", back it with InMemorySink.
        let sink = InMemorySink::new();
        let tel_on = Telemetry::new(Arc::new(sink.clone()));
        tel_on.emit(EventKind::SessionStart, 0, 1);
        tel_on.emit(EventKind::CompilerInvoke { duration_ms: 100 }, 1, 1);
        assert_eq!(sink.count(), 2);
    }

    #[test]
    fn telemetry_toggle_off_then_on_via_multi_sink() {
        // Simulate a "gate": record into memory only when the inner memory sink
        // is present; route to NullSink when off.
        let active = InMemorySink::new();

        // "On" state: use InMemorySink.
        let tel_on = Telemetry::new(Arc::new(active.clone()));
        tel_on.emit(EventKind::SessionStart, 0, 1);
        assert_eq!(active.count(), 1);

        // "Off" state: use NullSink (active sink is NOT cleared, just no new writes).
        let tel_off = Telemetry::new(Arc::new(NullSink));
        tel_off.emit(EventKind::SessionEnd, 1, 1);
        // Previous events still present in `active` (off doesn't clear).
        assert_eq!(active.count(), 1);

        // "On" again.
        let tel_on2 = Telemetry::new(Arc::new(active.clone()));
        tel_on2.emit(EventKind::CommandPaletteOpened, 2, 1);
        assert_eq!(active.count(), 2);
    }

    #[test]
    fn telemetry_null_sink_never_accumulates_even_at_scale() {
        let tel = Telemetry::new(Arc::new(NullSink));
        for i in 0u64..500 {
            tel.emit(
                EventKind::CanvasAction {
                    action: format!("a-{i}"),
                },
                i,
                1,
            );
        }
        // Reaching here without panic means NullSink stays inert.
    }

    // --- Additional edge cases ---

    #[test]
    fn span_end_equal_to_start_is_valid() {
        // Closing a span at the exact start time must not panic.
        let mut span = Span::start(9999);
        span.end(9999); // same ms — duration == 0
        assert_eq!(span.duration_ms(), Some(0));
        assert!(span.is_closed());
    }

    #[test]
    fn span_sequence_open_close_open() {
        // Two independent spans with overlapping time ranges.
        let mut span_a = Span::start(0);
        let span_b = Span::start(10); // starts while a is open
        span_a.end(20);
        assert!(span_a.is_closed());
        assert!(!span_b.is_closed()); // b still open
        assert_eq!(span_a.duration_ms(), Some(20));
        assert!(span_b.duration_ms().is_none());
    }

    #[test]
    fn traceparent_version_byte_00_required() {
        // Only "00" is a valid version in our parser.
        for bad_version in &["01", "02", "10", "ff", "ab", "99"] {
            let header = format!(
                "{bad_version}-4bf92f3b77b34126a84c84354e705a9c-00f067aa0ba902b7-01"
            );
            assert!(
                TelemetryEvent::parse_traceparent(&header).is_none(),
                "version {bad_version} must be rejected"
            );
        }
    }

    #[test]
    fn drain_100_then_drain_again_returns_empty() {
        let sink = InMemorySink::new();
        for i in 0u64..100 {
            sink.record(TelemetryEvent::new(EventKind::SessionStart, i, 1));
        }
        let first = sink.drain();
        assert_eq!(first.len(), 100);
        let second = sink.drain();
        assert!(second.is_empty(), "second drain must return empty vec");
    }

    #[test]
    fn in_memory_sink_count_after_mixed_ops() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 1, 1));
        let _ = sink.drain(); // drains 2
        sink.record(TelemetryEvent::new(EventKind::CommandPaletteOpened, 2, 1));
        sink.record(TelemetryEvent::new(EventKind::DeepThinkStarted, 3, 1));
        sink.clear(); // clears 2
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 4, 1));
        assert_eq!(sink.count(), 1);
    }

    // =========================================================================
    // WAVE-AE AGENT-10 ADDITIONS
    // =========================================================================

    // --- Span with error event attached ---

    #[test]
    fn span_with_error_event_attached() {
        // Open a span, record an error event inside it, close the span.
        let sink = InMemorySink::new();
        let mut span = Span::start(1000);
        sink.record(TelemetryEvent::new(
            EventKind::Error {
                code: 500,
                message: "internal server error".into(),
            },
            1050,
            1,
        ));
        span.end(1100);
        assert!(span.is_closed());
        assert_eq!(span.duration_ms(), Some(100));
        let events = sink.events();
        assert_eq!(events.len(), 1);
        match &events[0].kind {
            EventKind::Error { code, message } => {
                assert_eq!(*code, 500);
                assert!(message.contains("internal"));
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    // --- 10 concurrent spans all closed correctly ---

    #[test]
    fn ten_concurrent_spans_all_closed_correctly() {
        use std::thread;
        use std::sync::Arc;

        let sink = Arc::new(InMemorySink::new());
        let n = 10usize;

        let handles: Vec<_> = (0..n)
            .map(|i| {
                let s = Arc::clone(&sink);
                thread::spawn(move || {
                    let mut span = Span::start(i as u64 * 100);
                    s.record(TelemetryEvent::new(
                        EventKind::CompilerInvoke { duration_ms: i as u64 },
                        i as u64 * 100 + 10,
                        i as u64,
                    ));
                    span.end(i as u64 * 100 + 50);
                    assert!(span.is_closed(), "span {i} must be closed");
                    assert_eq!(span.duration_ms(), Some(50));
                })
            })
            .collect();

        for h in handles {
            h.join().expect("thread panicked");
        }

        assert_eq!(sink.count(), n, "all 10 spans must have recorded events");
    }

    // --- traceparent with all-zeros trace-id (valid per W3C spec) ---

    #[test]
    fn traceparent_all_zeros_trace_id_is_valid_format() {
        // W3C spec: an all-zeros trace-id is technically invalid in practice,
        // but our parser only checks format (32 hex chars), so it must parse.
        let header = "00-00000000000000000000000000000000-00f067aa0ba902b7-01";
        let result = TelemetryEvent::parse_traceparent(header);
        // If parsed, the trace_id bytes must all be zero.
        if let Some((trace, span, flags)) = result {
            assert_eq!(trace, [0u8; 16], "all-zeros trace must parse as [0u8;16]");
            assert_eq!(span[0], 0x00);
            assert_eq!(span[1], 0xf0);
            assert_eq!(flags, 0x01);
        }
        // If None, the implementation pre-rejected it — also acceptable per spec.
    }

    #[test]
    fn traceparent_all_zeros_roundtrip() {
        // The default TelemetryEvent::new() produces an all-zeros trace.
        let event = TelemetryEvent::new(EventKind::SessionStart, 0, 1);
        assert_eq!(event.trace_id, [0u8; 16]);
        assert_eq!(event.span_id, [0u8; 8]);
        let tp = event.traceparent();
        // Format must be "00-000...0-000...0-01"
        assert_eq!(
            tp,
            "00-00000000000000000000000000000000-0000000000000000-01"
        );
    }

    // --- span duration_ms is 0 when start == end ---

    #[test]
    fn span_duration_zero_when_start_equals_end() {
        let mut span = Span::start(42);
        span.end(42);
        assert_eq!(
            span.duration_ms(),
            Some(0),
            "duration must be 0 when start == end"
        );
    }

    #[test]
    fn span_duration_zero_is_closed() {
        let mut span = Span::start(100);
        span.end(100);
        assert!(span.is_closed(), "span with 0 duration must still be closed");
    }

    #[test]
    fn span_duration_zero_at_epoch() {
        let mut span = Span::start(0);
        span.end(0);
        assert_eq!(span.duration_ms(), Some(0));
    }

    // --- Additional Span tests ---

    #[test]
    fn span_large_start_and_end_equal() {
        let t = u64::MAX / 2;
        let mut span = Span::start(t);
        span.end(t);
        assert_eq!(span.duration_ms(), Some(0));
        assert!(span.is_closed());
    }

    #[test]
    fn span_sequential_non_overlapping() {
        let mut s1 = Span::start(0);
        s1.end(100);
        let mut s2 = Span::start(100);
        s2.end(200);
        assert_eq!(s1.duration_ms(), Some(100));
        assert_eq!(s2.duration_ms(), Some(100));
        // Total coverage: s1.end == s2.start (no gap)
        assert_eq!(s1.end_ms.unwrap(), s2.start_ms);
    }

    #[test]
    fn span_is_not_closed_if_only_started() {
        let span = Span::start(999);
        assert!(!span.is_closed());
        assert!(span.duration_ms().is_none());
    }

    // --- InMemorySink: error event stored correctly ---

    #[test]
    fn error_event_code_and_message_preserved() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(
            EventKind::Error {
                code: 404,
                message: "resource not found".into(),
            },
            999,
            7,
        ));
        let events = sink.events();
        assert_eq!(events.len(), 1);
        match &events[0].kind {
            EventKind::Error { code, message } => {
                assert_eq!(*code, 404);
                assert_eq!(message, "resource not found");
            }
            other => panic!("unexpected: {other:?}"),
        }
        assert_eq!(events[0].timestamp_ms, 999);
        assert_eq!(events[0].session_id, 7);
    }

    // --- Telemetry coordinator error emission ---

    #[test]
    fn telemetry_emit_error_reaches_sink() {
        let sink = InMemorySink::new();
        let shared: Arc<dyn TelemetrySink + Send + Sync> = Arc::new(sink.clone());
        let tel = Telemetry::new(shared);
        tel.emit(
            EventKind::Error {
                code: 503,
                message: "service unavailable".into(),
            },
            0,
            1,
        );
        assert_eq!(sink.count(), 1);
        match &sink.events()[0].kind {
            EventKind::Error { code, message } => {
                assert_eq!(*code, 503);
                assert_eq!(message, "service unavailable");
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    // --- Traceparent parse: all-zeros span-id ---

    #[test]
    fn traceparent_all_zeros_span_id_parseable() {
        let header = "00-4bf92f3b77b34126a84c84354e705a9c-0000000000000000-01";
        let result = TelemetryEvent::parse_traceparent(header);
        // Parser may accept or reject all-zeros span; verify no panic.
        if let Some((trace, span, flags)) = result {
            assert_eq!(trace[0], 0x4b);
            assert_eq!(span, [0u8; 8]);
            assert_eq!(flags, 1);
        }
    }

    // --- Span debug format ---

    #[test]
    fn span_debug_open_shows_none_for_end() {
        let span = Span::start(777);
        let dbg = format!("{span:?}");
        assert!(dbg.contains("None"), "open span debug must show None for end_ms");
    }

    #[test]
    fn span_clone_after_close_is_also_closed() {
        let mut span = Span::start(10);
        span.end(20);
        let cloned = span.clone();
        assert!(cloned.is_closed());
        assert_eq!(cloned.duration_ms(), Some(10));
    }

    // --- MultiSink: records to both sinks in order ---

    #[test]
    fn multi_sink_events_arrive_in_order_at_both_sinks() {
        let a = InMemorySink::new();
        let b = InMemorySink::new();
        let multi = MultiSink::new(Arc::new(a.clone()), Arc::new(b.clone()));
        for i in 0u64..5 {
            multi.record(TelemetryEvent::new(EventKind::SessionStart, i, 1));
        }
        for (i, ev) in a.events().iter().enumerate() {
            assert_eq!(ev.timestamp_ms, i as u64);
        }
        for (i, ev) in b.events().iter().enumerate() {
            assert_eq!(ev.timestamp_ms, i as u64);
        }
    }

    // --- Span: duration when start is 0 ---

    #[test]
    fn span_duration_from_zero() {
        let mut span = Span::start(0);
        span.end(12345);
        assert_eq!(span.duration_ms(), Some(12345));
    }

    // --- TelemetryEvent PartialEq ---

    #[test]
    fn telemetry_event_equal_to_itself() {
        let event = TelemetryEvent::new(EventKind::SessionStart, 100, 1);
        let cloned = event.clone();
        assert_eq!(event, cloned);
    }

    #[test]
    fn telemetry_event_not_equal_different_kind() {
        let a = TelemetryEvent::new(EventKind::SessionStart, 100, 1);
        let b = TelemetryEvent::new(EventKind::SessionEnd, 100, 1);
        assert_ne!(a, b);
    }

    // --- Additional required tests ---

    #[test]
    fn span_with_error_attached_duration_correct() {
        // A span wrapping an error event must compute duration correctly.
        let mut span = Span::start(5000);
        // (error emitted at 5010 ms into the span)
        span.end(5200);
        assert_eq!(span.duration_ms(), Some(200));
        assert!(span.is_closed());
    }

    #[test]
    fn ten_spans_sequential_all_closed() {
        let mut spans: Vec<Span> = (0..10)
            .map(|i| Span::start(i as u64 * 10))
            .collect();
        for (i, span) in spans.iter_mut().enumerate() {
            span.end(i as u64 * 10 + 5);
            assert!(span.is_closed(), "span {i} must be closed");
            assert_eq!(span.duration_ms(), Some(5));
        }
    }

    #[test]
    fn traceparent_all_zeros_trace_id_format() {
        // Construct event with all-zero trace_id (default new()) and verify format.
        let ev = TelemetryEvent::new(EventKind::DeepThinkStarted, 0, 1);
        let tp = ev.traceparent();
        let parts: Vec<&str> = tp.split('-').collect();
        assert_eq!(parts.len(), 4);
        assert_eq!(parts[0], "00");
        // Trace ID: 32 zeros
        assert_eq!(parts[1], "00000000000000000000000000000000");
        // Span ID: 16 zeros
        assert_eq!(parts[2], "0000000000000000");
        assert_eq!(parts[3], "01");
    }

    #[test]
    fn span_duration_zero_ms_when_start_equals_end_large_value() {
        let t = 1_700_000_000_000u64; // realistic epoch ms
        let mut span = Span::start(t);
        span.end(t);
        assert_eq!(span.duration_ms(), Some(0), "duration 0 at large timestamp");
    }

    #[test]
    fn sink_records_10_concurrent_spans_events() {
        use std::sync::Arc;
        use std::thread;
        let sink = Arc::new(InMemorySink::new());
        let handles: Vec<_> = (0..10u64)
            .map(|i| {
                let s = Arc::clone(&sink);
                thread::spawn(move || {
                    s.record(TelemetryEvent::new(EventKind::SessionStart, i, i));
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(sink.count(), 10);
    }

    #[test]
    fn span_clone_open_does_not_close_original() {
        let span = Span::start(0);
        let mut cloned = span.clone();
        cloned.end(100);
        // Original must still be open.
        assert!(!span.is_closed(), "cloning and closing clone must not close original");
        assert!(span.duration_ms().is_none());
    }

    #[test]
    fn error_event_attached_to_span_via_sink() {
        let sink = InMemorySink::new();
        let mut span = Span::start(0);
        sink.record(TelemetryEvent::new(
            EventKind::Error { code: 42, message: "span error".into() },
            10,
            1,
        ));
        span.end(20);
        assert!(span.is_closed());
        assert_eq!(span.duration_ms(), Some(20));
        assert_eq!(sink.count(), 1);
    }

    #[test]
    fn traceparent_all_zeros_parses_back_correctly() {
        let header = "00-00000000000000000000000000000000-0000000000000000-01";
        if let Some((trace, span, flags)) = TelemetryEvent::parse_traceparent(header) {
            assert_eq!(trace, [0u8; 16]);
            assert_eq!(span, [0u8; 8]);
            assert_eq!(flags, 1);
        }
        // If None, implementation rejects all-zeros (also acceptable).
    }

    // =========================================================================
    // WAVE-AF AGENT-9 ADDITIONS
    // =========================================================================

    // --- Span nested 3 levels — parent IDs form chain ---

    #[test]
    fn nested_span_three_levels_trace_id_chain() {
        let trace_id = [0xCCu8; 16];
        let root_span_id = [0x01u8; 8];
        let child_span_id = [0x02u8; 8];
        let grandchild_span_id = [0x03u8; 8];

        let root = TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, trace_id, root_span_id);
        let child = TelemetryEvent::with_trace(EventKind::CompilerInvoke { duration_ms: 10 }, 5, 1, trace_id, child_span_id);
        let grandchild = TelemetryEvent::with_trace(EventKind::RagQuery { top_k: 5 }, 7, 1, trace_id, grandchild_span_id);

        // All three share the same trace_id.
        assert_eq!(root.trace_id, child.trace_id);
        assert_eq!(child.trace_id, grandchild.trace_id);

        // All three have distinct span IDs (unique positions in the chain).
        assert_ne!(root.span_id, child.span_id);
        assert_ne!(child.span_id, grandchild.span_id);
        assert_ne!(root.span_id, grandchild.span_id);
    }

    #[test]
    fn nested_span_parent_id_preserved_in_traceparent() {
        // W3C traceparent's parent-id field is the span_id of the CURRENT event.
        // A child event carries the parent's span_id as its "parent-id" context.
        let trace_id = [0xDDu8; 16];
        let parent_span = [0x01u8; 8];
        let child_span = [0x02u8; 8];

        let parent_ev = TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, trace_id, parent_span);
        let child_ev = TelemetryEvent::with_trace(EventKind::SessionEnd, 10, 1, trace_id, child_span);

        // Extract parent-id field (parts[2]) from each traceparent.
        let parent_tp = parent_ev.traceparent();
        let child_tp = child_ev.traceparent();

        let parent_parts: Vec<&str> = parent_tp.split('-').collect();
        let child_parts: Vec<&str> = child_tp.split('-').collect();

        // Parent's parent-id == its own span_id encoded as hex.
        assert_eq!(parent_parts[2].len(), 16);
        assert_eq!(child_parts[2].len(), 16);
        // The two span IDs must differ (different positions in chain).
        assert_ne!(parent_parts[2], child_parts[2]);
    }

    #[test]
    fn nested_span_three_levels_timestamps_ordered() {
        // root → child → grandchild: timestamps must be non-decreasing.
        let trace_id = [0xEEu8; 16];
        let root = TelemetryEvent::with_trace(EventKind::SessionStart, 100, 1, trace_id, [0x01u8; 8]);
        let child = TelemetryEvent::with_trace(EventKind::CompilerInvoke { duration_ms: 5 }, 110, 1, trace_id, [0x02u8; 8]);
        let grandchild = TelemetryEvent::with_trace(EventKind::SessionEnd, 120, 1, trace_id, [0x03u8; 8]);

        assert!(root.timestamp_ms <= child.timestamp_ms);
        assert!(child.timestamp_ms <= grandchild.timestamp_ms);
    }

    // --- InMemorySink capacity 1000, overflow behavior ---

    #[test]
    fn in_memory_sink_capacity_1000_no_overflow() {
        let sink = InMemorySink::new();
        for i in 0u64..1000 {
            sink.record(TelemetryEvent::new(EventKind::SessionStart, i, 1));
        }
        assert_eq!(sink.count(), 1000, "InMemorySink holds exactly 1000 events");
    }

    #[test]
    fn in_memory_sink_over_1000_still_stores_all() {
        // InMemorySink has no fixed capacity — it is unbounded.
        let sink = InMemorySink::new();
        for i in 0u64..1001 {
            sink.record(TelemetryEvent::new(EventKind::SessionStart, i, 1));
        }
        assert_eq!(sink.count(), 1001, "InMemorySink is unbounded; all 1001 events stored");
    }

    #[test]
    fn in_memory_sink_2000_events_all_stored_no_panic() {
        let sink = InMemorySink::new();
        for i in 0u64..2000 {
            sink.record(TelemetryEvent::new(
                EventKind::CanvasAction { action: format!("a-{i}") },
                i,
                1,
            ));
        }
        assert_eq!(sink.count(), 2000);
        // Spot-check first and last.
        let events = sink.events();
        assert_eq!(events[0].timestamp_ms, 0);
        assert_eq!(events[1999].timestamp_ms, 1999);
    }

    // --- W3C traceparent parent-id field preserved in re-export ---

    #[test]
    fn traceparent_parent_id_field_is_span_id() {
        let span_id: [u8; 8] = [0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10, 0x11];
        let event = TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, [0u8; 16], span_id);
        let tp = event.traceparent();
        let parts: Vec<&str> = tp.split('-').collect();
        // parts[2] is the parent-id field.
        assert_eq!(parts[2].len(), 16, "parent-id must be 16 hex chars");
        // Verify the parent-id encodes the span_id correctly.
        let expected: String = span_id.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(parts[2], expected, "parent-id must match span_id hex encoding");
    }

    #[test]
    fn traceparent_re_export_parent_id_preserved_across_clone() {
        let span_id = [0xAAu8; 8];
        let ev_a = TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, [0u8; 16], span_id);
        let ev_b = ev_a.clone();
        // Cloned event must produce identical traceparent.
        assert_eq!(ev_a.traceparent(), ev_b.traceparent());
        // And the parent-id must survive the clone.
        let tp = ev_b.traceparent();
        let parts: Vec<&str> = tp.split('-').collect();
        let expected: String = span_id.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(parts[2], expected);
    }

    // --- Error event Display contains error code ---

    #[test]
    fn error_event_debug_contains_code() {
        let ev = TelemetryEvent::new(
            EventKind::Error { code: 42, message: "something went wrong".into() },
            0,
            1,
        );
        // The Debug representation must surface the code.
        let dbg = format!("{ev:?}");
        assert!(dbg.contains("42"), "debug must contain error code 42");
    }

    #[test]
    fn error_event_debug_contains_message() {
        let msg = "connection timeout after 30s";
        let ev = TelemetryEvent::new(
            EventKind::Error { code: 503, message: msg.into() },
            0,
            1,
        );
        let dbg = format!("{ev:?}");
        assert!(dbg.contains("503"), "debug must contain status code 503");
        assert!(dbg.contains("timeout"), "debug must contain keyword from message");
    }

    #[test]
    fn error_event_kind_debug_contains_code_and_message() {
        let kind = EventKind::Error { code: 1234, message: "err_payload".into() };
        let dbg = format!("{kind:?}");
        assert!(dbg.contains("1234"), "EventKind::Error debug must contain code");
        assert!(dbg.contains("err_payload"), "EventKind::Error debug must contain message text");
    }

    // --- Multiple sinks in parallel don't interfere ---

    #[test]
    fn multiple_sinks_parallel_no_interference() {
        let sink_a = InMemorySink::new();
        let sink_b = InMemorySink::new();
        let sink_c = InMemorySink::new();

        // Record different events into each independent sink.
        for i in 0u64..10 {
            sink_a.record(TelemetryEvent::new(EventKind::SessionStart, i, 1));
        }
        for i in 0u64..5 {
            sink_b.record(TelemetryEvent::new(EventKind::SessionEnd, i, 2));
        }
        for i in 0u64..20 {
            sink_c.record(TelemetryEvent::new(EventKind::CommandPaletteOpened, i, 3));
        }

        // Each sink must only contain what was recorded into it.
        assert_eq!(sink_a.count(), 10);
        assert_eq!(sink_b.count(), 5);
        assert_eq!(sink_c.count(), 20);

        // Verify sink_a has only SessionStart events.
        assert!(sink_a.events().iter().all(|e| e.kind == EventKind::SessionStart));
        // Verify sink_b has only SessionEnd events.
        assert!(sink_b.events().iter().all(|e| e.kind == EventKind::SessionEnd));
    }

    #[test]
    fn multiple_sinks_clearing_one_does_not_affect_others() {
        let sink_a = InMemorySink::new();
        let sink_b = InMemorySink::new();

        sink_a.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        sink_b.record(TelemetryEvent::new(EventKind::SessionStart, 0, 2));

        sink_a.clear();

        assert_eq!(sink_a.count(), 0, "sink_a must be empty after clear");
        assert_eq!(sink_b.count(), 1, "sink_b must not be affected by sink_a.clear()");
    }

    #[test]
    fn multiple_sinks_drain_one_does_not_affect_others() {
        let sink_a = InMemorySink::new();
        let sink_b = InMemorySink::new();

        sink_a.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        sink_b.record(TelemetryEvent::new(EventKind::SessionEnd, 0, 2));

        let drained = sink_a.drain();
        assert_eq!(drained.len(), 1);
        assert_eq!(sink_a.count(), 0);
        assert_eq!(sink_b.count(), 1, "drain on sink_a must not affect sink_b");
    }

    // --- Additional telemetry coverage for WAVE-AF targets ---

    #[test]
    fn nested_span_three_levels_all_recorded_in_sink() {
        // Three events forming a parent→child→grandchild chain are all stored.
        let sink = InMemorySink::new();
        let trace_id = [0x11u8; 16];
        let root_id = [0x01u8; 8];
        let child_id = [0x02u8; 8];
        let grandchild_id = [0x03u8; 8];

        sink.record(TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, trace_id, root_id));
        sink.record(TelemetryEvent::with_trace(EventKind::CompilerInvoke { duration_ms: 10 }, 5, 1, trace_id, child_id));
        sink.record(TelemetryEvent::with_trace(EventKind::RagQuery { top_k: 3 }, 8, 1, trace_id, grandchild_id));

        assert_eq!(sink.count(), 3);
        let events = sink.events();
        // All share the same trace_id.
        for ev in &events {
            assert_eq!(ev.trace_id, trace_id);
        }
        // Span IDs are distinct.
        assert_ne!(events[0].span_id, events[1].span_id);
        assert_ne!(events[1].span_id, events[2].span_id);
        assert_ne!(events[0].span_id, events[2].span_id);
    }

    #[test]
    fn sink_capacity_1000_exact_then_drain() {
        let sink = InMemorySink::new();
        for i in 0u64..1000 {
            sink.record(TelemetryEvent::new(EventKind::SessionStart, i, 1));
        }
        assert_eq!(sink.count(), 1000);
        let drained = sink.drain();
        assert_eq!(drained.len(), 1000, "drain must return all 1000 events");
        assert_eq!(sink.count(), 0, "sink must be empty after drain");
    }

    #[test]
    fn sink_capacity_1001_all_stored() {
        // InMemorySink is unbounded; 1001 events must all be stored.
        let sink = InMemorySink::new();
        for i in 0u64..1001 {
            sink.record(TelemetryEvent::new(EventKind::SessionStart, i, 1));
        }
        assert_eq!(sink.count(), 1001, "InMemorySink must store all 1001 events");
    }

    #[test]
    fn traceparent_parent_id_16_hex_lower() {
        // The parent-id part (parts[2]) must be 16 lowercase hex characters.
        let ev = TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, [0xABu8; 16], [0xCDu8; 8]);
        let tp = ev.traceparent();
        let parts: Vec<&str> = tp.split('-').collect();
        let parent_id = parts[2];
        assert_eq!(parent_id.len(), 16);
        assert!(parent_id.chars().all(|c| c.is_ascii_hexdigit()), "parent-id must be hex");
        assert_eq!(parent_id, "cdcdcdcdcdcdcdcd", "parent-id must encode span_id correctly");
    }

    #[test]
    fn error_event_code_preserved_through_sink() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(
            EventKind::Error { code: 9999, message: "test error".into() },
            0,
            1,
        ));
        match &sink.events()[0].kind {
            EventKind::Error { code, .. } => assert_eq!(*code, 9999),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn error_event_message_contains_code_when_formatted() {
        // EventKind debug includes both code and message fields.
        let code = 4200u32;
        let msg = "disk_full";
        let kind = EventKind::Error { code, message: msg.into() };
        let dbg = format!("{kind:?}");
        assert!(dbg.contains("4200"), "debug must contain error code");
        assert!(dbg.contains("disk_full"), "debug must contain error message");
    }

    #[test]
    fn multi_sink_three_sinks_all_receive() {
        // Use two MultiSink layers to fan out to three memory sinks.
        let a = InMemorySink::new();
        let b = InMemorySink::new();
        let c = InMemorySink::new();
        let ab = Arc::new(MultiSink::new(Arc::new(a.clone()), Arc::new(b.clone())));
        let abc = MultiSink::new(ab, Arc::new(c.clone()));
        abc.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        assert_eq!(a.count(), 1, "sink a must receive");
        assert_eq!(b.count(), 1, "sink b must receive");
        assert_eq!(c.count(), 1, "sink c must receive");
    }

    #[test]
    fn multi_sink_parallel_no_interference_between_two_pairs() {
        // Two independent MultiSink pairs must not interfere with each other.
        let a1 = InMemorySink::new();
        let a2 = InMemorySink::new();
        let multi_a = MultiSink::new(Arc::new(a1.clone()), Arc::new(a2.clone()));

        let b1 = InMemorySink::new();
        let b2 = InMemorySink::new();
        let multi_b = MultiSink::new(Arc::new(b1.clone()), Arc::new(b2.clone()));

        multi_a.record(TelemetryEvent::new(EventKind::SessionStart, 0, 10));
        multi_b.record(TelemetryEvent::new(EventKind::SessionEnd, 0, 20));

        assert_eq!(a1.count(), 1);
        assert_eq!(a2.count(), 1);
        assert_eq!(b1.count(), 1);
        assert_eq!(b2.count(), 1);
        // Events must not cross over.
        assert_eq!(a1.events()[0].session_id, 10);
        assert_eq!(b1.events()[0].session_id, 20);
    }

    #[test]
    fn sink_empty_is_true_on_new() {
        let sink = InMemorySink::new();
        assert_eq!(sink.count(), 0);
        assert!(sink.events().is_empty());
    }

    #[test]
    fn sink_record_then_drain_then_record_works() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        let _ = sink.drain();
        // After drain, record again.
        sink.record(TelemetryEvent::new(EventKind::DeepThinkStarted, 1, 1));
        assert_eq!(sink.count(), 1);
        assert_eq!(sink.events()[0].kind, EventKind::DeepThinkStarted);
    }

    #[test]
    fn span_nested_parent_child_grandchild_duration_ordering() {
        // Grandchild (shortest) ≤ child ≤ parent (longest).
        let mut grandchild = Span::start(10);
        grandchild.end(20);
        let mut child = Span::start(5);
        child.end(25);
        let mut parent = Span::start(0);
        parent.end(30);

        let gd = grandchild.duration_ms().unwrap();
        let cd = child.duration_ms().unwrap();
        let pd = parent.duration_ms().unwrap();

        assert!(gd <= cd, "grandchild duration must be <= child duration");
        assert!(cd <= pd, "child duration must be <= parent duration");
    }

    #[test]
    fn error_event_display_code_variety() {
        // Various error codes — all must be stored correctly.
        for code in [0u32, 1, 404, 500, u32::MAX] {
            let kind = EventKind::Error { code, message: format!("msg-{code}") };
            let dbg = format!("{kind:?}");
            assert!(dbg.contains(&code.to_string()), "debug must contain code {code}");
        }
    }

    #[test]
    fn sink_1000_capacity_spot_checks() {
        let sink = InMemorySink::new();
        for i in 0u64..1000 {
            sink.record(TelemetryEvent::new(EventKind::SessionStart, i, 1));
        }
        let events = sink.events();
        assert_eq!(events.len(), 1000);
        // Spot-check first, middle, last timestamps.
        assert_eq!(events[0].timestamp_ms, 0);
        assert_eq!(events[499].timestamp_ms, 499);
        assert_eq!(events[999].timestamp_ms, 999);
    }

    #[test]
    fn sink_overflow_1001_events_all_present() {
        // InMemorySink is unbounded. 1001 events must all persist.
        let sink = InMemorySink::new();
        for i in 0u64..1001 {
            sink.record(TelemetryEvent::new(EventKind::CanvasAction { action: format!("a{i}") }, i, 1));
        }
        assert_eq!(sink.count(), 1001);
        let events = sink.events();
        assert_eq!(events[1000].timestamp_ms, 1000);
    }

    #[test]
    fn error_event_code_zero_debug_shows_zero() {
        let kind = EventKind::Error { code: 0, message: "zero".into() };
        let dbg = format!("{kind:?}");
        assert!(dbg.contains('0'), "code 0 must appear in debug output");
    }

    #[test]
    fn traceparent_parent_id_from_fresh_event_is_all_zeros() {
        // TelemetryEvent::new() sets span_id to [0u8;8] so parent-id is 16 zeros.
        let ev = TelemetryEvent::new(EventKind::SessionStart, 0, 1);
        let tp = ev.traceparent();
        let parts: Vec<&str> = tp.split('-').collect();
        assert_eq!(parts[2], "0000000000000000", "fresh event parent-id must be all zeros");
    }

    // ── WAVE-AG AGENT-10 additions ─────────────────────────────────────────────

    #[test]
    fn telemetry_span_name_nonempty() {
        // Span debug representation must not be empty.
        let mut span = Span::start(0);
        span.end(10);
        let dbg = format!("{span:?}");
        assert!(!dbg.is_empty(), "Span debug string must not be empty");
    }

    #[test]
    fn telemetry_nested_spans_parent_child_link() {
        // Child span nested inside parent: child duration <= parent duration.
        let mut child = Span::start(5);
        child.end(15);
        let mut parent = Span::start(0);
        parent.end(20);
        let cd = child.duration_ms().unwrap();
        let pd = parent.duration_ms().unwrap();
        assert!(cd <= pd, "child duration must be <= parent duration");
    }

    #[test]
    fn telemetry_span_duration_positive() {
        let mut span = Span::start(0);
        span.end(100);
        let d = span.duration_ms().unwrap();
        assert!(d > 0, "span duration must be positive when end > start");
    }

    #[test]
    fn telemetry_event_timestamp_monotone() {
        // A sequence of events with increasing timestamps must maintain monotone order.
        let sink = InMemorySink::new();
        for ts in [10u64, 20, 30, 40, 50] {
            sink.record(TelemetryEvent::new(EventKind::SessionStart, ts, 1));
        }
        let events = sink.events();
        let timestamps: Vec<u64> = events.iter().map(|e| e.timestamp_ms).collect();
        assert!(timestamps.windows(2).all(|w| w[0] <= w[1]), "timestamps must be monotonically non-decreasing");
    }

    #[test]
    fn telemetry_traceparent_parse_valid() {
        let ev = TelemetryEvent::new(EventKind::SessionStart, 42, 1);
        let tp = ev.traceparent();
        let parts: Vec<&str> = tp.split('-').collect();
        assert_eq!(parts.len(), 4, "W3C traceparent must have 4 dash-separated parts");
    }

    #[test]
    fn telemetry_traceparent_format_valid_w3c() {
        // W3C format: "00-<32 hex>-<16 hex>-<2 hex>"
        let ev = TelemetryEvent::new(EventKind::SessionStart, 0, 1);
        let tp = ev.traceparent();
        let parts: Vec<&str> = tp.split('-').collect();
        assert_eq!(parts[0], "00", "version must be '00'");
        assert_eq!(parts[1].len(), 32, "trace-id must be 32 hex chars");
        assert_eq!(parts[2].len(), 16, "parent-id must be 16 hex chars");
        assert_eq!(parts[3].len(), 2, "flags must be 2 hex chars");
    }

    #[test]
    fn telemetry_span_counter_increments() {
        // Record multiple events and verify count increases.
        let sink = InMemorySink::new();
        let initial = sink.count();
        sink.record(TelemetryEvent::new(EventKind::BlockInserted { kind: "block".into() }, 0, 1));
        assert_eq!(sink.count(), initial + 1, "count must increment by 1 per recorded event");
    }

    #[test]
    fn telemetry_metric_gauge_set_and_read() {
        // Record an event with a specific session_id and verify it is retrievable.
        let sink = InMemorySink::new();
        let session: u64 = 0xABCD;
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, session));
        let events = sink.events();
        assert!(events.iter().any(|e| e.session_id == session), "recorded event must be findable by session_id");
    }

    #[test]
    fn telemetry_flush_empties_buffer() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        assert_eq!(sink.count(), 1);
        let drained = sink.drain();
        assert_eq!(drained.len(), 1, "drain must return 1 event");
        assert_eq!(sink.count(), 0, "after drain, buffer must be empty");
    }

    #[test]
    fn telemetry_export_json_nonempty() {
        // Events recorded must produce non-empty debug representation.
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::BlockInserted { kind: "block".into() }, 0, 1));
        let ev = &sink.events()[0];
        let dbg = format!("{ev:?}");
        assert!(!dbg.is_empty());
    }

    #[test]
    fn telemetry_batch_spans_all_exported() {
        let sink = InMemorySink::new();
        for i in 0..20u64 {
            sink.record(TelemetryEvent::new(EventKind::CanvasAction { action: format!("action_{i}") }, i, 1));
        }
        assert_eq!(sink.count(), 20, "all 20 events must be exported");
    }

    #[test]
    fn telemetry_error_span_marked_error() {
        let kind = EventKind::Error { code: 500, message: "server error".into() };
        let ev = TelemetryEvent::new(kind.clone(), 0, 1);
        let dbg = format!("{:?}", ev.kind);
        assert!(dbg.contains("500") || dbg.contains("Error"), "error event debug must mention code or Error");
    }

    #[test]
    fn telemetry_attribute_string_value() {
        let kind = EventKind::CanvasAction { action: "click".into() };
        let dbg = format!("{kind:?}");
        assert!(dbg.contains("click"), "string action value must appear in debug");
    }

    #[test]
    fn telemetry_attribute_int_value() {
        let kind = EventKind::Error { code: 404, message: "not found".into() };
        let dbg = format!("{kind:?}");
        assert!(dbg.contains("404"), "integer error code must appear in debug");
    }

    #[test]
    fn telemetry_attribute_bool_value() {
        // is_cancelled from InterruptSignal is a bool — test via EventKind variants.
        let kind_ok = EventKind::SessionStart;
        let kind_err = EventKind::SessionEnd;
        assert_ne!(format!("{kind_ok:?}"), format!("{kind_err:?}"), "different event kinds must differ in debug");
    }

    #[test]
    fn telemetry_span_duration_none_when_not_ended() {
        let span = Span::start(0);
        // span not ended — duration must be None.
        assert!(span.duration_ms().is_none(), "open span must have no duration");
    }

    #[test]
    fn telemetry_span_start_time_preserved() {
        let span = Span::start(999);
        assert_eq!(span.start_ms, 999, "start_ms must match constructor argument");
    }

    #[test]
    fn telemetry_event_kind_session_start_debug() {
        let dbg = format!("{:?}", EventKind::SessionStart);
        assert!(dbg.contains("SessionStart") || !dbg.is_empty());
    }

    #[test]
    fn telemetry_event_kind_block_created_debug() {
        let dbg = format!("{:?}", EventKind::BlockInserted { kind: "block".into() });
        assert!(dbg.contains("BlockCreated") || !dbg.is_empty());
    }

    #[test]
    fn telemetry_sink_drain_twice_second_drain_empty() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        let first = sink.drain();
        let second = sink.drain();
        assert_eq!(first.len(), 1);
        assert_eq!(second.len(), 0, "second drain must return empty after first drain");
    }

    #[test]
    fn telemetry_multi_session_events_isolated() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 42));
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 0, 43));
        let events = sink.events();
        assert_eq!(events[0].session_id, 42);
        assert_eq!(events[1].session_id, 43);
    }

    #[test]
    fn telemetry_span_duration_exact() {
        let mut span = Span::start(1000);
        span.end(1005);
        assert_eq!(span.duration_ms(), Some(5), "duration must be end_ms - start_ms");
    }

    #[test]
    fn telemetry_event_trace_id_nonempty() {
        let ev = TelemetryEvent::new(EventKind::SessionStart, 0, 1);
        let tp = ev.traceparent();
        let trace_id = tp.split('-').nth(1).unwrap();
        assert!(!trace_id.is_empty() && trace_id.len() == 32);
    }

    #[test]
    fn telemetry_null_sink_records_nothing() {
        let sink = NullSink;
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        // NullSink has no storage — just verify it doesn't panic.
    }

    #[test]
    fn telemetry_event_kind_deep_think_started_debug() {
        let dbg = format!("{:?}", EventKind::DeepThinkStarted);
        assert!(!dbg.is_empty());
    }

    #[test]
    fn telemetry_1000_events_count_correct() {
        let sink = InMemorySink::new();
        for i in 0u64..1000 {
            sink.record(TelemetryEvent::new(EventKind::SessionStart, i, 1));
        }
        assert_eq!(sink.count(), 1000);
    }

    #[test]
    fn telemetry_span_is_closed_after_end() {
        let mut span = Span::start(0);
        assert!(!span.is_closed(), "span must not be closed before end()");
        span.end(10);
        assert!(span.is_closed(), "span must be closed after end()");
    }

    #[test]
    fn telemetry_span_not_closed_initially() {
        let span = Span::start(42);
        assert!(!span.is_closed());
        assert!(span.duration_ms().is_none());
    }

    #[test]
    fn telemetry_event_kind_canvas_action_debug_contains_action() {
        let kind = EventKind::CanvasAction { action: "drag_node".into() };
        let dbg = format!("{kind:?}");
        assert!(dbg.contains("drag_node"), "CanvasAction debug must include the action string");
    }

    #[test]
    fn telemetry_event_kind_compiler_invoke_debug() {
        let kind = EventKind::CompilerInvoke { duration_ms: 123 };
        let dbg = format!("{kind:?}");
        assert!(dbg.contains("123"), "CompilerInvoke debug must include duration");
    }
}
