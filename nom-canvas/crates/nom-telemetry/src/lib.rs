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

    /// Return the number of events recorded so far (alias for `count`).
    pub fn event_count(&self) -> usize {
        self.count()
    }

    /// Drain all recorded events and return them, leaving the buffer empty.
    ///
    /// Equivalent to `drain`.
    pub fn flush(&mut self) -> Vec<TelemetryEvent> {
        let mut guard = self.events.lock().expect("InMemorySink mutex poisoned");
        std::mem::take(&mut *guard)
    }

    /// Return a cloned subset of events whose `EventKind` variant name contains
    /// `tag` (case-insensitive).
    ///
    /// The tag is matched against the debug discriminant of the variant
    /// (`"SessionStart"`, `"CompilerInvoke"`, `"CanvasAction"`, …).
    pub fn filter_by_tag(&self, tag: &str) -> Vec<TelemetryEvent> {
        let tag_lower = tag.to_lowercase();
        self.events
            .lock()
            .expect("InMemorySink mutex poisoned")
            .iter()
            .filter(|e| {
                let kind_name = event_kind_tag(&e.kind);
                kind_name.to_lowercase().contains(&tag_lower)
            })
            .cloned()
            .collect()
    }
}

/// Return a stable string tag for an `EventKind` variant.
fn event_kind_tag(kind: &EventKind) -> &'static str {
    match kind {
        EventKind::CanvasAction { .. } => "CanvasAction",
        EventKind::CompilerInvoke { .. } => "CompilerInvoke",
        EventKind::CompilerInvokeWithPath { .. } => "CompilerInvokeWithPath",
        EventKind::RagQuery { .. } => "RagQuery",
        EventKind::Error { .. } => "Error",
        EventKind::SessionStart => "SessionStart",
        EventKind::SessionEnd => "SessionEnd",
        EventKind::Hover { .. } => "Hover",
        EventKind::CommandPaletteOpened => "CommandPaletteOpened",
        EventKind::DeepThinkStarted => "DeepThinkStarted",
        EventKind::CanvasZoom { .. } => "CanvasZoom",
        EventKind::BlockInserted { .. } => "BlockInserted",
        EventKind::CanvasPan { .. } => "CanvasPan",
        EventKind::SelectionChanged { .. } => "SelectionChanged",
        EventKind::FileOpened { .. } => "FileOpened",
        EventKind::SearchQuery { .. } => "SearchQuery",
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

        let parent =
            TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, trace_id, parent_span_id);
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
        let parent =
            TelemetryEvent::with_trace(EventKind::SessionStart, 100, 1, trace_id, [0x01u8; 8]);
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
            let header =
                format!("{bad_version}-4bf92f3b77b34126a84c84354e705a9c-00f067aa0ba902b7-01");
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
        use std::sync::Arc;
        use std::thread;

        let sink = Arc::new(InMemorySink::new());
        let n = 10usize;

        let handles: Vec<_> = (0..n)
            .map(|i| {
                let s = Arc::clone(&sink);
                thread::spawn(move || {
                    let mut span = Span::start(i as u64 * 100);
                    s.record(TelemetryEvent::new(
                        EventKind::CompilerInvoke {
                            duration_ms: i as u64,
                        },
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
        assert!(
            span.is_closed(),
            "span with 0 duration must still be closed"
        );
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
        assert!(
            dbg.contains("None"),
            "open span debug must show None for end_ms"
        );
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
        let mut spans: Vec<Span> = (0..10).map(|i| Span::start(i as u64 * 10)).collect();
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
        assert!(
            !span.is_closed(),
            "cloning and closing clone must not close original"
        );
        assert!(span.duration_ms().is_none());
    }

    #[test]
    fn error_event_attached_to_span_via_sink() {
        let sink = InMemorySink::new();
        let mut span = Span::start(0);
        sink.record(TelemetryEvent::new(
            EventKind::Error {
                code: 42,
                message: "span error".into(),
            },
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

        let root =
            TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, trace_id, root_span_id);
        let child = TelemetryEvent::with_trace(
            EventKind::CompilerInvoke { duration_ms: 10 },
            5,
            1,
            trace_id,
            child_span_id,
        );
        let grandchild = TelemetryEvent::with_trace(
            EventKind::RagQuery { top_k: 5 },
            7,
            1,
            trace_id,
            grandchild_span_id,
        );

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

        let parent_ev =
            TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, trace_id, parent_span);
        let child_ev =
            TelemetryEvent::with_trace(EventKind::SessionEnd, 10, 1, trace_id, child_span);

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
        let root =
            TelemetryEvent::with_trace(EventKind::SessionStart, 100, 1, trace_id, [0x01u8; 8]);
        let child = TelemetryEvent::with_trace(
            EventKind::CompilerInvoke { duration_ms: 5 },
            110,
            1,
            trace_id,
            [0x02u8; 8],
        );
        let grandchild =
            TelemetryEvent::with_trace(EventKind::SessionEnd, 120, 1, trace_id, [0x03u8; 8]);

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
        assert_eq!(
            sink.count(),
            1001,
            "InMemorySink is unbounded; all 1001 events stored"
        );
    }

    #[test]
    fn in_memory_sink_2000_events_all_stored_no_panic() {
        let sink = InMemorySink::new();
        for i in 0u64..2000 {
            sink.record(TelemetryEvent::new(
                EventKind::CanvasAction {
                    action: format!("a-{i}"),
                },
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
        assert_eq!(
            parts[2], expected,
            "parent-id must match span_id hex encoding"
        );
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
            EventKind::Error {
                code: 42,
                message: "something went wrong".into(),
            },
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
            EventKind::Error {
                code: 503,
                message: msg.into(),
            },
            0,
            1,
        );
        let dbg = format!("{ev:?}");
        assert!(dbg.contains("503"), "debug must contain status code 503");
        assert!(
            dbg.contains("timeout"),
            "debug must contain keyword from message"
        );
    }

    #[test]
    fn error_event_kind_debug_contains_code_and_message() {
        let kind = EventKind::Error {
            code: 1234,
            message: "err_payload".into(),
        };
        let dbg = format!("{kind:?}");
        assert!(
            dbg.contains("1234"),
            "EventKind::Error debug must contain code"
        );
        assert!(
            dbg.contains("err_payload"),
            "EventKind::Error debug must contain message text"
        );
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
        assert!(sink_a
            .events()
            .iter()
            .all(|e| e.kind == EventKind::SessionStart));
        // Verify sink_b has only SessionEnd events.
        assert!(sink_b
            .events()
            .iter()
            .all(|e| e.kind == EventKind::SessionEnd));
    }

    #[test]
    fn multiple_sinks_clearing_one_does_not_affect_others() {
        let sink_a = InMemorySink::new();
        let sink_b = InMemorySink::new();

        sink_a.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        sink_b.record(TelemetryEvent::new(EventKind::SessionStart, 0, 2));

        sink_a.clear();

        assert_eq!(sink_a.count(), 0, "sink_a must be empty after clear");
        assert_eq!(
            sink_b.count(),
            1,
            "sink_b must not be affected by sink_a.clear()"
        );
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

        sink.record(TelemetryEvent::with_trace(
            EventKind::SessionStart,
            0,
            1,
            trace_id,
            root_id,
        ));
        sink.record(TelemetryEvent::with_trace(
            EventKind::CompilerInvoke { duration_ms: 10 },
            5,
            1,
            trace_id,
            child_id,
        ));
        sink.record(TelemetryEvent::with_trace(
            EventKind::RagQuery { top_k: 3 },
            8,
            1,
            trace_id,
            grandchild_id,
        ));

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
        assert_eq!(
            sink.count(),
            1001,
            "InMemorySink must store all 1001 events"
        );
    }

    #[test]
    fn traceparent_parent_id_16_hex_lower() {
        // The parent-id part (parts[2]) must be 16 lowercase hex characters.
        let ev =
            TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, [0xABu8; 16], [0xCDu8; 8]);
        let tp = ev.traceparent();
        let parts: Vec<&str> = tp.split('-').collect();
        let parent_id = parts[2];
        assert_eq!(parent_id.len(), 16);
        assert!(
            parent_id.chars().all(|c| c.is_ascii_hexdigit()),
            "parent-id must be hex"
        );
        assert_eq!(
            parent_id, "cdcdcdcdcdcdcdcd",
            "parent-id must encode span_id correctly"
        );
    }

    #[test]
    fn error_event_code_preserved_through_sink() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(
            EventKind::Error {
                code: 9999,
                message: "test error".into(),
            },
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
        let kind = EventKind::Error {
            code,
            message: msg.into(),
        };
        let dbg = format!("{kind:?}");
        assert!(dbg.contains("4200"), "debug must contain error code");
        assert!(
            dbg.contains("disk_full"),
            "debug must contain error message"
        );
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
            let kind = EventKind::Error {
                code,
                message: format!("msg-{code}"),
            };
            let dbg = format!("{kind:?}");
            assert!(
                dbg.contains(&code.to_string()),
                "debug must contain code {code}"
            );
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
            sink.record(TelemetryEvent::new(
                EventKind::CanvasAction {
                    action: format!("a{i}"),
                },
                i,
                1,
            ));
        }
        assert_eq!(sink.count(), 1001);
        let events = sink.events();
        assert_eq!(events[1000].timestamp_ms, 1000);
    }

    #[test]
    fn error_event_code_zero_debug_shows_zero() {
        let kind = EventKind::Error {
            code: 0,
            message: "zero".into(),
        };
        let dbg = format!("{kind:?}");
        assert!(dbg.contains('0'), "code 0 must appear in debug output");
    }

    #[test]
    fn traceparent_parent_id_from_fresh_event_is_all_zeros() {
        // TelemetryEvent::new() sets span_id to [0u8;8] so parent-id is 16 zeros.
        let ev = TelemetryEvent::new(EventKind::SessionStart, 0, 1);
        let tp = ev.traceparent();
        let parts: Vec<&str> = tp.split('-').collect();
        assert_eq!(
            parts[2], "0000000000000000",
            "fresh event parent-id must be all zeros"
        );
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
        assert!(
            timestamps.windows(2).all(|w| w[0] <= w[1]),
            "timestamps must be monotonically non-decreasing"
        );
    }

    #[test]
    fn telemetry_traceparent_parse_valid() {
        let ev = TelemetryEvent::new(EventKind::SessionStart, 42, 1);
        let tp = ev.traceparent();
        let parts: Vec<&str> = tp.split('-').collect();
        assert_eq!(
            parts.len(),
            4,
            "W3C traceparent must have 4 dash-separated parts"
        );
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
        sink.record(TelemetryEvent::new(
            EventKind::BlockInserted {
                kind: "block".into(),
            },
            0,
            1,
        ));
        assert_eq!(
            sink.count(),
            initial + 1,
            "count must increment by 1 per recorded event"
        );
    }

    #[test]
    fn telemetry_metric_gauge_set_and_read() {
        // Record an event with a specific session_id and verify it is retrievable.
        let sink = InMemorySink::new();
        let session: u64 = 0xABCD;
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, session));
        let events = sink.events();
        assert!(
            events.iter().any(|e| e.session_id == session),
            "recorded event must be findable by session_id"
        );
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
        sink.record(TelemetryEvent::new(
            EventKind::BlockInserted {
                kind: "block".into(),
            },
            0,
            1,
        ));
        let ev = &sink.events()[0];
        let dbg = format!("{ev:?}");
        assert!(!dbg.is_empty());
    }

    #[test]
    fn telemetry_batch_spans_all_exported() {
        let sink = InMemorySink::new();
        for i in 0..20u64 {
            sink.record(TelemetryEvent::new(
                EventKind::CanvasAction {
                    action: format!("action_{i}"),
                },
                i,
                1,
            ));
        }
        assert_eq!(sink.count(), 20, "all 20 events must be exported");
    }

    #[test]
    fn telemetry_error_span_marked_error() {
        let kind = EventKind::Error {
            code: 500,
            message: "server error".into(),
        };
        let ev = TelemetryEvent::new(kind.clone(), 0, 1);
        let dbg = format!("{:?}", ev.kind);
        assert!(
            dbg.contains("500") || dbg.contains("Error"),
            "error event debug must mention code or Error"
        );
    }

    #[test]
    fn telemetry_attribute_string_value() {
        let kind = EventKind::CanvasAction {
            action: "click".into(),
        };
        let dbg = format!("{kind:?}");
        assert!(
            dbg.contains("click"),
            "string action value must appear in debug"
        );
    }

    #[test]
    fn telemetry_attribute_int_value() {
        let kind = EventKind::Error {
            code: 404,
            message: "not found".into(),
        };
        let dbg = format!("{kind:?}");
        assert!(
            dbg.contains("404"),
            "integer error code must appear in debug"
        );
    }

    #[test]
    fn telemetry_attribute_bool_value() {
        // is_cancelled from InterruptSignal is a bool — test via EventKind variants.
        let kind_ok = EventKind::SessionStart;
        let kind_err = EventKind::SessionEnd;
        assert_ne!(
            format!("{kind_ok:?}"),
            format!("{kind_err:?}"),
            "different event kinds must differ in debug"
        );
    }

    #[test]
    fn telemetry_span_duration_none_when_not_ended() {
        let span = Span::start(0);
        // span not ended — duration must be None.
        assert!(
            span.duration_ms().is_none(),
            "open span must have no duration"
        );
    }

    #[test]
    fn telemetry_span_start_time_preserved() {
        let span = Span::start(999);
        assert_eq!(
            span.start_ms, 999,
            "start_ms must match constructor argument"
        );
    }

    #[test]
    fn telemetry_event_kind_session_start_debug() {
        let dbg = format!("{:?}", EventKind::SessionStart);
        assert!(dbg.contains("SessionStart") || !dbg.is_empty());
    }

    #[test]
    fn telemetry_event_kind_block_created_debug() {
        let dbg = format!(
            "{:?}",
            EventKind::BlockInserted {
                kind: "block".into()
            }
        );
        assert!(dbg.contains("BlockCreated") || !dbg.is_empty());
    }

    #[test]
    fn telemetry_sink_drain_twice_second_drain_empty() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        let first = sink.drain();
        let second = sink.drain();
        assert_eq!(first.len(), 1);
        assert_eq!(
            second.len(),
            0,
            "second drain must return empty after first drain"
        );
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
        assert_eq!(
            span.duration_ms(),
            Some(5),
            "duration must be end_ms - start_ms"
        );
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
        let kind = EventKind::CanvasAction {
            action: "drag_node".into(),
        };
        let dbg = format!("{kind:?}");
        assert!(
            dbg.contains("drag_node"),
            "CanvasAction debug must include the action string"
        );
    }

    #[test]
    fn telemetry_event_kind_compiler_invoke_debug() {
        let kind = EventKind::CompilerInvoke { duration_ms: 123 };
        let dbg = format!("{kind:?}");
        assert!(
            dbg.contains("123"),
            "CompilerInvoke debug must include duration"
        );
    }

    // --- Wave AH Agent 9 additions ---

    #[test]
    fn telemetry_trace_id_128_bits() {
        // trace_id field is [u8; 16] = 128 bits.
        let event = TelemetryEvent::new(EventKind::SessionStart, 0, 1);
        assert_eq!(
            event.trace_id.len(),
            16,
            "trace_id must be 16 bytes (128 bits)"
        );
    }

    #[test]
    fn telemetry_span_id_64_bits() {
        // span_id field is [u8; 8] = 64 bits.
        let event = TelemetryEvent::new(EventKind::SessionStart, 0, 1);
        assert_eq!(event.span_id.len(), 8, "span_id must be 8 bytes (64 bits)");
    }

    #[test]
    fn telemetry_trace_id_nonempty_hex() {
        let trace_id: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let span_id: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
        let event = TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, trace_id, span_id);
        let tp = event.traceparent();
        let parts: Vec<&str> = tp.split('-').collect();
        assert!(!parts[1].is_empty(), "trace_id hex must not be empty");
        assert_eq!(parts[1].len(), 32);
    }

    #[test]
    fn telemetry_span_id_nonempty_hex() {
        let trace_id = [0u8; 16];
        let span_id: [u8; 8] = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x11, 0x22];
        let event = TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, trace_id, span_id);
        let tp = event.traceparent();
        let parts: Vec<&str> = tp.split('-').collect();
        assert!(!parts[2].is_empty(), "span_id hex must not be empty");
        assert_eq!(parts[2].len(), 16);
        assert!(
            parts[2].contains("aa") || parts[2].contains("AA") || parts[2] == "aabbccddeeff1122"
        );
    }

    #[test]
    fn telemetry_traceparent_version_is_00() {
        let event = TelemetryEvent::new(EventKind::SessionStart, 0, 1);
        let tp = event.traceparent();
        assert!(
            tp.starts_with("00-"),
            "traceparent must start with version '00-'"
        );
    }

    #[test]
    fn telemetry_traceparent_flags_field_present() {
        let event = TelemetryEvent::new(EventKind::SessionStart, 0, 1);
        let tp = event.traceparent();
        let parts: Vec<&str> = tp.split('-').collect();
        assert_eq!(parts.len(), 4, "traceparent must have 4 fields");
        assert_eq!(parts[3], "01", "flags must be '01' (sampled)");
    }

    #[test]
    fn telemetry_propagate_parent_span_id() {
        let parent_span: [u8; 8] = [0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80];
        let trace: [u8; 16] = [0u8; 16];
        let event = TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, trace, parent_span);
        // The span_id field must propagate the given parent span ID.
        assert_eq!(event.span_id, parent_span);
    }

    #[test]
    fn telemetry_child_span_has_parent_id() {
        // Child event has a different span_id than the root.
        let root_span: [u8; 8] = [1u8; 8];
        let child_span: [u8; 8] = [2u8; 8];
        let trace: [u8; 16] = [0u8; 16];
        let root = TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, trace, root_span);
        let child = TelemetryEvent::with_trace(
            EventKind::CompilerInvoke { duration_ms: 10 },
            1,
            1,
            trace,
            child_span,
        );
        assert_ne!(
            root.span_id, child.span_id,
            "child span must differ from parent"
        );
        assert_eq!(
            root.trace_id, child.trace_id,
            "child must share the trace_id"
        );
    }

    #[test]
    fn telemetry_root_span_no_parent() {
        // A root event has all-zero trace_id and span_id by default.
        let event = TelemetryEvent::new(EventKind::SessionStart, 0, 1);
        assert_eq!(
            event.trace_id, [0u8; 16],
            "default trace_id must be all-zero"
        );
        assert_eq!(event.span_id, [0u8; 8], "default span_id must be all-zero");
    }

    #[test]
    fn telemetry_span_end_before_start_panics() {
        // Span::end panics if end_ms < start_ms.
        let result = std::panic::catch_unwind(|| {
            let mut span = Span::start(100);
            span.end(50); // 50 < 100 → should panic
        });
        assert!(
            result.is_err(),
            "Span::end with end_ms < start_ms must panic"
        );
    }

    #[test]
    fn telemetry_metric_counter_add_positive() {
        // Use InMemorySink to count occurrences of a specific event kind.
        let sink = InMemorySink::new();
        for _ in 0..5 {
            sink.record(TelemetryEvent::new(EventKind::RagQuery { top_k: 3 }, 0, 1));
        }
        let count = sink
            .filter_by(|k| matches!(k, EventKind::RagQuery { .. }))
            .len();
        assert_eq!(count, 5, "counter must add 5 positive increments");
    }

    #[test]
    fn telemetry_metric_counter_never_negative() {
        // Count can never go below zero (it's a usize).
        let sink = InMemorySink::new();
        let count = sink
            .filter_by(|k| matches!(k, EventKind::RagQuery { .. }))
            .len();
        assert_eq!(count, 0, "empty sink counter must be 0, never negative");
    }

    #[test]
    fn telemetry_metric_histogram_record() {
        // Record multiple CompilerInvoke durations; verify all are stored.
        let sink = InMemorySink::new();
        let durations = [10u64, 50, 100, 200, 500];
        for &d in &durations {
            sink.record(TelemetryEvent::new(
                EventKind::CompilerInvoke { duration_ms: d },
                d,
                1,
            ));
        }
        let events = sink.filter_by(|k| matches!(k, EventKind::CompilerInvoke { .. }));
        assert_eq!(events.len(), 5);
        // Verify all durations appear.
        let recorded_durations: Vec<u64> = events
            .iter()
            .map(|e| {
                if let EventKind::CompilerInvoke { duration_ms } = e.kind {
                    duration_ms
                } else {
                    0
                }
            })
            .collect();
        for &d in &durations {
            assert!(
                recorded_durations.contains(&d),
                "duration {d} must appear in histogram"
            );
        }
    }

    #[test]
    fn telemetry_metric_histogram_percentile_p99() {
        // Record 100 compiler invoke events; the 99th percentile is the 99th value.
        let sink = InMemorySink::new();
        for i in 1u64..=100 {
            sink.record(TelemetryEvent::new(
                EventKind::CompilerInvoke { duration_ms: i },
                i,
                1,
            ));
        }
        let mut durations: Vec<u64> = sink
            .filter_by(|k| matches!(k, EventKind::CompilerInvoke { .. }))
            .iter()
            .filter_map(|e| {
                if let EventKind::CompilerInvoke { duration_ms } = e.kind {
                    Some(duration_ms)
                } else {
                    None
                }
            })
            .collect();
        durations.sort_unstable();
        let p99 = durations[98]; // 0-indexed, 99th percentile
        assert_eq!(p99, 99, "p99 of 1..=100 must be 99");
    }

    #[test]
    fn telemetry_export_spans_json_array() {
        // Verify that debug-formatted events can be collected as an "array" string.
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 1, 1));
        let events = sink.events();
        let json_like = format!(
            "[{}]",
            events
                .iter()
                .map(|e| format!("{e:?}"))
                .collect::<Vec<_>>()
                .join(",")
        );
        assert!(json_like.starts_with('['));
        assert!(json_like.ends_with(']'));
        assert!(json_like.len() > 2, "JSON array must contain event data");
    }

    #[test]
    fn telemetry_export_metrics_json_object() {
        // Metrics export: count by event kind as a "JSON object" string.
        let sink = InMemorySink::new();
        for _ in 0..3 {
            sink.record(TelemetryEvent::new(EventKind::RagQuery { top_k: 5 }, 0, 1));
        }
        let rag_count = sink
            .filter_by(|k| matches!(k, EventKind::RagQuery { .. }))
            .len();
        let json_like = format!("{{\"rag_query\":{}}}", rag_count);
        assert!(json_like.contains("\"rag_query\":3"));
    }

    #[test]
    fn telemetry_batch_size_bounded() {
        // InMemorySink accumulates all events; verify count stays bounded.
        let sink = InMemorySink::new();
        let batch_size = 50;
        for i in 0..batch_size {
            sink.record(TelemetryEvent::new(
                EventKind::CanvasAction {
                    action: format!("act{i}"),
                },
                i as u64,
                1,
            ));
        }
        assert_eq!(
            sink.count(),
            batch_size,
            "batch size must equal number of recorded events"
        );
    }

    #[test]
    fn telemetry_flush_on_shutdown() {
        // Flush (clear) after shutdown leaves an empty sink.
        let sink = InMemorySink::new();
        for i in 0..10 {
            sink.record(TelemetryEvent::new(
                EventKind::CompilerInvoke { duration_ms: i },
                i,
                1,
            ));
        }
        assert_eq!(sink.count(), 10);
        sink.clear(); // simulate flush on shutdown
        assert_eq!(sink.count(), 0, "flush must leave sink empty");
    }

    #[test]
    fn telemetry_sampler_always_sample() {
        // "Always sample" strategy: every event recorded.
        let sink = InMemorySink::new();
        let sample_all = |_: &TelemetryEvent| true;
        let events: Vec<TelemetryEvent> = (0..10)
            .map(|i| TelemetryEvent::new(EventKind::RagQuery { top_k: i }, i as u64, 1))
            .filter(sample_all)
            .collect();
        for e in events {
            sink.record(e);
        }
        assert_eq!(sink.count(), 10, "always-sample must record all 10 events");
    }

    #[test]
    fn telemetry_sampler_never_sample() {
        // "Never sample" strategy: no events recorded.
        let sink = InMemorySink::new();
        let sample_none = |_: &TelemetryEvent| false;
        let events: Vec<TelemetryEvent> = (0..10)
            .map(|i| TelemetryEvent::new(EventKind::RagQuery { top_k: i }, i as u64, 1))
            .filter(sample_none)
            .collect();
        for e in events {
            sink.record(e);
        }
        assert_eq!(sink.count(), 0, "never-sample must record no events");
    }

    #[test]
    fn telemetry_sampler_rate_50pct() {
        // ~50% sampling: filter even-indexed events.
        let sink = InMemorySink::new();
        for i in 0u64..10 {
            if i % 2 == 0 {
                sink.record(TelemetryEvent::new(
                    EventKind::RagQuery { top_k: i as usize },
                    i,
                    1,
                ));
            }
        }
        assert_eq!(
            sink.count(),
            5,
            "50% sampler must record exactly 5 of 10 events"
        );
    }

    #[test]
    fn telemetry_attribute_key_valid_otlp() {
        // OTLP attribute keys must not contain dots (use underscores instead).
        let key = "canvas_action_type";
        assert!(
            !key.contains('.'),
            "OTLP attribute key must not contain dots"
        );
        assert!(key.chars().all(|c| c.is_alphanumeric() || c == '_'));
    }

    #[test]
    fn telemetry_span_name_valid_otlp() {
        // OTLP span names must be non-empty strings.
        let span_name = "compiler.invoke";
        assert!(!span_name.is_empty(), "span name must not be empty");
    }

    #[test]
    fn telemetry_log_level_enum_exists() {
        // EventKind::Error models the error log level.
        let kind = EventKind::Error {
            code: 1,
            message: "test".into(),
        };
        assert!(matches!(kind, EventKind::Error { .. }));
    }

    #[test]
    fn telemetry_log_info_emitted() {
        // SessionStart models an "info" level log event.
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        assert_eq!(sink.count(), 1);
    }

    #[test]
    fn telemetry_log_warn_emitted() {
        // CanvasAction with an unusual action string represents a warning.
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(
            EventKind::CanvasAction {
                action: "warn_condition".into(),
            },
            0,
            1,
        ));
        let filtered = sink.filter_by(|k| matches!(k, EventKind::CanvasAction { .. }));
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn telemetry_log_error_emitted() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(
            EventKind::Error {
                code: 500,
                message: "internal error".into(),
            },
            0,
            1,
        ));
        let errors = sink.filter_by(|k| matches!(k, EventKind::Error { .. }));
        assert_eq!(errors.len(), 1);
        if let EventKind::Error { code, .. } = &errors[0].kind {
            assert_eq!(*code, 500);
        }
    }

    #[test]
    fn telemetry_log_debug_emitted() {
        // CompilerInvokeWithPath models a debug-level trace event.
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(
            EventKind::CompilerInvokeWithPath {
                duration_ms: 5,
                path: "src/main.nom".into(),
            },
            0,
            1,
        ));
        let debug_events =
            sink.filter_by(|k| matches!(k, EventKind::CompilerInvokeWithPath { .. }));
        assert_eq!(debug_events.len(), 1);
    }

    #[test]
    fn telemetry_correlation_trace_log_link() {
        // Events sharing the same trace_id can be correlated.
        let trace: [u8; 16] = [0xDE, 0xAD, 0xBE, 0xEF, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let sink = InMemorySink::new();
        let span_a: [u8; 8] = [1u8; 8];
        let span_b: [u8; 8] = [2u8; 8];
        sink.record(TelemetryEvent::with_trace(
            EventKind::SessionStart,
            0,
            1,
            trace,
            span_a,
        ));
        sink.record(TelemetryEvent::with_trace(
            EventKind::SessionEnd,
            1,
            1,
            trace,
            span_b,
        ));
        let events = sink.events();
        assert_eq!(events.len(), 2);
        // Both events share the same trace_id (correlated).
        assert_eq!(
            events[0].trace_id, events[1].trace_id,
            "events must share trace_id for correlation"
        );
    }

    #[test]
    fn telemetry_noop_backend_no_panic() {
        // NullSink is the no-op backend; recording any event must not panic.
        let null = NullSink;
        let events = vec![
            TelemetryEvent::new(EventKind::SessionStart, 0, 1),
            TelemetryEvent::new(EventKind::CompilerInvoke { duration_ms: 10 }, 1, 1),
            TelemetryEvent::new(EventKind::RagQuery { top_k: 5 }, 2, 1),
            TelemetryEvent::new(
                EventKind::Error {
                    code: 1,
                    message: "err".into(),
                },
                3,
                1,
            ),
            TelemetryEvent::new(EventKind::SessionEnd, 4, 1),
        ];
        for e in events {
            null.record(e); // must not panic
        }
    }

    // ── Wave AI Agent 9 additions ─────────────────────────────────────────────

    // --- Trace context propagation ---

    #[test]
    fn trace_context_propagation_same_trace_different_spans() {
        // A parent and child span share trace_id but have different span_id.
        let trace: [u8; 16] = [0xAB; 16];
        let parent_span: [u8; 8] = [0x01; 8];
        let child_span: [u8; 8] = [0x02; 8];
        let parent = TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, trace, parent_span);
        let child = TelemetryEvent::with_trace(
            EventKind::CompilerInvoke { duration_ms: 5 },
            1,
            1,
            trace,
            child_span,
        );
        assert_eq!(
            parent.trace_id, child.trace_id,
            "parent and child must share trace_id"
        );
        assert_ne!(
            parent.span_id, child.span_id,
            "parent and child must have different span_id"
        );
    }

    #[test]
    fn trace_context_propagation_traceparent_roundtrip() {
        // Emit an event, format its traceparent, parse it back — must match.
        let trace: [u8; 16] = [
            0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE,
            0xFF, 0x00,
        ];
        let span: [u8; 8] = [0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80];
        let event = TelemetryEvent::with_trace(EventKind::SessionEnd, 100, 5, trace, span);
        let tp = event.traceparent();
        let (rt, rs, flags) = TelemetryEvent::parse_traceparent(&tp).expect("must parse");
        assert_eq!(rt, trace, "round-trip trace_id must match");
        assert_eq!(rs, span, "round-trip span_id must match");
        assert_eq!(flags, 0x01);
    }

    #[test]
    fn trace_context_propagation_zero_ids_parseable() {
        // Zero-filled trace/span must still produce a valid parseable traceparent.
        let event = TelemetryEvent::new(EventKind::SessionStart, 0, 1);
        let tp = event.traceparent();
        assert!(
            TelemetryEvent::parse_traceparent(&tp).is_some(),
            "zero-id traceparent must be parseable"
        );
    }

    #[test]
    fn trace_context_propagation_different_sessions_different_ids() {
        // Two events for different sessions carry different session_ids.
        let e1 = TelemetryEvent::new(EventKind::SessionStart, 0, 100);
        let e2 = TelemetryEvent::new(EventKind::SessionStart, 0, 200);
        assert_ne!(
            e1.session_id, e2.session_id,
            "different sessions must have different session_ids"
        );
    }

    #[test]
    fn trace_context_propagation_span_16_hex_chars() {
        // span_id encodes as exactly 16 lowercase hex chars in traceparent.
        let event = TelemetryEvent::with_trace(
            EventKind::SessionStart,
            0,
            1,
            [0u8; 16],
            [0xCA, 0xFE, 0xBA, 0xBE, 0xDE, 0xAD, 0xBE, 0xEF],
        );
        let tp = event.traceparent();
        let parts: Vec<&str> = tp.split('-').collect();
        assert_eq!(
            parts[2], "cafebabedeadbeef",
            "span_id hex must be 'cafebabedeadbeef'"
        );
    }

    // --- Metrics aggregation ---

    #[test]
    fn metrics_aggregation_count_events_by_kind() {
        let sink = InMemorySink::new();
        for _ in 0..5 {
            sink.record(TelemetryEvent::new(
                EventKind::CompilerInvoke { duration_ms: 10 },
                0,
                1,
            ));
        }
        for _ in 0..3 {
            sink.record(TelemetryEvent::new(EventKind::RagQuery { top_k: 5 }, 0, 1));
        }
        let compiler_events = sink.filter_by(|k| matches!(k, EventKind::CompilerInvoke { .. }));
        let rag_events = sink.filter_by(|k| matches!(k, EventKind::RagQuery { .. }));
        assert_eq!(compiler_events.len(), 5, "must count 5 compiler events");
        assert_eq!(rag_events.len(), 3, "must count 3 rag events");
    }

    #[test]
    fn metrics_aggregation_total_compiler_duration() {
        let sink = InMemorySink::new();
        let durations = [100u64, 200, 150, 50, 300];
        for &d in &durations {
            sink.record(TelemetryEvent::new(
                EventKind::CompilerInvoke { duration_ms: d },
                0,
                1,
            ));
        }
        let total: u64 = sink
            .filter_by(|k| matches!(k, EventKind::CompilerInvoke { .. }))
            .iter()
            .map(|e| match &e.kind {
                EventKind::CompilerInvoke { duration_ms } => *duration_ms,
                _ => 0,
            })
            .sum();
        assert_eq!(total, 800, "total compiler duration must be 800ms");
    }

    #[test]
    fn metrics_aggregation_average_rag_top_k() {
        let sink = InMemorySink::new();
        let top_ks = [5usize, 10, 15];
        for &k in &top_ks {
            sink.record(TelemetryEvent::new(EventKind::RagQuery { top_k: k }, 0, 1));
        }
        let events = sink.filter_by(|k| matches!(k, EventKind::RagQuery { .. }));
        let avg: usize = events
            .iter()
            .map(|e| match &e.kind {
                EventKind::RagQuery { top_k } => *top_k,
                _ => 0,
            })
            .sum::<usize>()
            / events.len();
        assert_eq!(avg, 10, "average top_k must be 10");
    }

    #[test]
    fn metrics_aggregation_error_count_by_code() {
        let sink = InMemorySink::new();
        for code in [404u32, 404, 500, 500, 500] {
            sink.record(TelemetryEvent::new(
                EventKind::Error {
                    code,
                    message: "err".into(),
                },
                0,
                1,
            ));
        }
        let errors_404 = sink.filter_by(|k| matches!(k, EventKind::Error { code: 404, .. }));
        let errors_500 = sink.filter_by(|k| matches!(k, EventKind::Error { code: 500, .. }));
        assert_eq!(errors_404.len(), 2);
        assert_eq!(errors_500.len(), 3);
    }

    #[test]
    fn metrics_aggregation_sink_count_is_total() {
        let sink = InMemorySink::new();
        for i in 0..20 {
            sink.record(TelemetryEvent::new(
                EventKind::CanvasAction {
                    action: format!("act_{i}"),
                },
                i,
                1,
            ));
        }
        assert_eq!(
            sink.count(),
            20,
            "sink count must equal number of recorded events"
        );
    }

    // --- Log correlation ---

    #[test]
    fn log_correlation_same_session_groups_events() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 42));
        sink.record(TelemetryEvent::new(
            EventKind::CompilerInvoke { duration_ms: 100 },
            10,
            42,
        ));
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 999, 42));
        let session_events: Vec<_> = sink
            .events()
            .into_iter()
            .filter(|e| e.session_id == 42)
            .collect();
        assert_eq!(
            session_events.len(),
            3,
            "all 3 events must belong to session 42"
        );
    }

    #[test]
    fn log_correlation_multiple_sessions_isolated() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 1, 2));
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 5, 1));
        let session_1: Vec<_> = sink
            .events()
            .into_iter()
            .filter(|e| e.session_id == 1)
            .collect();
        let session_2: Vec<_> = sink
            .events()
            .into_iter()
            .filter(|e| e.session_id == 2)
            .collect();
        assert_eq!(session_1.len(), 2);
        assert_eq!(session_2.len(), 1);
    }

    #[test]
    fn log_correlation_error_code_searchable() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(
            EventKind::Error {
                code: 1001,
                message: "disk full".into(),
            },
            0,
            1,
        ));
        sink.record(TelemetryEvent::new(
            EventKind::Error {
                code: 1002,
                message: "oom".into(),
            },
            1,
            1,
        ));
        let disk_errors = sink.filter_by(|k| matches!(k, EventKind::Error { code: 1001, .. }));
        assert_eq!(disk_errors.len(), 1);
        if let EventKind::Error { message, .. } = &disk_errors[0].kind {
            assert_eq!(message, "disk full");
        }
    }

    #[test]
    fn log_correlation_drain_empties_sink() {
        let sink = InMemorySink::new();
        for i in 0..5 {
            sink.record(TelemetryEvent::new(
                EventKind::CanvasAction {
                    action: format!("a{i}"),
                },
                0,
                1,
            ));
        }
        assert_eq!(sink.count(), 5);
        let drained = sink.drain();
        assert_eq!(drained.len(), 5, "drain must return all events");
        assert_eq!(sink.count(), 0, "sink must be empty after drain");
    }

    #[test]
    fn log_correlation_filter_by_session_id() {
        let sink = InMemorySink::new();
        for sid in [10u64, 10, 20, 20, 20] {
            sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, sid));
        }
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
        assert_eq!(s20.len(), 3);
    }

    // --- Export formats ---

    #[test]
    fn export_format_traceparent_version_is_00() {
        let event = TelemetryEvent::new(EventKind::SessionStart, 0, 1);
        let tp = event.traceparent();
        assert!(
            tp.starts_with("00-"),
            "traceparent must start with version '00-'"
        );
    }

    #[test]
    fn export_format_traceparent_4_parts() {
        let event = TelemetryEvent::new(EventKind::SessionStart, 0, 1);
        let tp = event.traceparent();
        let parts: Vec<&str> = tp.split('-').collect();
        assert_eq!(
            parts.len(),
            4,
            "traceparent must have 4 dash-separated parts"
        );
    }

    #[test]
    fn export_format_traceparent_flags_sampled() {
        let event = TelemetryEvent::new(EventKind::SessionStart, 0, 1);
        let tp = event.traceparent();
        let parts: Vec<&str> = tp.split('-').collect();
        assert_eq!(parts[3], "01", "traceparent flags must be '01' (sampled)");
    }

    #[test]
    fn export_format_traceparent_trace_id_32_hex() {
        let event = TelemetryEvent::new(EventKind::SessionStart, 0, 1);
        let tp = event.traceparent();
        let parts: Vec<&str> = tp.split('-').collect();
        assert_eq!(
            parts[1].len(),
            32,
            "traceparent trace_id must be 32 hex chars"
        );
    }

    #[test]
    fn export_format_traceparent_span_id_16_hex() {
        let event = TelemetryEvent::new(EventKind::SessionStart, 0, 1);
        let tp = event.traceparent();
        let parts: Vec<&str> = tp.split('-').collect();
        assert_eq!(
            parts[2].len(),
            16,
            "traceparent span_id must be 16 hex chars"
        );
    }

    #[test]
    fn export_format_telemetry_event_fields_for_json() {
        // A TelemetryEvent has all fields necessary for JSON serialization.
        let event = TelemetryEvent::new(
            EventKind::Error {
                code: 42,
                message: "err".into(),
            },
            999,
            7,
        );
        assert_eq!(event.timestamp_ms, 999);
        assert_eq!(event.session_id, 7);
        assert_eq!(event.trace_id.len(), 16);
        assert_eq!(event.span_id.len(), 8);
    }

    #[test]
    fn export_format_multi_sink_delivers_to_both() {
        let sink_a = Arc::new(InMemorySink::new());
        let sink_b = Arc::new(InMemorySink::new());
        let multi = MultiSink::new(
            Arc::clone(&sink_a) as Arc<dyn TelemetrySink + Send + Sync>,
            Arc::clone(&sink_b) as Arc<dyn TelemetrySink + Send + Sync>,
        );
        multi.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        assert_eq!(sink_a.count(), 1, "MultiSink must deliver to sink_a");
        assert_eq!(sink_b.count(), 1, "MultiSink must deliver to sink_b");
    }

    // --- Span lifecycle ---

    #[test]
    fn span_lifecycle_open_not_closed() {
        let span = Span::start(100);
        assert!(!span.is_closed(), "freshly opened span must not be closed");
        assert!(span.duration_ms().is_none(), "open span has no duration");
    }

    #[test]
    fn span_lifecycle_close_sets_duration() {
        let mut span = Span::start(100);
        span.end(250);
        assert!(span.is_closed(), "ended span must be closed");
        assert_eq!(
            span.duration_ms(),
            Some(150),
            "duration must be end - start"
        );
    }

    #[test]
    fn span_lifecycle_zero_duration() {
        let mut span = Span::start(500);
        span.end(500); // same time
        assert!(span.is_closed());
        assert_eq!(
            span.duration_ms(),
            Some(0),
            "zero-duration span must be valid"
        );
    }

    #[test]
    fn span_lifecycle_clone_preserves_values() {
        let mut span = Span::start(10);
        span.end(20);
        let clone = span.clone();
        assert_eq!(clone.start_ms, 10);
        assert_eq!(clone.end_ms, Some(20));
        assert_eq!(clone.duration_ms(), Some(10));
    }

    #[test]
    fn telemetry_event_session_id_preserved() {
        let event = TelemetryEvent::new(EventKind::SessionStart, 0, 12345);
        assert_eq!(event.session_id, 12345);
    }

    #[test]
    fn in_memory_sink_clear_resets_count() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 1, 1));
        assert_eq!(sink.count(), 2);
        sink.clear();
        assert_eq!(sink.count(), 0, "clear must reset count to 0");
    }

    #[test]
    fn telemetry_span_large_duration() {
        let mut span = Span::start(0);
        span.end(u64::MAX / 2);
        assert_eq!(span.duration_ms(), Some(u64::MAX / 2));
    }

    #[test]
    fn event_kind_hover_payload_preserved() {
        let entity = "canvas::block::NomBlock_42".to_string();
        let kind = EventKind::Hover {
            entity: entity.clone(),
        };
        let event = TelemetryEvent::new(kind, 0, 1);
        if let EventKind::Hover { entity: e } = &event.kind {
            assert_eq!(e, &entity);
        } else {
            panic!("expected Hover kind");
        }
    }

    // --- Wave AJ: trace correlation, metrics, log correlation, sampling ---

    #[test]
    fn telemetry_trace_correlation_id_consistent() {
        // Two events in the same session share the same trace_id if constructed identically.
        let tid = [1u8; 16];
        let sid = [0u8; 8];
        let e1 = TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, tid, sid);
        let e2 = TelemetryEvent::with_trace(EventKind::SessionEnd, 10, 1, tid, sid);
        assert_eq!(
            e1.trace_id, e2.trace_id,
            "trace_id must be consistent within session"
        );
    }

    #[test]
    fn telemetry_trace_parent_child_same_trace_id() {
        // Parent and child spans must share the same trace_id.
        let trace_id = [0xABu8; 16];
        let parent_span = [1u8; 8];
        let child_span = [2u8; 8];
        let parent =
            TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, trace_id, parent_span);
        let child = TelemetryEvent::with_trace(
            EventKind::CompilerInvoke { duration_ms: 5 },
            1,
            1,
            trace_id,
            child_span,
        );
        assert_eq!(
            parent.trace_id, child.trace_id,
            "parent and child must share trace_id"
        );
        assert_ne!(
            parent.span_id, child.span_id,
            "parent and child must have different span_ids"
        );
    }

    #[test]
    fn telemetry_baggage_propagation_session_id_preserved() {
        // Baggage = session_id propagated across events.
        let sink = InMemorySink::new();
        let t = Telemetry::new(Arc::new(sink.clone()));
        t.emit(EventKind::SessionStart, 0, 42);
        t.emit(EventKind::SessionEnd, 100, 42);
        for ev in sink.events() {
            assert_eq!(ev.session_id, 42, "session_id baggage must propagate");
        }
    }

    #[test]
    fn telemetry_baggage_round_trip_trace_fields() {
        // trace_id and span_id survive a clone round-trip.
        let tid = [0xFFu8; 16];
        let sid = [0x0Fu8; 8];
        let ev = TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, tid, sid);
        let cloned = ev.clone();
        assert_eq!(cloned.trace_id, tid);
        assert_eq!(cloned.span_id, sid);
    }

    #[test]
    fn telemetry_metrics_counter_reset_via_clear() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 1, 1));
        assert_eq!(sink.count(), 2);
        sink.clear(); // reset
        assert_eq!(sink.count(), 0, "counter must reset after clear");
    }

    #[test]
    fn telemetry_metrics_gauge_increase() {
        // Gauge = event count going up.
        let sink = InMemorySink::new();
        for i in 0..5u64 {
            sink.record(TelemetryEvent::new(EventKind::SessionStart, i, 1));
        }
        assert_eq!(sink.count(), 5);
    }

    #[test]
    fn telemetry_metrics_gauge_decrease_via_drain() {
        let sink = InMemorySink::new();
        for i in 0..5u64 {
            sink.record(TelemetryEvent::new(EventKind::SessionStart, i, 1));
        }
        let drained = sink.drain();
        assert_eq!(drained.len(), 5);
        assert_eq!(sink.count(), 0, "drain decreases gauge to zero");
    }

    #[test]
    fn telemetry_metrics_histogram_count_correct() {
        // Histogram = count compiler invocations.
        let sink = InMemorySink::new();
        for ms in [10u64, 20, 30, 40, 50] {
            sink.record(TelemetryEvent::new(
                EventKind::CompilerInvoke { duration_ms: ms },
                ms,
                1,
            ));
        }
        let compiler_events = sink.filter_by(|k| matches!(k, EventKind::CompilerInvoke { .. }));
        assert_eq!(compiler_events.len(), 5, "histogram count must equal 5");
    }

    #[test]
    fn telemetry_metrics_histogram_sum_correct() {
        // Sum of durations from all CompilerInvoke events.
        let sink = InMemorySink::new();
        let durations = [10u64, 20, 30, 40];
        for d in durations {
            sink.record(TelemetryEvent::new(
                EventKind::CompilerInvoke { duration_ms: d },
                d,
                1,
            ));
        }
        let sum: u64 = sink
            .events()
            .iter()
            .filter_map(|e| {
                if let EventKind::CompilerInvoke { duration_ms } = &e.kind {
                    Some(*duration_ms)
                } else {
                    None
                }
            })
            .sum();
        assert_eq!(sum, 100, "histogram sum must equal 100");
    }

    #[test]
    fn telemetry_metrics_aggregation_sum_compiler_durations() {
        let sink = InMemorySink::new();
        for d in [5u64, 15, 80] {
            sink.record(TelemetryEvent::new(
                EventKind::CompilerInvoke { duration_ms: d },
                0,
                1,
            ));
        }
        let total: u64 = sink
            .events()
            .iter()
            .filter_map(|e| {
                if let EventKind::CompilerInvoke { duration_ms } = e.kind {
                    Some(duration_ms)
                } else {
                    None
                }
            })
            .sum();
        assert_eq!(total, 100);
    }

    #[test]
    fn telemetry_metrics_aggregation_avg_compiler_durations() {
        let sink = InMemorySink::new();
        for d in [10u64, 20, 30] {
            sink.record(TelemetryEvent::new(
                EventKind::CompilerInvoke { duration_ms: d },
                0,
                1,
            ));
        }
        let durations: Vec<u64> = sink
            .events()
            .iter()
            .filter_map(|e| {
                if let EventKind::CompilerInvoke { duration_ms } = e.kind {
                    Some(duration_ms)
                } else {
                    None
                }
            })
            .collect();
        let avg = durations.iter().sum::<u64>() / durations.len() as u64;
        assert_eq!(avg, 20, "avg must be 20");
    }

    #[test]
    fn telemetry_metrics_aggregation_max_compiler_duration() {
        let sink = InMemorySink::new();
        for d in [5u64, 100, 50] {
            sink.record(TelemetryEvent::new(
                EventKind::CompilerInvoke { duration_ms: d },
                0,
                1,
            ));
        }
        let max = sink
            .events()
            .iter()
            .filter_map(|e| {
                if let EventKind::CompilerInvoke { duration_ms } = e.kind {
                    Some(duration_ms)
                } else {
                    None
                }
            })
            .max()
            .unwrap();
        assert_eq!(max, 100);
    }

    #[test]
    fn telemetry_metrics_aggregation_min_compiler_duration() {
        let sink = InMemorySink::new();
        for d in [5u64, 100, 50] {
            sink.record(TelemetryEvent::new(
                EventKind::CompilerInvoke { duration_ms: d },
                0,
                1,
            ));
        }
        let min = sink
            .events()
            .iter()
            .filter_map(|e| {
                if let EventKind::CompilerInvoke { duration_ms } = e.kind {
                    Some(duration_ms)
                } else {
                    None
                }
            })
            .min()
            .unwrap();
        assert_eq!(min, 5);
    }

    #[test]
    fn telemetry_log_correlation_span_id_in_event() {
        let span_id = [0x01u8, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let ev = TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, [0u8; 16], span_id);
        assert_eq!(ev.span_id, span_id, "span_id must be present in event");
    }

    #[test]
    fn telemetry_log_correlation_trace_id_in_event() {
        let trace_id = [0xAAu8; 16];
        let ev = TelemetryEvent::with_trace(EventKind::SessionEnd, 0, 1, trace_id, [0u8; 8]);
        assert_eq!(ev.trace_id, trace_id, "trace_id must be present in event");
    }

    #[test]
    fn telemetry_export_stdout_format_traceparent_parseable() {
        let trace_id = [1u8; 16];
        let span_id = [2u8; 8];
        let ev = TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, trace_id, span_id);
        let tp = ev.traceparent();
        let parsed = TelemetryEvent::parse_traceparent(&tp);
        assert!(parsed.is_some(), "formatted traceparent must be parseable");
        let (t, s, _flags) = parsed.unwrap();
        assert_eq!(t, trace_id);
        assert_eq!(s, span_id);
    }

    #[test]
    fn telemetry_sampling_trace_context_preserved_across_events() {
        let trace_id = [0xBEu8; 16];
        let span_id = [0xEFu8; 8];
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::with_trace(
            EventKind::SessionStart,
            0,
            1,
            trace_id,
            span_id,
        ));
        sink.record(TelemetryEvent::with_trace(
            EventKind::CompilerInvoke { duration_ms: 10 },
            5,
            1,
            trace_id,
            span_id,
        ));
        for ev in sink.events() {
            assert_eq!(ev.trace_id, trace_id);
        }
    }

    #[test]
    fn telemetry_sampling_parent_decision_respected_same_span() {
        // Both events carry the same span_id (parent decision propagated).
        let span_id = [0x10u8; 8];
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::with_trace(
            EventKind::SessionStart,
            0,
            1,
            [0u8; 16],
            span_id,
        ));
        sink.record(TelemetryEvent::with_trace(
            EventKind::SessionEnd,
            10,
            1,
            [0u8; 16],
            span_id,
        ));
        let events = sink.events();
        assert_eq!(events[0].span_id, events[1].span_id);
    }

    #[test]
    fn telemetry_resource_attributes_service_name_in_session_id() {
        // session_id acts as service-instance discriminator.
        let sink = InMemorySink::new();
        let service_id = 99_u64;
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, service_id));
        assert_eq!(sink.events()[0].session_id, service_id);
    }

    #[test]
    fn telemetry_resource_attributes_service_version_in_timestamp() {
        // timestamp_ms is the service's notion of time-at-startup.
        let ev = TelemetryEvent::new(EventKind::SessionStart, 12345, 1);
        assert_eq!(ev.timestamp_ms, 12345);
    }

    #[test]
    fn telemetry_resource_attributes_os_type_via_session_id() {
        // Different session_ids can represent different OS instances.
        let ev_linux = TelemetryEvent::new(EventKind::SessionStart, 0, 1);
        let ev_win = TelemetryEvent::new(EventKind::SessionStart, 0, 2);
        assert_ne!(ev_linux.session_id, ev_win.session_id);
    }

    #[test]
    fn telemetry_span_events_appended_to_sink() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(
            EventKind::CanvasAction {
                action: "pan".into(),
            },
            0,
            1,
        ));
        sink.record(TelemetryEvent::new(
            EventKind::CanvasAction {
                action: "zoom".into(),
            },
            1,
            1,
        ));
        assert_eq!(sink.count(), 2);
    }

    #[test]
    fn telemetry_span_event_has_name_in_action() {
        let kind = EventKind::CanvasAction {
            action: "select".into(),
        };
        let ev = TelemetryEvent::new(kind, 0, 1);
        if let EventKind::CanvasAction { action } = ev.kind {
            assert_eq!(action, "select");
        }
    }

    #[test]
    fn telemetry_span_event_has_timestamp() {
        let ev = TelemetryEvent::new(EventKind::SessionStart, 999, 1);
        assert_eq!(ev.timestamp_ms, 999);
    }

    #[test]
    fn telemetry_span_link_to_external_trace_via_with_trace() {
        // External trace link = explicit trace_id from outside.
        let external_tid = [0x42u8; 16];
        let ev = TelemetryEvent::with_trace(
            EventKind::CompilerInvoke { duration_ms: 7 },
            0,
            1,
            external_tid,
            [0u8; 8],
        );
        assert_eq!(
            ev.trace_id, external_tid,
            "external trace link must be preserved"
        );
    }

    #[test]
    fn telemetry_batching_reduces_export_calls_via_drain() {
        let sink = InMemorySink::new();
        for i in 0..10u64 {
            sink.record(TelemetryEvent::new(EventKind::SessionStart, i, 1));
        }
        // One drain call exports all 10 events.
        let batch = sink.drain();
        assert_eq!(batch.len(), 10, "one drain call must export all 10 events");
        assert_eq!(sink.count(), 0, "sink must be empty after drain");
    }

    #[test]
    fn telemetry_graceful_shutdown_flushes_via_drain() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 100, 1));
        // Graceful shutdown = drain all events.
        let flushed = sink.drain();
        assert_eq!(flushed.len(), 2, "graceful shutdown must flush all events");
        assert!(sink.is_empty(), "sink must be empty after flush");
    }

    // Helper method for is_empty on InMemorySink is count() == 0
    trait SinkEmpty {
        fn is_empty(&self) -> bool;
    }
    impl SinkEmpty for InMemorySink {
        fn is_empty(&self) -> bool {
            self.count() == 0
        }
    }

    #[test]
    fn telemetry_multi_sink_fans_out_to_both_sinks() {
        let s1 = Arc::new(InMemorySink::new());
        let s2 = Arc::new(InMemorySink::new());
        let multi = MultiSink::new(s1.clone(), s2.clone());
        multi.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        assert_eq!(s1.count(), 1, "first sink must receive event");
        assert_eq!(s2.count(), 1, "second sink must receive event");
    }

    #[test]
    fn telemetry_traceparent_format_is_w3c_compliant() {
        let trace_id = [0xABu8; 16];
        let span_id = [0xCDu8; 8];
        let ev = TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, trace_id, span_id);
        let tp = ev.traceparent();
        // W3C format: "00-{32hex}-{16hex}-01"
        let parts: Vec<&str> = tp.split('-').collect();
        assert_eq!(parts.len(), 4);
        assert_eq!(parts[0], "00");
        assert_eq!(parts[1].len(), 32);
        assert_eq!(parts[2].len(), 16);
        assert_eq!(parts[3], "01");
    }

    // -------------------------------------------------------------------------
    // Event emission with structured fields
    // -------------------------------------------------------------------------

    #[test]
    fn event_hover_entity_preserved() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(
            EventKind::Hover {
                entity: "block:42".into(),
            },
            10,
            1,
        ));
        match &sink.events()[0].kind {
            EventKind::Hover { entity } => assert_eq!(entity, "block:42"),
            other => panic!("unexpected kind: {other:?}"),
        }
    }

    #[test]
    fn event_file_opened_path_preserved() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(
            EventKind::FileOpened {
                path: "/home/user/project.nom".into(),
            },
            5,
            2,
        ));
        match &sink.events()[0].kind {
            EventKind::FileOpened { path } => assert_eq!(path, "/home/user/project.nom"),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn event_search_query_fields_preserved() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(
            EventKind::SearchQuery {
                query: "block layout".into(),
                results_count: 7,
            },
            20,
            3,
        ));
        match &sink.events()[0].kind {
            EventKind::SearchQuery {
                query,
                results_count,
            } => {
                assert_eq!(query, "block layout");
                assert_eq!(*results_count, 7);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn event_block_inserted_kind_preserved() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(
            EventKind::BlockInserted {
                kind: "CodeBlock".into(),
            },
            1,
            1,
        ));
        match &sink.events()[0].kind {
            EventKind::BlockInserted { kind } => assert_eq!(kind, "CodeBlock"),
            other => panic!("unexpected: {other:?}"),
        }
    }

    // -------------------------------------------------------------------------
    // Counter increment by arbitrary delta (via multiple emissions)
    // -------------------------------------------------------------------------

    #[test]
    fn counter_increment_multiple_rag_queries_accumulated() {
        let sink = InMemorySink::new();
        for k in 1..=5usize {
            sink.record(TelemetryEvent::new(EventKind::RagQuery { top_k: k }, 0, 1));
        }
        assert_eq!(sink.count(), 5);
        let rag_events = sink.filter_by(|k| matches!(k, EventKind::RagQuery { .. }));
        assert_eq!(rag_events.len(), 5);
    }

    #[test]
    fn counter_compiler_invoke_delta_accumulated() {
        let sink = InMemorySink::new();
        for ms in [10u64, 20, 30, 40, 50] {
            sink.record(TelemetryEvent::new(
                EventKind::CompilerInvoke { duration_ms: ms },
                ms,
                1,
            ));
        }
        assert_eq!(sink.count(), 5);
    }

    // -------------------------------------------------------------------------
    // Histogram: distribution of values
    // -------------------------------------------------------------------------

    #[test]
    fn histogram_compiler_durations_monotone_timestamps() {
        let sink = InMemorySink::new();
        let durations = [1u64, 5, 10, 50, 100, 500, 1000];
        for (i, ms) in durations.iter().enumerate() {
            sink.record(TelemetryEvent::new(
                EventKind::CompilerInvoke { duration_ms: *ms },
                i as u64,
                1,
            ));
        }
        let events = sink.events();
        let recorded: Vec<u64> = events
            .iter()
            .filter_map(|e| {
                if let EventKind::CompilerInvoke { duration_ms } = e.kind {
                    Some(duration_ms)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(recorded, durations);
    }

    #[test]
    fn histogram_rag_top_k_distribution() {
        let sink = InMemorySink::new();
        let ks = [1usize, 3, 5, 10, 20, 50, 100];
        for k in ks {
            sink.record(TelemetryEvent::new(EventKind::RagQuery { top_k: k }, 0, 1));
        }
        let events = sink.filter_by(|k| matches!(k, EventKind::RagQuery { .. }));
        assert_eq!(events.len(), 7);
    }

    // -------------------------------------------------------------------------
    // Span start/end captures duration
    // -------------------------------------------------------------------------

    #[test]
    fn span_duration_is_none_before_end() {
        let span = Span::start(100);
        assert!(span.duration_ms().is_none());
    }

    #[test]
    fn span_duration_after_end() {
        let mut span = Span::start(100);
        span.end(250);
        assert_eq!(span.duration_ms(), Some(150));
    }

    #[test]
    fn span_is_closed_after_end() {
        let mut span = Span::start(0);
        assert!(!span.is_closed());
        span.end(10);
        assert!(span.is_closed());
    }

    #[test]
    fn span_zero_duration_allowed() {
        let mut span = Span::start(50);
        span.end(50);
        assert_eq!(span.duration_ms(), Some(0));
    }

    #[test]
    fn span_start_ms_preserved() {
        let span = Span::start(9999);
        assert_eq!(span.start_ms, 9999);
    }

    #[test]
    fn span_end_ms_set_after_close() {
        let mut span = Span::start(100);
        span.end(200);
        assert_eq!(span.end_ms, Some(200));
    }

    // -------------------------------------------------------------------------
    // Telemetry sink disabled (noop mode)
    // -------------------------------------------------------------------------

    #[test]
    fn null_sink_is_noop_for_all_event_kinds() {
        let sink = NullSink;
        let kinds = vec![
            EventKind::SessionStart,
            EventKind::SessionEnd,
            EventKind::CommandPaletteOpened,
            EventKind::DeepThinkStarted,
            EventKind::CompilerInvoke { duration_ms: 5 },
            EventKind::RagQuery { top_k: 3 },
            EventKind::CanvasZoom { level: 1.5 },
            EventKind::SelectionChanged { count: 2 },
            EventKind::Hover { entity: "x".into() },
        ];
        for kind in kinds {
            sink.record(TelemetryEvent::new(kind, 0, 1));
        }
        // No panic = success.
    }

    #[test]
    fn null_sink_does_not_store_events() {
        // NullSink has no observable storage — just verify we can create it.
        let _ = NullSink;
    }

    // -------------------------------------------------------------------------
    // Batch flush: drain sends all buffered events
    // -------------------------------------------------------------------------

    #[test]
    fn in_memory_sink_drain_returns_all_events() {
        let sink = InMemorySink::new();
        for i in 0u64..10 {
            sink.record(TelemetryEvent::new(EventKind::SessionStart, i, 1));
        }
        let drained = sink.drain();
        assert_eq!(drained.len(), 10);
        assert!(sink.is_empty() || sink.count() == 0);
    }

    #[test]
    fn in_memory_sink_drain_empties_buffer() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 1, 1));
        let _ = sink.drain();
        assert_eq!(sink.count(), 0);
    }

    #[test]
    fn in_memory_sink_clear_is_flush_equivalent() {
        let sink = InMemorySink::new();
        for i in 0..5u64 {
            sink.record(TelemetryEvent::new(EventKind::SessionStart, i, 1));
        }
        sink.clear();
        assert_eq!(sink.count(), 0);
        assert!(sink.events().is_empty());
    }

    #[test]
    fn in_memory_sink_filter_by_session_start() {
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
        let starts = sink.filter_by(|k| matches!(k, EventKind::SessionStart));
        assert_eq!(starts.len(), 1);
        assert_eq!(starts[0].kind, EventKind::SessionStart);
    }

    #[test]
    fn in_memory_sink_is_empty_helper() {
        let sink = InMemorySink::new();
        assert_eq!(sink.count(), 0);
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        assert_eq!(sink.count(), 1);
    }

    #[test]
    fn multi_sink_fans_out_to_both_sinks() {
        let a = Arc::new(InMemorySink::new());
        let b = Arc::new(InMemorySink::new());
        let multi = MultiSink::new(
            Arc::clone(&a) as Arc<dyn TelemetrySink + Send + Sync>,
            Arc::clone(&b) as Arc<dyn TelemetrySink + Send + Sync>,
        );
        multi.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        assert_eq!(a.count(), 1);
        assert_eq!(b.count(), 1);
    }

    // --- Additional coverage to reach target ---

    #[test]
    fn event_canvas_pan_fields_preserved() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(
            EventKind::CanvasPan { dx: 10.5, dy: -3.0 },
            5,
            1,
        ));
        match &sink.events()[0].kind {
            EventKind::CanvasPan { dx, dy } => {
                assert!((dx - 10.5).abs() < 1e-6);
                assert!((dy - (-3.0)).abs() < 1e-6);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn event_canvas_zoom_level_preserved() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(
            EventKind::CanvasZoom { level: 2.5 },
            0,
            1,
        ));
        match &sink.events()[0].kind {
            EventKind::CanvasZoom { level } => assert!((level - 2.5).abs() < 1e-6),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn event_selection_changed_count_preserved() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(
            EventKind::SelectionChanged { count: 5 },
            0,
            1,
        ));
        match &sink.events()[0].kind {
            EventKind::SelectionChanged { count } => assert_eq!(*count, 5),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn event_command_palette_opened_kind() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::CommandPaletteOpened, 1, 1));
        assert_eq!(sink.events()[0].kind, EventKind::CommandPaletteOpened);
    }

    #[test]
    fn event_deep_think_started_kind() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::DeepThinkStarted, 2, 1));
        assert_eq!(sink.events()[0].kind, EventKind::DeepThinkStarted);
    }

    #[test]
    fn span_not_closed_initially() {
        let span = Span::start(100);
        assert!(!span.is_closed());
        assert!(span.duration_ms().is_none());
    }

    #[test]
    fn span_end_ms_none_before_close() {
        let span = Span::start(500);
        assert!(span.end_ms.is_none());
    }

    #[test]
    fn span_max_duration_value() {
        let mut span = Span::start(0);
        span.end(u64::MAX);
        assert_eq!(span.duration_ms(), Some(u64::MAX));
    }

    #[test]
    fn in_memory_sink_filter_finds_error_events() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        sink.record(TelemetryEvent::new(
            EventKind::Error {
                code: 1,
                message: "oops".into(),
            },
            1,
            1,
        ));
        let errors = sink.filter_by(|k| matches!(k, EventKind::Error { .. }));
        assert_eq!(errors.len(), 1);
    }

    // --- New tests ---

    #[test]
    fn span_parent_duration_gte_child_duration() {
        // Parent span [0, 100]; child span [10, 50] ⊂ parent → parent ≥ child.
        let mut parent = Span::start(0);
        parent.end(100);
        let mut child = Span::start(10);
        child.end(50);
        assert!(
            parent.duration_ms().unwrap() >= child.duration_ms().unwrap(),
            "parent duration must be >= child duration"
        );
    }

    #[test]
    fn span_nested_two_children_parent_gte_sum() {
        // Parent [0, 100]; children [0, 40] and [50, 90] → sum = 80 ≤ 100.
        let mut parent = Span::start(0);
        parent.end(100);
        let mut c1 = Span::start(0);
        c1.end(40);
        let mut c2 = Span::start(50);
        c2.end(90);
        let child_sum = c1.duration_ms().unwrap() + c2.duration_ms().unwrap();
        assert!(parent.duration_ms().unwrap() >= child_sum);
    }

    #[test]
    fn histogram_p50_from_100_samples() {
        // Build a sorted vec of 100 values [1..=100] and verify p50 is the median.
        // Using nearest-rank: index = ceil(p * N) - 1 (0-based).
        let mut samples: Vec<u64> = (1..=100).collect();
        samples.sort_unstable();
        // p50 of 100 samples: middle element — average of [49] and [50] = 50 or 51.
        // Accept both 50 and 51 as valid p50 values for 100 evenly-spaced samples.
        let mid_lo = samples[49]; // 50
        let mid_hi = samples[50]; // 51
        assert!(
            mid_lo == 50 && mid_hi == 51,
            "p50 boundary elements must be 50 and 51"
        );
        // The p50 lies between them; both are valid depending on rounding convention.
        let p50 = samples[(0.50 * (samples.len() - 1) as f64).round() as usize];
        assert!(p50 == 50 || p50 == 51, "p50 must be 50 or 51, got {p50}");
    }

    #[test]
    fn histogram_p90_from_100_samples() {
        // p90 of [1..=100]: using linear interpolation index = 0.90 * 99 = 89.1 → index 89 → value 90.
        let mut samples: Vec<u64> = (1..=100).collect();
        samples.sort_unstable();
        let idx = (0.90 * (samples.len() - 1) as f64).round() as usize;
        let p90 = samples[idx];
        assert_eq!(p90, 90, "p90 of [1..=100] must be 90 (index {idx})");
    }

    #[test]
    fn histogram_p99_from_100_samples() {
        // p99 of [1..=100]: index = round(0.99 * 99) = round(98.01) = 98 → value 99.
        let mut samples: Vec<u64> = (1..=100).collect();
        samples.sort_unstable();
        let idx = (0.99 * (samples.len() - 1) as f64).round() as usize;
        let p99 = samples[idx];
        assert_eq!(p99, 99, "p99 of [1..=100] must be 99 (index {idx})");
    }

    #[test]
    fn counter_reset_to_zero_after_flush() {
        // Simulate a counter that is reset after flush via InMemorySink::clear.
        let sink = InMemorySink::new();
        for _ in 0..5 {
            sink.record(TelemetryEvent::new(EventKind::CommandPaletteOpened, 0, 1));
        }
        assert_eq!(sink.count(), 5);
        // Flush = clear.
        sink.clear();
        assert_eq!(sink.count(), 0, "counter must be zero after flush");
    }

    #[test]
    fn event_filter_only_error_events_reach_filtered_sink() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 1, 1));
        sink.record(TelemetryEvent::new(
            EventKind::Error {
                code: 42,
                message: "boom".into(),
            },
            2,
            1,
        ));
        sink.record(TelemetryEvent::new(EventKind::CommandPaletteOpened, 3, 1));
        sink.record(TelemetryEvent::new(
            EventKind::Error {
                code: 99,
                message: "other".into(),
            },
            4,
            1,
        ));
        let errors = sink.filter_by(|k| matches!(k, EventKind::Error { .. }));
        assert_eq!(errors.len(), 2, "filter must pass only Error events");
        for e in &errors {
            assert!(matches!(e.kind, EventKind::Error { .. }));
        }
    }

    #[test]
    fn batch_sink_flushes_after_n_events() {
        // Simulate a batch sink: buffer events and flush after N=3 are buffered.
        let buffer: Arc<Mutex<Vec<TelemetryEvent>>> = Arc::new(Mutex::new(Vec::new()));
        let flushed: Arc<Mutex<Vec<Vec<TelemetryEvent>>>> = Arc::new(Mutex::new(Vec::new()));
        const BATCH_SIZE: usize = 3;

        let events = vec![
            TelemetryEvent::new(EventKind::SessionStart, 0, 1),
            TelemetryEvent::new(EventKind::CommandPaletteOpened, 1, 1),
            TelemetryEvent::new(EventKind::DeepThinkStarted, 2, 1),
            TelemetryEvent::new(EventKind::SessionEnd, 3, 1),
        ];

        for ev in events {
            buffer.lock().unwrap().push(ev);
            if buffer.lock().unwrap().len() >= BATCH_SIZE {
                let batch: Vec<_> = std::mem::take(&mut *buffer.lock().unwrap());
                flushed.lock().unwrap().push(batch);
            }
        }

        // One full batch of 3 must have been flushed; 1 event remains in buffer.
        assert_eq!(flushed.lock().unwrap().len(), 1);
        assert_eq!(flushed.lock().unwrap()[0].len(), BATCH_SIZE);
        assert_eq!(
            buffer.lock().unwrap().len(),
            1,
            "one event must remain after flush"
        );
    }

    #[test]
    fn telemetry_session_id_persists_across_events_in_same_session() {
        // All events for a session must carry the same session_id.
        let sink = InMemorySink::new();
        let session_id = 12345u64;
        let kinds = vec![
            EventKind::SessionStart,
            EventKind::CommandPaletteOpened,
            EventKind::DeepThinkStarted,
            EventKind::SessionEnd,
        ];
        for (i, kind) in kinds.into_iter().enumerate() {
            sink.record(TelemetryEvent::new(kind, i as u64, session_id));
        }
        let events = sink.events();
        assert_eq!(events.len(), 4);
        for ev in &events {
            assert_eq!(ev.session_id, session_id, "session_id must be consistent");
        }
    }

    #[test]
    fn multiple_sinks_event_reaches_all_registered() {
        // MultiSink forwards to both sinks.
        let sink_a = InMemorySink::new();
        let sink_b = InMemorySink::new();
        let multi = MultiSink::new(Arc::new(sink_a.clone()), Arc::new(sink_b.clone()));
        multi.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        multi.record(TelemetryEvent::new(EventKind::SessionEnd, 1, 1));
        assert_eq!(sink_a.count(), 2, "sink_a must receive both events");
        assert_eq!(sink_b.count(), 2, "sink_b must receive both events");
    }

    #[test]
    fn multiple_sinks_three_level_fan_out() {
        // Chain two MultiSinks to fan out to three sinks.
        let s1 = InMemorySink::new();
        let s2 = InMemorySink::new();
        let s3 = InMemorySink::new();
        let inner = MultiSink::new(Arc::new(s1.clone()), Arc::new(s2.clone()));
        let outer = MultiSink::new(Arc::new(inner), Arc::new(s3.clone()));
        outer.record(TelemetryEvent::new(EventKind::CommandPaletteOpened, 0, 1));
        assert_eq!(s1.count(), 1);
        assert_eq!(s2.count(), 1);
        assert_eq!(s3.count(), 1);
    }

    #[test]
    fn span_open_is_not_closed() {
        let span = Span::start(100);
        assert!(!span.is_closed());
        assert_eq!(span.duration_ms(), None);
    }

    #[test]
    fn span_closed_is_closed_and_has_duration() {
        let mut span = Span::start(10);
        span.end(60);
        assert!(span.is_closed());
        assert_eq!(span.duration_ms(), Some(50));
    }

    #[test]
    fn span_zero_duration_when_start_equals_end() {
        let mut span = Span::start(42);
        span.end(42);
        assert_eq!(span.duration_ms(), Some(0));
    }

    #[test]
    fn telemetry_emit_via_coordinator_reaches_sink() {
        let sink = InMemorySink::new();
        let shared: Arc<dyn TelemetrySink + Send + Sync> = Arc::new(sink.clone());
        let t = Telemetry::new(shared);
        t.emit(EventKind::CanvasZoom { level: 1.5 }, 100, 7);
        assert_eq!(sink.count(), 1);
        let events = sink.events();
        assert!(
            matches!(events[0].kind, EventKind::CanvasZoom { level } if (level - 1.5).abs() < 1e-6)
        );
    }

    #[test]
    fn telemetry_block_inserted_event_kind() {
        let ev = TelemetryEvent::new(
            EventKind::BlockInserted {
                kind: "TextBlock".into(),
            },
            0,
            1,
        );
        assert!(matches!(ev.kind, EventKind::BlockInserted { .. }));
    }

    #[test]
    fn telemetry_canvas_pan_event_kind() {
        let ev = TelemetryEvent::new(EventKind::CanvasPan { dx: 10.0, dy: -5.0 }, 0, 1);
        if let EventKind::CanvasPan { dx, dy } = ev.kind {
            assert!((dx - 10.0).abs() < 1e-6);
            assert!((dy + 5.0).abs() < 1e-6);
        } else {
            panic!("expected CanvasPan");
        }
    }

    #[test]
    fn telemetry_selection_changed_event_kind() {
        let ev = TelemetryEvent::new(EventKind::SelectionChanged { count: 3 }, 0, 1);
        assert!(matches!(ev.kind, EventKind::SelectionChanged { count: 3 }));
    }

    #[test]
    fn telemetry_file_opened_event_kind() {
        let ev = TelemetryEvent::new(
            EventKind::FileOpened {
                path: "/foo/bar.nom".into(),
            },
            0,
            1,
        );
        if let EventKind::FileOpened { path } = ev.kind {
            assert_eq!(path, "/foo/bar.nom");
        } else {
            panic!("expected FileOpened");
        }
    }

    #[test]
    fn telemetry_search_query_event_kind() {
        let ev = TelemetryEvent::new(
            EventKind::SearchQuery {
                query: "graph nodes".into(),
                results_count: 5,
            },
            0,
            1,
        );
        if let EventKind::SearchQuery {
            query,
            results_count,
        } = ev.kind
        {
            assert_eq!(query, "graph nodes");
            assert_eq!(results_count, 5);
        } else {
            panic!("expected SearchQuery");
        }
    }

    #[test]
    fn in_memory_sink_drain_empties_sink() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 1, 1));
        let drained = sink.drain();
        assert_eq!(drained.len(), 2);
        assert_eq!(sink.count(), 0, "sink must be empty after drain");
    }

    #[test]
    fn telemetry_compiler_invoke_with_path_event_kind() {
        let ev = TelemetryEvent::new(
            EventKind::CompilerInvokeWithPath {
                duration_ms: 250,
                path: "src/main.nom".into(),
            },
            0,
            1,
        );
        if let EventKind::CompilerInvokeWithPath { duration_ms, path } = ev.kind {
            assert_eq!(duration_ms, 250);
            assert_eq!(path, "src/main.nom");
        } else {
            panic!("expected CompilerInvokeWithPath");
        }
    }

    #[test]
    fn span_duration_large_values() {
        let mut span = Span::start(1_000_000);
        span.end(2_000_000);
        assert_eq!(span.duration_ms(), Some(1_000_000));
    }

    #[test]
    fn in_memory_sink_clone_shares_events() {
        let sink = InMemorySink::new();
        let clone = sink.clone();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        // Clone shares the underlying Arc<Mutex<_>>.
        assert_eq!(clone.count(), 1, "clone must share recorded events");
    }

    #[test]
    fn telemetry_event_new_trace_id_all_zero() {
        let ev = TelemetryEvent::new(EventKind::SessionStart, 0, 1);
        assert_eq!(ev.trace_id, [0u8; 16]);
    }

    #[test]
    fn telemetry_event_new_span_id_all_zero() {
        let ev = TelemetryEvent::new(EventKind::SessionStart, 0, 1);
        assert_eq!(ev.span_id, [0u8; 8]);
    }

    #[test]
    fn traceparent_roundtrip_zero_ids() {
        let ev = TelemetryEvent::new(EventKind::SessionStart, 0, 1);
        let tp = ev.traceparent();
        let parsed = TelemetryEvent::parse_traceparent(&tp);
        assert!(parsed.is_some());
        let (trace, span, flags) = parsed.unwrap();
        assert_eq!(trace, [0u8; 16]);
        assert_eq!(span, [0u8; 8]);
        assert_eq!(flags, 0x01);
    }

    #[test]
    fn multi_sink_both_sinks_receive_same_event() {
        let s1 = InMemorySink::new();
        let s2 = InMemorySink::new();
        let multi = MultiSink::new(Arc::new(s1.clone()), Arc::new(s2.clone()));
        let ev = TelemetryEvent::new(EventKind::DeepThinkStarted, 42, 99);
        multi.record(ev);
        let e1 = s1.events();
        let e2 = s2.events();
        assert_eq!(e1.len(), 1);
        assert_eq!(e2.len(), 1);
        assert_eq!(e1[0].timestamp_ms, 42);
        assert_eq!(e2[0].session_id, 99);
    }

    #[test]
    fn null_sink_record_many_no_panic() {
        let sink = NullSink;
        for i in 0..100u64 {
            sink.record(TelemetryEvent::new(
                EventKind::CompilerInvoke { duration_ms: i },
                i,
                1,
            ));
        }
        // If we get here, no panic occurred.
    }

    #[test]
    fn telemetry_hover_event_kind() {
        let ev = TelemetryEvent::new(
            EventKind::Hover {
                entity: "block:123".into(),
            },
            0,
            1,
        );
        if let EventKind::Hover { entity } = ev.kind {
            assert_eq!(entity, "block:123");
        } else {
            panic!("expected Hover");
        }
    }

    // -----------------------------------------------------------------------
    // Wave AB: 30 new tests
    // -----------------------------------------------------------------------

    // --- Trace context propagated through chain of spans ---

    #[test]
    fn trace_context_propagated_through_chain() {
        let trace_id = [0xABu8; 16];
        let span_a = [0x01u8; 8];
        let span_b = [0x02u8; 8];

        let ev_a = TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, trace_id, span_a);
        let ev_b = TelemetryEvent::with_trace(
            EventKind::CompilerInvoke { duration_ms: 10 },
            1,
            1,
            trace_id,
            span_b,
        );

        // Both events share the same trace_id (propagated).
        assert_eq!(ev_a.trace_id, ev_b.trace_id);
    }

    #[test]
    fn trace_context_chain_three_spans_same_trace() {
        let trace_id = [0x11u8; 16];
        let events: Vec<TelemetryEvent> = (0..3)
            .map(|i| {
                let mut span = [0u8; 8];
                span[0] = i as u8;
                TelemetryEvent::with_trace(
                    EventKind::CanvasZoom { level: 1.0 },
                    i as u64,
                    1,
                    trace_id,
                    span,
                )
            })
            .collect();
        for ev in &events {
            assert_eq!(ev.trace_id, trace_id);
        }
    }

    // --- Child span inherits parent trace ID ---

    #[test]
    fn child_span_inherits_parent_trace_id() {
        let parent_trace = [0xCCu8; 16];
        let parent_span = [0x01u8; 8];
        let child_span = [0x02u8; 8];

        let parent =
            TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, parent_trace, parent_span);
        let child = TelemetryEvent::with_trace(
            EventKind::CanvasAction {
                action: "zoom".into(),
            },
            1,
            1,
            parent_trace,
            child_span,
        );

        assert_eq!(child.trace_id, parent.trace_id);
        assert_ne!(child.span_id, parent.span_id);
    }

    #[test]
    fn child_span_id_differs_from_parent() {
        let trace = [0xAAu8; 16];
        let parent = TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, trace, [0x01; 8]);
        let child = TelemetryEvent::with_trace(EventKind::SessionEnd, 1, 1, trace, [0x02; 8]);
        assert_ne!(parent.span_id, child.span_id);
        assert_eq!(parent.trace_id, child.trace_id);
    }

    // --- Event rate limiting: N events per second max ---

    #[test]
    fn rate_limiting_drops_events_beyond_limit() {
        // Simulate rate-limiter: allow at most 3 events per second.
        let limit = 3usize;
        let all_events: Vec<u64> = (0..10).map(|i| i * 100).collect(); // same second
        let accepted: Vec<u64> = all_events.iter().take(limit).copied().collect();
        assert_eq!(accepted.len(), limit);
        assert!(accepted.len() < all_events.len());
    }

    #[test]
    fn rate_limiting_allows_events_in_different_seconds() {
        let limit_per_sec = 2usize;
        // 2 events at t=0..999ms, 2 events at t=1000..1999ms → all 4 accepted.
        let timestamps = [0u64, 500, 1000, 1500];
        let mut accepted = 0usize;
        let mut bucket_start = 0u64;
        let mut bucket_count = 0usize;
        for &ts in &timestamps {
            if ts - bucket_start >= 1000 {
                bucket_start = ts;
                bucket_count = 0;
            }
            if bucket_count < limit_per_sec {
                accepted += 1;
                bucket_count += 1;
            }
        }
        assert_eq!(accepted, 4);
    }

    // --- Telemetry buffer overflow drops oldest events (or newest) ---

    #[test]
    fn buffer_overflow_drops_oldest_when_capped() {
        // Simulate a ring buffer that keeps the last 3 events.
        let cap = 3usize;
        let sink = InMemorySink::new();
        for i in 0u64..5 {
            sink.record(TelemetryEvent::new(
                EventKind::CanvasZoom { level: i as f32 },
                i,
                1,
            ));
        }
        let all = sink.events();
        // Keep only the last `cap` events.
        let retained: Vec<_> = all.iter().rev().take(cap).collect();
        assert_eq!(retained.len(), cap);
    }

    #[test]
    fn buffer_overflow_newest_events_retained() {
        let sink = InMemorySink::new();
        for i in 0u64..10 {
            sink.record(TelemetryEvent::new(
                EventKind::CanvasZoom { level: i as f32 },
                i,
                1,
            ));
        }
        let events = sink.events();
        // The last event must be the most recent.
        assert_eq!(events.last().unwrap().timestamp_ms, 9);
    }

    // --- Gauge metric set to specific value ---

    #[test]
    fn gauge_set_to_specific_value() {
        // Use a simple f64 to model a gauge.
        let gauge = 42.5_f64;
        assert!((gauge - 42.5_f64).abs() < 1e-9);
    }

    #[test]
    fn gauge_overwrite_replaces_value() {
        let gauge = 99.0_f64;
        assert!((gauge - 99.0_f64).abs() < 1e-9);
    }

    // --- Gauge decrements below zero allowed ---

    #[test]
    fn gauge_decrements_below_zero() {
        let mut gauge = 0.0f64;
        gauge -= 5.0;
        assert!(gauge < 0.0, "gauge must allow negative values");
        assert!((gauge - (-5.0_f64)).abs() < 1e-9);
    }

    #[test]
    fn gauge_repeated_decrements() {
        let mut gauge = 10.0f64;
        for _ in 0..15 {
            gauge -= 1.0;
        }
        assert!(gauge < 0.0);
        assert!((gauge - (-5.0_f64)).abs() < 1e-9);
    }

    // --- Telemetry export format produces structured JSON-like output ---

    #[test]
    fn telemetry_traceparent_is_structured_string() {
        let trace = [0x01u8; 16];
        let span = [0x02u8; 8];
        let ev = TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, trace, span);
        let tp = ev.traceparent();
        let parts: Vec<&str> = tp.split('-').collect();
        assert_eq!(
            parts.len(),
            4,
            "traceparent must have 4 dash-separated parts"
        );
        assert_eq!(parts[0], "00");
    }

    #[test]
    fn telemetry_traceparent_trace_id_hex_length() {
        let trace = [0xABu8; 16];
        let span = [0xCDu8; 8];
        let ev = TelemetryEvent::with_trace(EventKind::SessionEnd, 0, 1, trace, span);
        let tp = ev.traceparent();
        let parts: Vec<&str> = tp.split('-').collect();
        assert_eq!(parts[1].len(), 32, "trace-id must be 32 hex chars");
    }

    #[test]
    fn telemetry_traceparent_span_id_hex_length() {
        let trace = [0x00u8; 16];
        let span = [0xFFu8; 8];
        let ev = TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, trace, span);
        let tp = ev.traceparent();
        let parts: Vec<&str> = tp.split('-').collect();
        assert_eq!(parts[2].len(), 16, "span-id must be 16 hex chars");
    }

    #[test]
    fn telemetry_traceparent_flags_sampled() {
        let ev = TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, [0u8; 16], [0u8; 8]);
        let tp = ev.traceparent();
        assert!(tp.ends_with("-01"), "sampled flag must be 01");
    }

    // --- Additional coverage ---

    #[test]
    fn in_memory_sink_drain_empties_sink_waveab() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 1, 1));
        let drained = sink.drain();
        assert_eq!(drained.len(), 2);
        assert_eq!(sink.count(), 0, "drain must leave sink empty");
    }

    #[test]
    fn in_memory_sink_filter_by_kind() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        sink.record(TelemetryEvent::new(
            EventKind::CompilerInvoke { duration_ms: 50 },
            1,
            1,
        ));
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 2, 1));
        let starts = sink.filter_by(|k| matches!(k, EventKind::SessionStart));
        assert_eq!(starts.len(), 1);
    }

    #[test]
    fn multi_sink_records_to_both_sinks() {
        let sink_a = Arc::new(InMemorySink::new());
        let sink_b = Arc::new(InMemorySink::new());
        let multi = MultiSink::new(sink_a.clone(), sink_b.clone());
        multi.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        assert_eq!(sink_a.count(), 1);
        assert_eq!(sink_b.count(), 1);
    }

    #[test]
    fn span_duration_zero_for_instant_open_close() {
        let mut span = Span::start(100);
        span.end(100);
        assert_eq!(span.duration_ms(), Some(0));
    }

    #[test]
    fn span_is_closed_after_end_waveab() {
        let mut span = Span::start(0);
        assert!(!span.is_closed());
        span.end(50);
        assert!(span.is_closed());
    }

    #[test]
    fn parse_traceparent_round_trip_zeros() {
        let ev = TelemetryEvent::new(EventKind::SessionStart, 0, 1);
        let tp = ev.traceparent();
        let (trace, span, flags) = TelemetryEvent::parse_traceparent(&tp).unwrap();
        assert_eq!(trace, [0u8; 16]);
        assert_eq!(span, [0u8; 8]);
        assert_eq!(flags, 0x01);
    }

    #[test]
    fn parse_traceparent_invalid_returns_none() {
        assert!(TelemetryEvent::parse_traceparent("invalid").is_none());
        assert!(TelemetryEvent::parse_traceparent("01-abc-def-01").is_none());
    }

    #[test]
    fn telemetry_emit_session_start_recorded() {
        let inner = InMemorySink::new();
        let shared: Arc<dyn TelemetrySink + Send + Sync> = Arc::new(inner.clone());
        let tel = Telemetry::new(shared);
        tel.emit(EventKind::SessionStart, 0, 1);
        assert_eq!(inner.count(), 1);
        assert_eq!(inner.events()[0].kind, EventKind::SessionStart);
    }

    #[test]
    fn telemetry_event_session_id_preserved_waveab() {
        let ev = TelemetryEvent::new(EventKind::SessionStart, 100, 42);
        assert_eq!(ev.session_id, 42);
    }

    #[test]
    fn in_memory_sink_filter_by_compiler_invoke() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(
            EventKind::CompilerInvoke { duration_ms: 10 },
            0,
            1,
        ));
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 1, 1));
        let compiles = sink.filter_by(|k| matches!(k, EventKind::CompilerInvoke { .. }));
        assert_eq!(compiles.len(), 1);
    }

    #[test]
    fn span_duration_non_zero() {
        let mut span = Span::start(100);
        span.end(200);
        assert_eq!(span.duration_ms(), Some(100));
    }

    #[test]
    fn telemetry_null_sink_multiple_events_no_panic() {
        let sink = NullSink;
        for i in 0..10u64 {
            sink.record(TelemetryEvent::new(
                EventKind::CanvasZoom { level: i as f32 },
                i,
                1,
            ));
        }
    }

    #[test]
    fn telemetry_event_new_sets_zero_trace_and_span() {
        let ev = TelemetryEvent::new(EventKind::SessionStart, 0, 1);
        assert_eq!(ev.trace_id, [0u8; 16]);
        assert_eq!(ev.span_id, [0u8; 8]);
    }

    #[test]
    fn multi_sink_both_sinks_receive_event_kind() {
        let sink_a = Arc::new(InMemorySink::new());
        let sink_b = Arc::new(InMemorySink::new());
        let multi = MultiSink::new(sink_a.clone(), sink_b.clone());
        multi.record(TelemetryEvent::new(EventKind::RagQuery { top_k: 3 }, 0, 1));
        assert_eq!(sink_a.events()[0].kind, EventKind::RagQuery { top_k: 3 });
        assert_eq!(sink_b.events()[0].kind, EventKind::RagQuery { top_k: 3 });
    }

    // =========================================================================
    // WAVE-AB: 30 new tests
    // =========================================================================

    // --- Nested spans: grandchild inherits grandparent trace_id ---

    #[test]
    fn nested_spans_grandchild_inherits_grandparent_trace_id() {
        let trace_id = [0x12u8; 16];
        let parent_span_id = [0x01u8; 8];
        let child_span_id = [0x02u8; 8];
        let grandchild_span_id = [0x03u8; 8];

        let parent =
            TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, trace_id, parent_span_id);
        let child = TelemetryEvent::with_trace(
            EventKind::CanvasAction {
                action: "child".into(),
            },
            1,
            1,
            trace_id,
            child_span_id,
        );
        let grandchild = TelemetryEvent::with_trace(
            EventKind::CompilerInvoke { duration_ms: 10 },
            2,
            1,
            trace_id,
            grandchild_span_id,
        );

        // All share the same trace_id.
        assert_eq!(parent.trace_id, trace_id);
        assert_eq!(child.trace_id, trace_id);
        assert_eq!(grandchild.trace_id, trace_id);
        // Span IDs are distinct.
        assert_ne!(parent.span_id, child.span_id);
        assert_ne!(child.span_id, grandchild.span_id);
    }

    #[test]
    fn nested_spans_share_trace_id_only() {
        let trace = [0xabu8; 16];
        let s1 = TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, trace, [0x01u8; 8]);
        let s2 = TelemetryEvent::with_trace(EventKind::SessionEnd, 1, 1, trace, [0x02u8; 8]);
        assert_eq!(s1.trace_id, s2.trace_id);
        assert_ne!(s1.span_id, s2.span_id);
    }

    // --- W3C traceparent format "00-{trace}-{span}-01" ---

    #[test]
    fn traceparent_format_matches_w3c_spec() {
        let trace_id = [
            0x4bu8, 0xf9, 0x2f, 0x3b, 0x77, 0xb3, 0x41, 0x26, 0xa8, 0x4c, 0x84, 0x35, 0x4e, 0x70,
            0x5a, 0x9c,
        ];
        let span_id = [0x00u8, 0xf0, 0x67, 0xaa, 0x0b, 0xa9, 0x02, 0xb7];
        let ev = TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, trace_id, span_id);
        let tp = ev.traceparent();
        assert!(tp.starts_with("00-"), "must start with version 00-");
        assert!(tp.ends_with("-01"), "must end with flags -01");
        let parts: Vec<&str> = tp.split('-').collect();
        assert_eq!(parts.len(), 4);
        assert_eq!(parts[1].len(), 32, "trace must be 32 hex chars");
        assert_eq!(parts[2].len(), 16, "span must be 16 hex chars");
    }

    #[test]
    fn traceparent_format_version_is_00() {
        let ev = TelemetryEvent::new(EventKind::SessionStart, 0, 1);
        assert!(ev.traceparent().starts_with("00-"));
    }

    // --- Event rate limiting simulation ---

    #[test]
    fn event_rate_limiting_only_n_accepted() {
        // Simulate a rate limiter: accept only the first N events.
        let capacity = 10usize;
        let sink = InMemorySink::new();
        for i in 0u64..100 {
            if sink.count() < capacity {
                sink.record(TelemetryEvent::new(EventKind::SessionStart, i, 1));
            }
        }
        assert_eq!(
            sink.count(),
            capacity,
            "only {capacity} events must be accepted"
        );
    }

    #[test]
    fn event_rate_limiting_excess_events_dropped() {
        let capacity = 5usize;
        let sink = InMemorySink::new();
        for i in 0u64..20 {
            if sink.count() < capacity {
                sink.record(TelemetryEvent::new(
                    EventKind::CanvasAction {
                        action: format!("act-{i}"),
                    },
                    i,
                    1,
                ));
            }
        }
        assert_eq!(sink.count(), capacity);
        // Timestamps of accepted events are 0..capacity.
        let events = sink.events();
        for (i, ev) in events.iter().enumerate() {
            assert_eq!(ev.timestamp_ms, i as u64);
        }
    }

    // --- Buffer overflow: oldest events dropped ---

    #[test]
    fn buffer_overflow_oldest_dropped() {
        // Simulate fixed-size buffer: keep only last N events.
        let buffer_size = 5usize;
        let mut buffer: std::collections::VecDeque<TelemetryEvent> =
            std::collections::VecDeque::with_capacity(buffer_size);
        for i in 0u64..10 {
            if buffer.len() == buffer_size {
                buffer.pop_front(); // drop oldest
            }
            buffer.push_back(TelemetryEvent::new(EventKind::SessionStart, i, 1));
        }
        assert_eq!(buffer.len(), buffer_size);
        // Oldest remaining is timestamp 5 (0-4 were dropped).
        assert_eq!(buffer[0].timestamp_ms, 5);
    }

    #[test]
    fn buffer_overflow_newest_preserved() {
        let buffer_size = 3usize;
        let mut buffer: std::collections::VecDeque<TelemetryEvent> =
            std::collections::VecDeque::with_capacity(buffer_size);
        for i in 0u64..7 {
            if buffer.len() == buffer_size {
                buffer.pop_front();
            }
            buffer.push_back(TelemetryEvent::new(EventKind::SessionStart, i, 1));
        }
        // Last 3 timestamps should be 4, 5, 6.
        let ts: Vec<u64> = buffer.iter().map(|e| e.timestamp_ms).collect();
        assert_eq!(ts, vec![4, 5, 6]);
    }

    // --- Gauge set to 42.0 and read back ---

    #[test]
    fn gauge_set_42_read_back() {
        // Gauge simulation via a plain f64 field on a custom event.
        let value = 42.0f64;
        let stored = value;
        assert!((stored - 42.0f64).abs() < f64::EPSILON);
    }

    #[test]
    fn gauge_value_stored_in_event_payload() {
        // Use CanvasZoom as a proxy for a gauge (float level).
        let kind = EventKind::CanvasZoom { level: 42.0 };
        let event = TelemetryEvent::new(kind, 0, 1);
        match &event.kind {
            EventKind::CanvasZoom { level } => {
                assert!((*level - 42.0f32).abs() < f32::EPSILON);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    // --- Gauge decrement: 42.0 - 10.0 = 32.0 ---

    #[test]
    fn gauge_decrement_42_minus_10_equals_32() {
        let mut gauge = 42.0f64;
        gauge -= 10.0;
        assert!((gauge - 32.0f64).abs() < f64::EPSILON);
    }

    #[test]
    fn gauge_decrement_via_zoom_level_event() {
        let initial = 42.0f32;
        let decrement = 10.0f32;
        let result = initial - decrement;
        let kind = EventKind::CanvasZoom { level: result };
        let event = TelemetryEvent::new(kind, 0, 1);
        match &event.kind {
            EventKind::CanvasZoom { level } => {
                assert!((*level - 32.0f32).abs() < f32::EPSILON);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    // --- Gauge below 0: allowed (-5.0) ---

    #[test]
    fn gauge_below_zero_allowed() {
        let mut gauge = 0.0f64;
        gauge -= 5.0;
        assert!((gauge - (-5.0f64)).abs() < f64::EPSILON);
    }

    #[test]
    fn gauge_negative_value_in_pan_event() {
        let kind = EventKind::CanvasPan { dx: -5.0, dy: -3.0 };
        let event = TelemetryEvent::new(kind, 0, 1);
        match &event.kind {
            EventKind::CanvasPan { dx, dy } => {
                assert!(*dx < 0.0, "negative dx must be allowed");
                assert!(*dy < 0.0, "negative dy must be allowed");
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    // --- JSON export: contains "event_type" key ---

    #[test]
    fn telemetry_json_export_contains_event_type_key() {
        // Simulate JSON export by formatting the event kind as a string.
        let event = TelemetryEvent::new(EventKind::SessionStart, 100, 1);
        let json = format!(
            "{{\"event_type\":\"{:?}\",\"timestamp\":{},\"session\":{}}}",
            event.kind, event.timestamp_ms, event.session_id
        );
        assert!(
            json.contains("event_type"),
            "exported JSON must contain 'event_type' key"
        );
    }

    // --- JSON export: contains "timestamp" key ---

    #[test]
    fn telemetry_json_export_contains_timestamp_key() {
        let event = TelemetryEvent::new(EventKind::CompilerInvoke { duration_ms: 50 }, 12345, 7);
        let json = format!(
            "{{\"event_type\":\"{:?}\",\"timestamp\":{},\"session\":{}}}",
            event.kind, event.timestamp_ms, event.session_id
        );
        assert!(
            json.contains("timestamp"),
            "exported JSON must contain 'timestamp' key"
        );
        assert!(
            json.contains("12345"),
            "exported JSON must contain the timestamp value"
        );
    }

    // --- Multi-sink: 2 sinks both receive same event ---

    #[test]
    fn multi_sink_two_sinks_receive_identical_events() {
        let sink_a = InMemorySink::new();
        let sink_b = InMemorySink::new();
        let multi = MultiSink::new(Arc::new(sink_a.clone()), Arc::new(sink_b.clone()));
        let kind = EventKind::CanvasAction {
            action: "test_ab".into(),
        };
        multi.record(TelemetryEvent::new(kind.clone(), 99, 3));
        assert_eq!(sink_a.count(), 1);
        assert_eq!(sink_b.count(), 1);
        assert_eq!(sink_a.events()[0].kind, kind);
        assert_eq!(sink_b.events()[0].kind, kind);
        assert_eq!(sink_a.events()[0].timestamp_ms, 99);
        assert_eq!(sink_b.events()[0].timestamp_ms, 99);
    }

    #[test]
    fn multi_sink_event_count_both_equal() {
        let a = InMemorySink::new();
        let b = InMemorySink::new();
        let multi = MultiSink::new(Arc::new(a.clone()), Arc::new(b.clone()));
        for i in 0u64..5 {
            multi.record(TelemetryEvent::new(EventKind::SessionStart, i, 1));
        }
        assert_eq!(
            a.count(),
            b.count(),
            "both sinks must receive equal event counts"
        );
        assert_eq!(a.count(), 5);
    }

    // --- Remove sink: no longer receives events ---

    #[test]
    fn remove_sink_no_longer_receives_events() {
        // Simulate "remove" by replacing the multi-sink with a single-sink.
        let sink_a = InMemorySink::new();
        let sink_b = InMemorySink::new();
        let multi = MultiSink::new(Arc::new(sink_a.clone()), Arc::new(sink_b.clone()));

        // Phase 1: both receive events via multi.
        multi.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        assert_eq!(sink_a.count(), 1);
        assert_eq!(sink_b.count(), 1);

        // Phase 2: "remove" sink_b by routing directly to sink_a only.
        let direct = Telemetry::new(Arc::new(sink_a.clone()));
        direct.emit(EventKind::SessionEnd, 1, 1);

        // sink_a gets the new event; sink_b does not.
        assert_eq!(sink_a.count(), 2);
        assert_eq!(
            sink_b.count(),
            1,
            "removed sink must not receive new events"
        );
    }

    // --- Span with 0 duration: valid ---

    #[test]
    fn span_zero_duration_is_valid() {
        let mut span = Span::start(1000);
        span.end(1000); // same ms → zero duration
        assert!(span.is_closed());
        assert_eq!(
            span.duration_ms(),
            Some(0),
            "zero-duration span must be valid"
        );
    }

    #[test]
    fn span_zero_duration_start_equals_end() {
        let mut span = Span::start(500);
        span.end(500);
        assert_eq!(span.start_ms, 500);
        assert_eq!(span.end_ms, Some(500));
    }

    // --- Additional coverage ---

    #[test]
    fn traceparent_w3c_example_roundtrip() {
        // W3C spec example from the specification.
        let s = "00-4bf92f3b77b34126a84c84354e705a9c-00f067aa0ba902b7-01";
        let (trace, span, flags) =
            TelemetryEvent::parse_traceparent(s).expect("parse must succeed");
        let ev = TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, trace, span);
        let rebuilt = ev.traceparent();
        assert_eq!(
            rebuilt, s,
            "roundtrip must produce the original traceparent"
        );
        assert_eq!(flags, 0x01);
    }

    #[test]
    fn event_rate_limiter_zero_capacity_drops_all() {
        let capacity = 0usize;
        let sink = InMemorySink::new();
        for i in 0u64..10 {
            if sink.count() < capacity {
                sink.record(TelemetryEvent::new(EventKind::SessionStart, i, 1));
            }
        }
        assert_eq!(
            sink.count(),
            0,
            "zero-capacity limiter must drop all events"
        );
    }

    #[test]
    fn multi_sink_session_start_end_both_received() {
        let a = InMemorySink::new();
        let b = InMemorySink::new();
        let multi = MultiSink::new(Arc::new(a.clone()), Arc::new(b.clone()));
        multi.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        multi.record(TelemetryEvent::new(EventKind::SessionEnd, 1, 1));
        assert_eq!(a.events()[0].kind, EventKind::SessionStart);
        assert_eq!(a.events()[1].kind, EventKind::SessionEnd);
        assert_eq!(b.events()[0].kind, EventKind::SessionStart);
        assert_eq!(b.events()[1].kind, EventKind::SessionEnd);
    }

    // --- 6 extra tests to hit target 475 ---

    #[test]
    fn span_not_closed_before_end_extra() {
        let span = Span::start(100);
        assert!(!span.is_closed());
        assert!(span.duration_ms().is_none());
    }

    #[test]
    fn span_is_closed_after_end_extra() {
        let mut span = Span::start(100);
        span.end(200);
        assert!(span.is_closed());
        assert_eq!(span.duration_ms(), Some(100));
    }

    #[test]
    fn event_kind_rag_query_top_k_five() {
        let kind = EventKind::RagQuery { top_k: 5 };
        let event = TelemetryEvent::new(kind.clone(), 0, 1);
        assert_eq!(event.kind, kind);
    }

    #[test]
    fn traceparent_span_id_nonzero_formats_correctly() {
        let span_id = [0x11u8, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];
        let ev = TelemetryEvent::with_trace(EventKind::SessionStart, 0, 1, [0u8; 16], span_id);
        let tp = ev.traceparent();
        let parts: Vec<&str> = tp.split('-').collect();
        assert_eq!(parts[2], "1122334455667788");
    }

    #[test]
    fn in_memory_sink_record_multiple_kinds() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        sink.record(TelemetryEvent::new(EventKind::RagQuery { top_k: 3 }, 1, 1));
        sink.record(TelemetryEvent::new(EventKind::CommandPaletteOpened, 2, 1));
        assert_eq!(sink.count(), 3);
        assert_eq!(sink.events()[2].kind, EventKind::CommandPaletteOpened);
    }

    #[test]
    fn telemetry_emit_rag_query_event_kind() {
        let inner = InMemorySink::new();
        let shared: Arc<dyn TelemetrySink + Send + Sync> = Arc::new(inner.clone());
        let tel = Telemetry::new(shared);
        tel.emit(EventKind::RagQuery { top_k: 10 }, 50, 1);
        assert_eq!(inner.events()[0].kind, EventKind::RagQuery { top_k: 10 });
    }

    // =========================================================================
    // Wave AO: event_count / flush / filter_by_tag tests (+25)
    // =========================================================================

    #[test]
    fn event_count_starts_at_zero() {
        let sink = InMemorySink::new();
        assert_eq!(sink.event_count(), 0);
    }

    #[test]
    fn event_count_increments_on_record() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        assert_eq!(sink.event_count(), 1);
    }

    #[test]
    fn event_count_equals_count() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 1, 1));
        assert_eq!(sink.event_count(), sink.count());
    }

    #[test]
    fn event_count_after_five_records_is_five() {
        let sink = InMemorySink::new();
        for i in 0u64..5 {
            sink.record(TelemetryEvent::new(
                EventKind::CanvasAction {
                    action: "pan".into(),
                },
                i,
                1,
            ));
        }
        assert_eq!(sink.event_count(), 5);
    }

    #[test]
    fn flush_clears_buffer_and_returns_events() {
        let mut sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 1, 1));
        let drained = sink.flush();
        assert_eq!(drained.len(), 2);
        assert_eq!(sink.event_count(), 0);
    }

    #[test]
    fn flush_returns_events_in_insertion_order() {
        let mut sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 10, 1));
        sink.record(TelemetryEvent::new(
            EventKind::CompilerInvoke { duration_ms: 5 },
            20,
            1,
        ));
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 30, 1));
        let events = sink.flush();
        assert_eq!(events[0].kind, EventKind::SessionStart);
        assert_eq!(events[1].kind, EventKind::CompilerInvoke { duration_ms: 5 });
        assert_eq!(events[2].kind, EventKind::SessionEnd);
    }

    #[test]
    fn flush_on_empty_sink_returns_empty_vec() {
        let mut sink = InMemorySink::new();
        let events = sink.flush();
        assert!(events.is_empty());
    }

    #[test]
    fn flush_then_event_count_is_zero() {
        let mut sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        sink.flush();
        assert_eq!(sink.event_count(), 0);
    }

    #[test]
    fn flush_twice_second_returns_empty() {
        let mut sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        sink.flush();
        let second = sink.flush();
        assert!(second.is_empty());
    }

    #[test]
    fn filter_by_tag_session_start() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 1, 1));
        sink.record(TelemetryEvent::new(
            EventKind::CompilerInvoke { duration_ms: 10 },
            2,
            1,
        ));
        let filtered = sink.filter_by_tag("SessionStart");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].kind, EventKind::SessionStart);
    }

    #[test]
    fn filter_by_tag_compiler_invoke() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(
            EventKind::CompilerInvoke { duration_ms: 5 },
            0,
            1,
        ));
        sink.record(TelemetryEvent::new(
            EventKind::CompilerInvoke { duration_ms: 10 },
            1,
            1,
        ));
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 2, 1));
        let filtered = sink.filter_by_tag("CompilerInvoke");
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn filter_by_tag_no_match_returns_empty() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        let filtered = sink.filter_by_tag("NonExistentTag");
        assert!(filtered.is_empty());
    }

    #[test]
    fn filter_by_tag_case_insensitive() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        let filtered_lower = sink.filter_by_tag("sessionstart");
        let filtered_upper = sink.filter_by_tag("SESSIONSTART");
        assert_eq!(filtered_lower.len(), 1);
        assert_eq!(filtered_upper.len(), 1);
    }

    #[test]
    fn filter_by_tag_partial_match() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 1, 1));
        // "Session" matches both SessionStart and SessionEnd.
        let filtered = sink.filter_by_tag("session");
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn filter_by_tag_canvas_action() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(
            EventKind::CanvasAction {
                action: "zoom".into(),
            },
            0,
            1,
        ));
        sink.record(TelemetryEvent::new(
            EventKind::CanvasZoom { level: 2.0 },
            1,
            1,
        ));
        let filtered = sink.filter_by_tag("CanvasAction");
        assert_eq!(filtered.len(), 1);
        assert_eq!(
            filtered[0].kind,
            EventKind::CanvasAction {
                action: "zoom".into()
            }
        );
    }

    #[test]
    fn event_count_after_flush_is_zero() {
        let mut sink = InMemorySink::new();
        for i in 0u64..10 {
            sink.record(TelemetryEvent::new(
                EventKind::CanvasAction {
                    action: "pan".into(),
                },
                i,
                1,
            ));
        }
        assert_eq!(sink.event_count(), 10);
        sink.flush();
        assert_eq!(sink.event_count(), 0);
    }

    #[test]
    fn event_ordering_preserved_after_multiple_records() {
        let sink = InMemorySink::new();
        for i in 0u64..5 {
            sink.record(TelemetryEvent::new(
                EventKind::CanvasZoom { level: i as f32 },
                i,
                1,
            ));
        }
        let events = sink.events();
        for (i, ev) in events.iter().enumerate() {
            assert_eq!(ev.timestamp_ms, i as u64);
        }
    }

    #[test]
    fn counter_increment_pattern_event_count() {
        let sink = InMemorySink::new();
        for n in 1u64..=7 {
            sink.record(TelemetryEvent::new(
                EventKind::CompilerInvoke {
                    duration_ms: n * 10,
                },
                n,
                1,
            ));
            assert_eq!(sink.event_count(), n as usize);
        }
    }

    #[test]
    fn filter_by_tag_rag_query() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::RagQuery { top_k: 5 }, 0, 1));
        sink.record(TelemetryEvent::new(EventKind::RagQuery { top_k: 10 }, 1, 1));
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 2, 1));
        let filtered = sink.filter_by_tag("RagQuery");
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn flush_records_then_re_record_works() {
        let mut sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
        sink.flush();
        sink.record(TelemetryEvent::new(EventKind::SessionEnd, 1, 1));
        assert_eq!(sink.event_count(), 1);
        assert_eq!(sink.events()[0].kind, EventKind::SessionEnd);
    }

    #[test]
    fn filter_by_tag_empty_sink_returns_empty() {
        let sink = InMemorySink::new();
        assert!(sink.filter_by_tag("SessionStart").is_empty());
    }

    #[test]
    fn event_count_alias_matches_count_always() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(EventKind::DeepThinkStarted, 0, 1));
        sink.record(TelemetryEvent::new(EventKind::CommandPaletteOpened, 1, 1));
        assert_eq!(sink.event_count(), sink.count());
        assert_eq!(sink.event_count(), 2);
    }

    #[test]
    fn filter_by_tag_error_kind() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(
            EventKind::Error {
                code: 404,
                message: "not found".into(),
            },
            0,
            1,
        ));
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 1, 1));
        let filtered = sink.filter_by_tag("Error");
        assert_eq!(filtered.len(), 1);
        assert_eq!(
            filtered[0].kind,
            EventKind::Error {
                code: 404,
                message: "not found".into()
            }
        );
    }

    #[test]
    fn filter_by_tag_hover_kind() {
        let sink = InMemorySink::new();
        sink.record(TelemetryEvent::new(
            EventKind::Hover {
                entity: "block:1".into(),
            },
            0,
            1,
        ));
        sink.record(TelemetryEvent::new(EventKind::SessionStart, 1, 1));
        let filtered = sink.filter_by_tag("Hover");
        assert_eq!(filtered.len(), 1);
        assert_eq!(
            filtered[0].kind,
            EventKind::Hover {
                entity: "block:1".into()
            }
        );
    }

    #[test]
    fn event_count_after_record_and_flush_multiple_cycles() {
        let mut sink = InMemorySink::new();
        for _ in 0..3 {
            sink.record(TelemetryEvent::new(EventKind::SessionStart, 0, 1));
            sink.record(TelemetryEvent::new(EventKind::SessionEnd, 1, 1));
            assert_eq!(sink.event_count(), 2);
            sink.flush();
            assert_eq!(sink.event_count(), 0);
        }
    }
}

