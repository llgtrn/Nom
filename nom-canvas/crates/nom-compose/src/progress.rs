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

    #[test]
    fn progress_zero_to_half_to_full_in_order() {
        let sink = VecProgressSink::new();
        sink.emit(ComposeEvent::Progress { percent: 0.0, stage: "start".into() });
        sink.emit(ComposeEvent::Progress { percent: 0.5, stage: "mid".into() });
        sink.emit(ComposeEvent::Progress { percent: 1.0, stage: "end".into() });
        let events = sink.take();
        let percents: Vec<f32> = events.iter().filter_map(|e| {
            if let ComposeEvent::Progress { percent, .. } = e { Some(*percent) } else { None }
        }).collect();
        assert_eq!(percents.len(), 3);
        assert!((percents[0] - 0.0).abs() < 1e-5);
        assert!((percents[1] - 0.5).abs() < 1e-5);
        assert!((percents[2] - 1.0).abs() < 1e-5);
        // verify ascending order
        assert!(percents[0] <= percents[1] && percents[1] <= percents[2]);
    }

    #[test]
    fn progress_out_of_order_values_are_stored_as_emitted() {
        // The sink does not clamp or reorder — it stores whatever is emitted.
        let sink = VecProgressSink::new();
        sink.emit(ComposeEvent::Progress { percent: 0.8, stage: "a".into() });
        sink.emit(ComposeEvent::Progress { percent: 0.3, stage: "b".into() }); // out-of-order
        sink.emit(ComposeEvent::Progress { percent: 1.2, stage: "c".into() }); // over 1.0
        let events = sink.take();
        let percents: Vec<f32> = events.iter().filter_map(|e| {
            if let ComposeEvent::Progress { percent, .. } = e { Some(*percent) } else { None }
        }).collect();
        // All three stored as-emitted; sink does not clamp.
        assert_eq!(percents, vec![0.8, 0.3, 1.2]);
    }

    #[test]
    fn progress_callback_invoked_for_each_event() {
        // Simulates a callback-like counter pattern: VecProgressSink fires for each emit call.
        let sink = VecProgressSink::new();
        let count = 7;
        for i in 0..count {
            sink.emit(ComposeEvent::Progress { percent: i as f32 / count as f32, stage: format!("s{i}") });
        }
        assert_eq!(sink.take().len(), count, "callback must be invoked once per emit");
    }

    #[test]
    fn progress_stage_names_preserved_in_order() {
        let sink = VecProgressSink::new();
        let stages = ["init", "validate", "encode", "finalize"];
        for (i, stage) in stages.iter().enumerate() {
            sink.emit(ComposeEvent::Progress { percent: i as f32 / stages.len() as f32, stage: (*stage).into() });
        }
        let events = sink.take();
        for (i, event) in events.iter().enumerate() {
            if let ComposeEvent::Progress { stage, .. } = event {
                assert_eq!(stage, stages[i]);
            }
        }
    }

    #[test]
    fn vec_progress_sink_default_starts_empty() {
        let sink = VecProgressSink::default();
        assert!(sink.take().is_empty());
    }

    #[test]
    fn progress_event_percent_stored_with_f32_precision() {
        let sink = VecProgressSink::new();
        sink.emit(ComposeEvent::Progress { percent: 0.333_333_3, stage: "third".into() });
        let events = sink.take();
        if let ComposeEvent::Progress { percent, .. } = events[0] {
            assert!((percent - 0.333_333_3_f32).abs() < 1e-6);
        }
    }

    #[test]
    fn progress_multiple_started_events_all_stored() {
        // Emit two Started events — sink stores both without deduplication.
        let sink = VecProgressSink::new();
        sink.emit(ComposeEvent::Started { backend: "video".into(), entity_id: "a".into() });
        sink.emit(ComposeEvent::Started { backend: "audio".into(), entity_id: "b".into() });
        let events = sink.take();
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], ComposeEvent::Started { .. }));
        assert!(matches!(events[1], ComposeEvent::Started { .. }));
    }

    // ── Wave AE new tests ────────────────────────────────────────────────────

    #[test]
    fn progress_initial_emit_can_be_zero() {
        // A backend conventionally emits percent=0.0 at the start of progress.
        let sink = VecProgressSink::new();
        sink.emit(ComposeEvent::Progress { percent: 0.0, stage: "start".into() });
        let events = sink.take();
        assert_eq!(events.len(), 1);
        if let ComposeEvent::Progress { percent, .. } = events[0] {
            assert!((percent - 0.0).abs() < f32::EPSILON, "initial progress must be 0.0");
        } else {
            panic!("expected Progress event");
        }
    }

    #[test]
    fn progress_final_emit_at_one_signals_completion() {
        // A backend conventionally emits percent=1.0 when finished.
        let sink = VecProgressSink::new();
        sink.emit(ComposeEvent::Progress { percent: 1.0, stage: "done".into() });
        let events = sink.take();
        if let ComposeEvent::Progress { percent, .. } = events[0] {
            assert!((percent - 1.0).abs() < f32::EPSILON, "final progress must be 1.0");
        }
    }

    #[test]
    fn progress_values_within_zero_to_one_inclusive() {
        // All progress values emitted in [0.0, 1.0] are stored without modification.
        let sink = VecProgressSink::new();
        let values = [0.0f32, 0.1, 0.25, 0.5, 0.75, 0.9, 1.0];
        for v in values {
            sink.emit(ComposeEvent::Progress { percent: v, stage: "step".into() });
        }
        let events = sink.take();
        for (i, event) in events.iter().enumerate() {
            if let ComposeEvent::Progress { percent, .. } = event {
                assert!(
                    (*percent - values[i]).abs() < 1e-5,
                    "progress value must be preserved as emitted"
                );
            }
        }
    }

    #[test]
    fn progress_observer_receives_each_update_exactly_once() {
        // Each emit call must result in exactly one stored event.
        let sink = VecProgressSink::new();
        let n = 10usize;
        for i in 0..n {
            sink.emit(ComposeEvent::Progress {
                percent: i as f32 / n as f32,
                stage: format!("s{i}"),
            });
        }
        assert_eq!(sink.take().len(), n, "observer must receive exactly n updates");
    }

    #[test]
    fn progress_percentage_fifty_percent_stored_as_half() {
        // 50% progress is stored as 0.5 in the f32 percent field.
        let sink = VecProgressSink::new();
        sink.emit(ComposeEvent::Progress { percent: 0.5, stage: "halfway".into() });
        let events = sink.take();
        if let ComposeEvent::Progress { percent, .. } = events[0] {
            assert!(
                (percent - 0.5).abs() < 1e-5,
                "50% progress must be stored as 0.5, got {percent}"
            );
        }
    }

    #[test]
    fn progress_stage_label_at_zero_percent() {
        // Stage label at 0% must be preserved.
        let sink = VecProgressSink::new();
        sink.emit(ComposeEvent::Progress { percent: 0.0, stage: "initialising".into() });
        let events = sink.take();
        if let ComposeEvent::Progress { stage, percent } = &events[0] {
            assert_eq!(stage, "initialising");
            assert!((*percent - 0.0).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn progress_stage_label_at_full_percent() {
        // Stage label at 100% must be preserved.
        let sink = VecProgressSink::new();
        sink.emit(ComposeEvent::Progress { percent: 1.0, stage: "complete".into() });
        let events = sink.take();
        if let ComposeEvent::Progress { stage, percent } = &events[0] {
            assert_eq!(stage, "complete");
            assert!((*percent - 1.0).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn progress_monotone_sequence_all_stored_in_order() {
        // Emit 5 monotonically increasing progress values and verify ordering.
        let sink = VecProgressSink::new();
        let steps = [0.0f32, 0.25, 0.5, 0.75, 1.0];
        for v in steps {
            sink.emit(ComposeEvent::Progress { percent: v, stage: "m".into() });
        }
        let events = sink.take();
        assert_eq!(events.len(), 5);
        let percents: Vec<f32> = events
            .iter()
            .filter_map(|e| {
                if let ComposeEvent::Progress { percent, .. } = e {
                    Some(*percent)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(percents.len(), 5);
        for w in percents.windows(2) {
            assert!(w[0] <= w[1], "monotone sequence must be non-decreasing in storage");
        }
    }

    #[test]
    fn progress_sink_independent_instances_do_not_share_state() {
        // Two separate VecProgressSink instances must not share event storage.
        let sink_a = VecProgressSink::new();
        let sink_b = VecProgressSink::new();
        sink_a.emit(ComposeEvent::Progress { percent: 0.5, stage: "a".into() });
        // sink_b receives nothing.
        let events_a = sink_a.take();
        let events_b = sink_b.take();
        assert_eq!(events_a.len(), 1);
        assert!(events_b.is_empty(), "sink_b must not see events emitted to sink_a");
    }
}
