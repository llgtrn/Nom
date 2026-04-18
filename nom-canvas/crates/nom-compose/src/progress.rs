#![deny(unsafe_code)]

#[derive(Debug, Clone)]
pub enum ComposeEvent {
    Started {
        backend: String,
        entity_id: String,
    },
    Progress {
        percent: f32,
        stage: String,
    },
    Completed {
        artifact_hash: [u8; 32],
        byte_size: u64,
    },
    Failed {
        reason: String,
    },
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
    pub fn new() -> Self {
        Self {
            events: std::sync::Mutex::new(vec![]),
        }
    }
    pub fn take(&self) -> Vec<ComposeEvent> {
        self.events
            .lock()
            .map(|mut v| std::mem::take(&mut *v))
            .unwrap_or_default()
    }
}

impl Default for VecProgressSink {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressSink for VecProgressSink {
    fn emit(&self, event: ComposeEvent) {
        if let Ok(mut v) = self.events.lock() {
            v.push(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec_progress_sink_collects_events() {
        let sink = VecProgressSink::new();
        sink.emit(ComposeEvent::Started {
            backend: "image".into(),
            entity_id: "e1".into(),
        });
        sink.emit(ComposeEvent::Progress {
            percent: 0.5,
            stage: "encoding".into(),
        });
        sink.emit(ComposeEvent::Completed {
            artifact_hash: [0u8; 32],
            byte_size: 100,
        });
        let events = sink.take();
        assert_eq!(events.len(), 3);
    }

    #[test]
    fn vec_progress_sink_take_clears() {
        let sink = VecProgressSink::new();
        sink.emit(ComposeEvent::Failed {
            reason: "oops".into(),
        });
        let first = sink.take();
        assert_eq!(first.len(), 1);
        let second = sink.take();
        assert!(second.is_empty(), "take() must clear the internal buffer");
    }

    #[test]
    fn log_progress_sink_accepts_all_variants() {
        // LogProgressSink drops events silently; this just verifies it doesn't panic.
        let sink = LogProgressSink;
        sink.emit(ComposeEvent::Started {
            backend: "audio".into(),
            entity_id: "x".into(),
        });
        sink.emit(ComposeEvent::Progress {
            percent: 1.0,
            stage: "done".into(),
        });
        sink.emit(ComposeEvent::Completed {
            artifact_hash: [1u8; 32],
            byte_size: 42,
        });
        sink.emit(ComposeEvent::Failed {
            reason: "test error".into(),
        });
    }

    #[test]
    fn vec_progress_sink_new_starts_empty() {
        let sink = VecProgressSink::new();
        let events = sink.take();
        assert!(events.is_empty());
    }

    #[test]
    fn progress_sink_events_ordered() {
        let sink = VecProgressSink::new();
        sink.emit(ComposeEvent::Started {
            backend: "video".into(),
            entity_id: "e1".into(),
        });
        sink.emit(ComposeEvent::Progress {
            percent: 0.25,
            stage: "step1".into(),
        });
        sink.emit(ComposeEvent::Progress {
            percent: 0.75,
            stage: "step2".into(),
        });
        sink.emit(ComposeEvent::Completed {
            artifact_hash: [0u8; 32],
            byte_size: 256,
        });
        let events = sink.take();
        assert_eq!(events.len(), 4);
        // First event must be Started.
        assert!(matches!(events[0], ComposeEvent::Started { .. }));
        // Last event must be Completed.
        assert!(matches!(events[3], ComposeEvent::Completed { .. }));
        // Middle events must be Progress.
        assert!(matches!(events[1], ComposeEvent::Progress { .. }));
        assert!(matches!(events[2], ComposeEvent::Progress { .. }));
    }

    #[test]
    fn progress_sink_last_progress() {
        let sink = VecProgressSink::new();
        sink.emit(ComposeEvent::Progress {
            percent: 0.1,
            stage: "init".into(),
        });
        sink.emit(ComposeEvent::Progress {
            percent: 0.5,
            stage: "mid".into(),
        });
        sink.emit(ComposeEvent::Progress {
            percent: 0.9,
            stage: "final".into(),
        });
        let events = sink.take();
        // The last progress event should have percent 0.9.
        let last_progress = events
            .iter()
            .filter_map(|e| {
                if let ComposeEvent::Progress { percent, .. } = e {
                    Some(*percent)
                } else {
                    None
                }
            })
            .next_back();
        assert!(last_progress.is_some());
        assert!((last_progress.unwrap() - 0.9).abs() < 1e-5);
    }

    #[test]
    fn compose_event_failed_reason_preserved() {
        let sink = VecProgressSink::new();
        sink.emit(ComposeEvent::Failed {
            reason: "timeout after 30s".into(),
        });
        let events = sink.take();
        assert_eq!(events.len(), 1);
        if let ComposeEvent::Failed { reason } = &events[0] {
            assert_eq!(reason, "timeout after 30s");
        } else {
            panic!("expected Failed event");
        }
    }

    #[test]
    fn compose_event_completed_fields_preserved() {
        let sink = VecProgressSink::new();
        let hash = [0xffu8; 32];
        sink.emit(ComposeEvent::Completed {
            artifact_hash: hash,
            byte_size: 8192,
        });
        let events = sink.take();
        if let ComposeEvent::Completed {
            artifact_hash,
            byte_size,
        } = events[0].clone()
        {
            assert_eq!(artifact_hash, hash);
            assert_eq!(byte_size, 8192);
        } else {
            panic!("expected Completed event");
        }
    }

    #[test]
    fn compose_event_started_fields_preserved() {
        let sink = VecProgressSink::new();
        sink.emit(ComposeEvent::Started {
            backend: "render".into(),
            entity_id: "e42".into(),
        });
        let events = sink.take();
        if let ComposeEvent::Started { backend, entity_id } = &events[0] {
            assert_eq!(backend, "render");
            assert_eq!(entity_id, "e42");
        } else {
            panic!("expected Started event");
        }
    }

    #[test]
    fn vec_progress_sink_many_events() {
        let sink = VecProgressSink::new();
        for i in 0..20u32 {
            sink.emit(ComposeEvent::Progress {
                percent: i as f32 * 5.0,
                stage: format!("step_{i}"),
            });
        }
        let events = sink.take();
        assert_eq!(events.len(), 20);
        assert!(sink.take().is_empty(), "take() must clear after drain");
    }
}