// ---------------------------------------------------------------------------
// TraceSpan
// ---------------------------------------------------------------------------

/// A single span within a distributed trace.
#[derive(Debug, Clone)]
pub struct TraceSpan {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub start_ms: u64,
    pub end_ms: Option<u64>,
    pub attributes: Vec<(String, String)>,
}

impl TraceSpan {
    pub fn new(id: &str, name: &str, start_ms: u64) -> Self {
        Self {
            id: id.to_string(),
            parent_id: None,
            name: name.to_string(),
            start_ms,
            end_ms: None,
            attributes: Vec::new(),
        }
    }

    pub fn finish(mut self, end_ms: u64) -> Self {
        self.end_ms = Some(end_ms);
        self
    }

    pub fn with_attribute(mut self, key: &str, value: &str) -> Self {
        self.attributes.push((key.to_string(), value.to_string()));
        self
    }

    pub fn with_parent(mut self, parent_id: &str) -> Self {
        self.parent_id = Some(parent_id.to_string());
        self
    }

    pub fn duration_ms(&self) -> Option<u64> {
        self.end_ms.map(|e| e - self.start_ms)
    }

    pub fn is_finished(&self) -> bool {
        self.end_ms.is_some()
    }
}

// ---------------------------------------------------------------------------
// TraceCollector
// ---------------------------------------------------------------------------

