#![deny(unsafe_code)]

#[derive(Debug, Clone)]
pub enum ComposeEvent {
    Started { backend: String, entity_id: String },
    Progress { percent: f32, stage: String },
    Completed { artifact_hash: [u8; 32], byte_size: u64 },
    Failed { reason: String },
}

pub trait ProgressSink: Send + Sync {
    fn emit(&self, event: ComposeEvent);
}

pub struct LogProgressSink;

impl ProgressSink for LogProgressSink {
    fn emit(&self, event: ComposeEvent) {
        // In production this would log to telemetry
        let _ = event;
    }
}

pub struct VecProgressSink {
    pub events: std::sync::Mutex<Vec<ComposeEvent>>,
}

impl VecProgressSink {
    pub fn new() -> Self { Self { events: std::sync::Mutex::new(vec![]) } }
    pub fn take(&self) -> Vec<ComposeEvent> {
        self.events.lock().map(|mut v| std::mem::take(&mut *v)).unwrap_or_default()
    }
}

impl ProgressSink for VecProgressSink {
    fn emit(&self, event: ComposeEvent) {
        if let Ok(mut v) = self.events.lock() { v.push(event); }
    }
}
