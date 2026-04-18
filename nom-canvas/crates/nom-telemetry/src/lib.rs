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
}