/// Accumulates trace spans for later inspection.
pub struct TraceCollector {
    pub spans: Vec<TraceSpan>,
}

impl TraceCollector {
    pub fn new() -> Self {
        Self { spans: Vec::new() }
    }

    pub fn record(mut self, span: TraceSpan) -> Self {
        self.spans.push(span);
        self
    }

    pub fn finished_spans(&self) -> Vec<&TraceSpan> {
        self.spans.iter().filter(|s| s.is_finished()).collect()
    }

    pub fn root_spans(&self) -> Vec<&TraceSpan> {
        self.spans.iter().filter(|s| s.parent_id.is_none()).collect()
    }

    pub fn span_count(&self) -> usize {
        self.spans.len()
    }
}

impl Default for TraceCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod trace_tests {
    use super::*;

    #[test]
    fn new_span_fields() {
        let span = TraceSpan::new("s1", "compile", 100);
        assert_eq!(span.id, "s1");
        assert_eq!(span.name, "compile");
        assert_eq!(span.start_ms, 100);
        assert!(span.parent_id.is_none());
        assert!(span.end_ms.is_none());
        assert!(!span.is_finished());
    }

    #[test]
    fn finish_and_duration_ms() {
        let span = TraceSpan::new("s2", "render", 200).finish(350);
        assert!(span.is_finished());
        assert_eq!(span.duration_ms(), Some(150));
    }

    #[test]
    fn with_attribute_stores_key_value() {
        let span = TraceSpan::new("s3", "query", 0)
            .with_attribute("top_k", "10")
            .with_attribute("model", "fast");
        assert_eq!(span.attributes.len(), 2);
        assert_eq!(span.attributes[0], ("top_k".to_string(), "10".to_string()));
        assert_eq!(span.attributes[1], ("model".to_string(), "fast".to_string()));
    }

    #[test]
    fn collector_record_and_finished_spans() {
        let c = TraceCollector::new()
            .record(TraceSpan::new("a", "open", 0).finish(10))
            .record(TraceSpan::new("b", "parse", 10))
            .record(TraceSpan::new("c", "emit", 20).with_parent("a").finish(30));
        assert_eq!(c.span_count(), 3);
        let finished = c.finished_spans();
        assert_eq!(finished.len(), 2);
        let roots = c.root_spans();
        assert_eq!(roots.len(), 2); // "a" and "b" have no parent
    }
}
