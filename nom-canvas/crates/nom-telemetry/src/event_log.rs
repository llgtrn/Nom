// ---------------------------------------------------------------------------
// EventKind
// ---------------------------------------------------------------------------

/// Categories for logged events.
#[derive(Debug, Clone, PartialEq)]
pub enum EventKind {
    UserAction,
    SystemEvent,
    ErrorEvent,
    DebugEvent,
}

impl EventKind {
    /// Returns `true` only for `UserAction`.
    pub fn is_user(&self) -> bool {
        matches!(self, EventKind::UserAction)
    }

    /// Numeric severity: UserAction=1, SystemEvent=2, ErrorEvent=4, DebugEvent=0.
    pub fn severity_level(&self) -> u8 {
        match self {
            EventKind::UserAction => 1,
            EventKind::SystemEvent => 2,
            EventKind::ErrorEvent => 4,
            EventKind::DebugEvent => 0,
        }
    }
}

// ---------------------------------------------------------------------------
// LoggedEvent
// ---------------------------------------------------------------------------

/// A single logged event with optional context.
#[derive(Debug, Clone, PartialEq)]
pub struct LoggedEvent {
    pub kind: EventKind,
    pub message: String,
    pub timestamp_ms: u64,
    pub context: Option<String>,
}

impl LoggedEvent {
    /// Returns `true` when `context` is `Some`.
    pub fn has_context(&self) -> bool {
        self.context.is_some()
    }

    /// Returns `true` when `kind` is `ErrorEvent`.
    pub fn is_error(&self) -> bool {
        self.kind == EventKind::ErrorEvent
    }
}

// ---------------------------------------------------------------------------
// EventLog
// ---------------------------------------------------------------------------

/// An ordered collection of `LoggedEvent` entries.
#[derive(Debug, Clone, Default)]
pub struct EventLog {
    pub events: Vec<LoggedEvent>,
}

impl EventLog {
    /// Append an event.
    pub fn push(&mut self, e: LoggedEvent) {
        self.events.push(e);
    }

    /// All events where `is_error()` is true.
    pub fn errors(&self) -> Vec<&LoggedEvent> {
        self.events.iter().filter(|e| e.is_error()).collect()
    }

    /// All events where `kind.is_user()` is true.
    pub fn user_actions(&self) -> Vec<&LoggedEvent> {
        self.events.iter().filter(|e| e.kind.is_user()).collect()
    }

    /// All events with `timestamp_ms >= threshold_ms`.
    pub fn since(&self, threshold_ms: u64) -> Vec<&LoggedEvent> {
        self.events
            .iter()
            .filter(|e| e.timestamp_ms >= threshold_ms)
            .collect()
    }
}

// ---------------------------------------------------------------------------
// EventLogStore
// ---------------------------------------------------------------------------

/// Holds up to `max_logs` `EventLog` instances; evicts the oldest when full.
#[derive(Debug, Clone)]
pub struct EventLogStore {
    pub logs: Vec<EventLog>,
    pub max_logs: usize,
}

impl EventLogStore {
    /// Create a new store with the given capacity.
    pub fn new(max_logs: usize) -> Self {
        Self {
            logs: Vec::new(),
            max_logs,
        }
    }

    /// Add a log, evicting the oldest entry if over `max_logs`.
    pub fn add_log(&mut self, log: EventLog) {
        if self.logs.len() >= self.max_logs {
            self.logs.remove(0);
        }
        self.logs.push(log);
    }

    /// Total number of events across all stored logs.
    pub fn total_events(&self) -> usize {
        self.logs.iter().map(|l| l.events.len()).sum()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod event_log_tests {
    use super::*;

    fn make_event(kind: EventKind, ts: u64, with_ctx: bool) -> LoggedEvent {
        LoggedEvent {
            kind,
            message: "test".to_string(),
            timestamp_ms: ts,
            context: if with_ctx { Some("ctx".to_string()) } else { None },
        }
    }

    #[test]
    fn kind_is_user_true_for_user_action() {
        assert!(EventKind::UserAction.is_user());
    }

    #[test]
    fn kind_is_user_false_for_others() {
        assert!(!EventKind::SystemEvent.is_user());
        assert!(!EventKind::ErrorEvent.is_user());
        assert!(!EventKind::DebugEvent.is_user());
    }

    #[test]
    fn kind_severity_level_error_is_4() {
        assert_eq!(EventKind::ErrorEvent.severity_level(), 4);
    }

    #[test]
    fn kind_severity_levels_all() {
        assert_eq!(EventKind::UserAction.severity_level(), 1);
        assert_eq!(EventKind::SystemEvent.severity_level(), 2);
        assert_eq!(EventKind::DebugEvent.severity_level(), 0);
    }

    #[test]
    fn event_has_context() {
        let with_ctx = make_event(EventKind::UserAction, 0, true);
        let without_ctx = make_event(EventKind::UserAction, 0, false);
        assert!(with_ctx.has_context());
        assert!(!without_ctx.has_context());
    }

    #[test]
    fn event_is_error() {
        let err = make_event(EventKind::ErrorEvent, 0, false);
        let not_err = make_event(EventKind::UserAction, 0, false);
        assert!(err.is_error());
        assert!(!not_err.is_error());
    }

    #[test]
    fn log_errors_count() {
        let mut log = EventLog::default();
        log.push(make_event(EventKind::ErrorEvent, 10, false));
        log.push(make_event(EventKind::UserAction, 20, false));
        log.push(make_event(EventKind::ErrorEvent, 30, false));
        assert_eq!(log.errors().len(), 2);
    }

    #[test]
    fn log_user_actions_count() {
        let mut log = EventLog::default();
        log.push(make_event(EventKind::UserAction, 10, false));
        log.push(make_event(EventKind::SystemEvent, 20, false));
        log.push(make_event(EventKind::UserAction, 30, false));
        assert_eq!(log.user_actions().len(), 2);
    }

    #[test]
    fn log_since_filter() {
        let mut log = EventLog::default();
        log.push(make_event(EventKind::DebugEvent, 100, false));
        log.push(make_event(EventKind::DebugEvent, 200, false));
        log.push(make_event(EventKind::DebugEvent, 300, false));
        let result = log.since(200);
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|e| e.timestamp_ms >= 200));
    }

    #[test]
    fn store_eviction() {
        let mut store = EventLogStore::new(2);
        store.add_log(EventLog::default());
        store.add_log(EventLog::default());
        store.add_log(EventLog::default());
        assert_eq!(store.logs.len(), 2);
    }

    #[test]
    fn store_total_events() {
        let mut store = EventLogStore::new(10);
        let mut log1 = EventLog::default();
        log1.push(make_event(EventKind::UserAction, 1, false));
        log1.push(make_event(EventKind::UserAction, 2, false));
        let mut log2 = EventLog::default();
        log2.push(make_event(EventKind::SystemEvent, 3, false));
        store.add_log(log1);
        store.add_log(log2);
        assert_eq!(store.total_events(), 3);
    }
}
