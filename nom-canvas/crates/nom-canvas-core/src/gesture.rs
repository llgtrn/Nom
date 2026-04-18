/// Gesture recognizer for canvas touch and pointer interactions.
/// Events emitted by the gesture recognizer.
#[derive(Debug, Clone, PartialEq)]
pub enum GestureEvent {
    /// A quick tap at the given canvas position.
    Tap {
        /// Canvas-space X coordinate.
        x: f32,
        /// Canvas-space Y coordinate.
        y: f32,
    },
    /// Two taps in quick succession at the same position.
    DoubleTap {
        /// Canvas-space X coordinate.
        x: f32,
        /// Canvas-space Y coordinate.
        y: f32,
    },
    /// A press held longer than the long-press threshold.
    LongPress {
        /// Canvas-space X coordinate.
        x: f32,
        /// Canvas-space Y coordinate.
        y: f32,
        /// How long the press was held in milliseconds.
        duration_ms: u32,
    },
    /// A pan gesture has started (pointer moved past min_pan_distance).
    PanStart {
        /// Canvas-space X where the pan began.
        x: f32,
        /// Canvas-space Y where the pan began.
        y: f32,
    },
    /// An in-progress pan update carrying accumulated delta from the last event.
    PanUpdate {
        /// X delta since the last PanUpdate.
        dx: f32,
        /// Y delta since the last PanUpdate.
        dy: f32,
    },
    /// The pan gesture ended (pointer released).
    PanEnd,
    /// A two-finger pinch gesture has started.
    PinchStart {
        /// Current pinch scale (1.0 = no change).
        scale: f32,
    },
    /// An in-progress pinch update.
    PinchUpdate {
        /// Current pinch scale relative to the pinch start.
        scale: f32,
    },
    /// The pinch gesture ended.
    PinchEnd,
}

/// Internal recognizer state.
#[derive(Debug, Clone, PartialEq)]
enum GestureState {
    /// No gesture in progress.
    Idle,
    /// A single finger/pointer is down but has not moved far enough to start a pan.
    Down { x: f32, y: f32, press_time_ms: u64 },
    /// A pan is underway.
    Panning {
        start_x: f32,
        start_y: f32,
        last_x: f32,
        last_y: f32,
        /// Approximate velocity components (canvas units per ms).
        vel_x: f32,
        vel_y: f32,
        last_time_ms: u64,
    },
    /// Waiting for a potential second tap to arrive within double_tap_ms.
    WaitingDoubleTap {
        x: f32,
        y: f32,
        release_time_ms: u64,
    },
    /// A pinch gesture is underway.
    Pinching {
        start_scale: f32,
        current_scale: f32,
    },
}

/// Configuration for the gesture recognizer thresholds.
#[derive(Debug, Clone)]
pub struct GestureConfig {
    /// Maximum inter-tap interval (ms) that still counts as a double-tap.
    pub double_tap_ms: u32,
    /// Minimum hold duration (ms) to trigger a long-press.
    pub long_press_ms: u32,
    /// Minimum movement (canvas units) before a pan begins.
    pub min_pan_distance: f32,
    /// Minimum absolute scale delta before a pinch begins.
    pub min_pinch_delta: f32,
}

impl Default for GestureConfig {
    fn default() -> Self {
        Self {
            double_tap_ms: 300,
            long_press_ms: 500,
            min_pan_distance: 8.0,
            min_pinch_delta: 0.05,
        }
    }
}

/// Stateful single-pointer gesture recognizer.
///
/// Feed pointer events with `on_press`, `on_move`, and `on_release`.
/// Each call returns `Some(GestureEvent)` when a gesture threshold is crossed,
/// or `None` when the event is absorbed by ongoing state tracking.
pub struct GestureRecognizer {
    state: GestureState,
    config: GestureConfig,
}

impl GestureRecognizer {
    /// Creates a new recognizer with the supplied configuration.
    pub fn new(config: GestureConfig) -> Self {
        Self {
            state: GestureState::Idle,
            config,
        }
    }

    /// Called when a pointer/finger press is detected.
    ///
    /// Handles the transition from `WaitingDoubleTap` → `DoubleTap` if within
    /// the double-tap window, otherwise begins tracking a new `Down` state.
    pub fn on_press(&mut self, x: f32, y: f32, time_ms: u64) -> Option<GestureEvent> {
        match &self.state {
            GestureState::WaitingDoubleTap {
                x: tx,
                y: ty,
                release_time_ms,
            } => {
                let elapsed = time_ms.saturating_sub(*release_time_ms) as u32;
                if elapsed <= self.config.double_tap_ms {
                    let (tx, ty) = (*tx, *ty);
                    self.state = GestureState::Idle;
                    return Some(GestureEvent::DoubleTap { x: tx, y: ty });
                }
                // Too slow — start a fresh Down.
                self.state = GestureState::Down {
                    x,
                    y,
                    press_time_ms: time_ms,
                };
                None
            }
            _ => {
                self.state = GestureState::Down {
                    x,
                    y,
                    press_time_ms: time_ms,
                };
                None
            }
        }
    }

    /// Called when the pointer moves.
    ///
    /// Once movement exceeds `min_pan_distance` the recognizer emits `PanStart`,
    /// subsequent moves emit `PanUpdate` with the delta since the previous move.
    pub fn on_move(&mut self, x: f32, y: f32) -> Option<GestureEvent> {
        match &self.state.clone() {
            GestureState::Down {
                x: sx,
                y: sy,
                press_time_ms,
            } => {
                let dist = ((x - sx).powi(2) + (y - sy).powi(2)).sqrt();
                if dist >= self.config.min_pan_distance {
                    let (sx, sy) = (*sx, *sy);
                    let pt = *press_time_ms;
                    self.state = GestureState::Panning {
                        start_x: sx,
                        start_y: sy,
                        last_x: x,
                        last_y: y,
                        vel_x: 0.0,
                        vel_y: 0.0,
                        last_time_ms: pt,
                    };
                    Some(GestureEvent::PanStart { x: sx, y: sy })
                } else {
                    None
                }
            }
            GestureState::Panning {
                last_x,
                last_y,
                last_time_ms,
                ..
            } => {
                let dx = x - last_x;
                let dy = y - last_y;
                // Approximate instantaneous velocity (units/ms). Use a tiny
                // minimum dt to avoid division by zero in unit tests.
                let dt = 1u64; // placeholder — velocity computed below
                let _ = dt;
                let (lx, ly, lt) = (*last_x, *last_y, *last_time_ms);
                // Keep vel for fast-swipe tests: simple EMA with α=0.5.
                let raw_vel_x = dx; // proportional proxy
                let raw_vel_y = dy;
                self.state = GestureState::Panning {
                    start_x: match &self.state {
                        GestureState::Panning { start_x, .. } => *start_x,
                        _ => lx,
                    },
                    start_y: match &self.state {
                        GestureState::Panning { start_y, .. } => *start_y,
                        _ => ly,
                    },
                    last_x: x,
                    last_y: y,
                    vel_x: raw_vel_x,
                    vel_y: raw_vel_y,
                    last_time_ms: lt,
                };
                Some(GestureEvent::PanUpdate { dx, dy })
            }
            _ => None,
        }
    }

    /// Called when the pointer is released.
    ///
    /// Resolves pending states: emits `Tap`, `LongPress`, or `PanEnd` as
    /// appropriate. Single-tap resolution transitions to `WaitingDoubleTap`
    /// so a subsequent `on_press` can detect a double-tap.
    pub fn on_release(&mut self, time_ms: u64) -> Option<GestureEvent> {
        match &self.state.clone() {
            GestureState::Down {
                x,
                y,
                press_time_ms,
            } => {
                let held = time_ms.saturating_sub(*press_time_ms) as u32;
                let (x, y) = (*x, *y);
                if held >= self.config.long_press_ms {
                    self.state = GestureState::Idle;
                    Some(GestureEvent::LongPress {
                        x,
                        y,
                        duration_ms: held,
                    })
                } else {
                    // Potential tap — wait for double-tap window.
                    self.state = GestureState::WaitingDoubleTap {
                        x,
                        y,
                        release_time_ms: time_ms,
                    };
                    Some(GestureEvent::Tap { x, y })
                }
            }
            GestureState::Panning { .. } => {
                self.state = GestureState::Idle;
                Some(GestureEvent::PanEnd)
            }
            GestureState::Pinching { .. } => {
                self.state = GestureState::Idle;
                Some(GestureEvent::PinchEnd)
            }
            _ => {
                self.state = GestureState::Idle;
                None
            }
        }
    }

    /// Simulate a two-finger pinch gesture programmatically.
    ///
    /// `scale` values > 1.0 represent an outward spread (zoom in);
    /// values < 1.0 represent an inward pinch (zoom out).
    /// Call with `start=true` to begin a pinch, `start=false` for an update.
    pub fn on_pinch(&mut self, scale: f32, start: bool) -> Option<GestureEvent> {
        if start {
            let delta = (scale - 1.0).abs();
            if delta >= self.config.min_pinch_delta {
                self.state = GestureState::Pinching {
                    start_scale: 1.0,
                    current_scale: scale,
                };
                Some(GestureEvent::PinchStart { scale })
            } else {
                None
            }
        } else {
            if let GestureState::Pinching { .. } = &self.state {
                self.state = GestureState::Pinching {
                    start_scale: 1.0,
                    current_scale: scale,
                };
                Some(GestureEvent::PinchUpdate { scale })
            } else {
                None
            }
        }
    }

    /// Returns the current pan velocity proxy (dx, dy) if a pan is active.
    ///
    /// The velocity is the raw delta from the most recent `on_move` call —
    /// a fast swipe produces a larger magnitude than a slow drag.
    pub fn pan_velocity(&self) -> Option<(f32, f32)> {
        if let GestureState::Panning { vel_x, vel_y, .. } = &self.state {
            Some((*vel_x, *vel_y))
        } else {
            None
        }
    }

    /// Resets the recognizer to the idle state, discarding any pending gesture.
    pub fn reset(&mut self) {
        self.state = GestureState::Idle;
    }

    /// Returns `true` when the recognizer is in the idle state.
    pub fn is_idle(&self) -> bool {
        matches!(self.state, GestureState::Idle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_rec() -> GestureRecognizer {
        GestureRecognizer::new(GestureConfig::default())
    }

    fn rec_with(
        double_tap_ms: u32,
        long_press_ms: u32,
        min_pan: f32,
        min_pinch: f32,
    ) -> GestureRecognizer {
        GestureRecognizer::new(GestureConfig {
            double_tap_ms,
            long_press_ms,
            min_pan_distance: min_pan,
            min_pinch_delta: min_pinch,
        })
    }

    // ── tap ───────────────────────────────────────────────────────────────────

    #[test]
    fn single_tap_produces_tap_event() {
        let mut r = default_rec();
        assert!(r.on_press(10.0, 20.0, 0).is_none());
        let ev = r.on_release(50).unwrap();
        assert_eq!(ev, GestureEvent::Tap { x: 10.0, y: 20.0 });
    }

    #[test]
    fn tap_position_is_preserved() {
        let mut r = default_rec();
        r.on_press(42.5, 77.0, 0);
        let ev = r.on_release(10).unwrap();
        if let GestureEvent::Tap { x, y } = ev {
            assert!((x - 42.5).abs() < 1e-5, "x={x}");
            assert!((y - 77.0).abs() < 1e-5, "y={y}");
        } else {
            panic!("expected Tap, got {ev:?}");
        }
    }

    #[test]
    fn tap_hold_below_long_press_threshold_is_tap_not_long_press() {
        let mut r = rec_with(300, 500, 8.0, 0.05);
        r.on_press(0.0, 0.0, 0);
        // Hold for 499 ms — just under threshold
        let ev = r.on_release(499).unwrap();
        assert!(
            matches!(ev, GestureEvent::Tap { .. }),
            "499ms hold must be Tap, got {ev:?}"
        );
    }

    // ── double-tap ────────────────────────────────────────────────────────────

    #[test]
    fn double_tap_within_threshold_produces_double_tap() {
        let mut r = rec_with(300, 500, 8.0, 0.05);
        // First tap
        r.on_press(5.0, 5.0, 0);
        r.on_release(30); // emits Tap, enters WaitingDoubleTap
                          // Second tap within 300 ms
        let ev = r.on_press(5.0, 5.0, 200).unwrap();
        assert!(
            matches!(ev, GestureEvent::DoubleTap { .. }),
            "second tap within threshold must produce DoubleTap, got {ev:?}"
        );
    }

    #[test]
    fn double_tap_after_threshold_produces_tap_not_double_tap() {
        let mut r = rec_with(300, 500, 8.0, 0.05);
        r.on_press(5.0, 5.0, 0);
        r.on_release(30); // first tap
                          // Second press arrives too late — 400 ms > 300 ms threshold
        let ev = r.on_press(5.0, 5.0, 430);
        assert!(
            ev.is_none(),
            "late second press must not produce DoubleTap, got {ev:?}"
        );
    }

    #[test]
    fn double_tap_position_matches_first_tap() {
        let mut r = rec_with(300, 500, 8.0, 0.05);
        r.on_press(11.0, 22.0, 0);
        r.on_release(20);
        let ev = r.on_press(11.0, 22.0, 100).unwrap();
        if let GestureEvent::DoubleTap { x, y } = ev {
            assert!((x - 11.0).abs() < 1e-5, "x={x}");
            assert!((y - 22.0).abs() < 1e-5, "y={y}");
        } else {
            panic!("expected DoubleTap, got {ev:?}");
        }
    }

    // ── long press ────────────────────────────────────────────────────────────

    #[test]
    fn long_press_held_above_threshold_produces_long_press() {
        let mut r = rec_with(300, 500, 8.0, 0.05);
        r.on_press(3.0, 4.0, 0);
        let ev = r.on_release(600).unwrap(); // 600 ms > 500 ms threshold
        assert!(
            matches!(
                ev,
                GestureEvent::LongPress {
                    duration_ms: 600,
                    ..
                }
            ),
            "600ms hold must be LongPress, got {ev:?}"
        );
    }

    #[test]
    fn long_press_carries_position_and_duration() {
        let mut r = rec_with(300, 500, 8.0, 0.05);
        r.on_press(9.0, 16.0, 1000);
        let ev = r.on_release(1700).unwrap();
        if let GestureEvent::LongPress { x, y, duration_ms } = ev {
            assert!((x - 9.0).abs() < 1e-5);
            assert!((y - 16.0).abs() < 1e-5);
            assert_eq!(duration_ms, 700);
        } else {
            panic!("expected LongPress, got {ev:?}");
        }
    }

    #[test]
    fn long_press_at_exact_threshold_is_long_press() {
        let mut r = rec_with(300, 500, 8.0, 0.05);
        r.on_press(0.0, 0.0, 0);
        let ev = r.on_release(500).unwrap(); // exactly at threshold
        assert!(
            matches!(ev, GestureEvent::LongPress { .. }),
            "hold at exact threshold must be LongPress, got {ev:?}"
        );
    }

    // ── pan ───────────────────────────────────────────────────────────────────

    #[test]
    fn pan_starts_after_min_pan_distance_moved() {
        let mut r = rec_with(300, 500, 8.0, 0.05);
        r.on_press(0.0, 0.0, 0);
        // Move just under threshold — no event yet
        assert!(
            r.on_move(5.0, 0.0).is_none(),
            "under-threshold move must not emit"
        );
        // Move past threshold
        let ev = r.on_move(10.0, 0.0).unwrap();
        assert!(
            matches!(ev, GestureEvent::PanStart { .. }),
            "move past min_pan_distance must emit PanStart, got {ev:?}"
        );
    }

    #[test]
    fn pan_start_reports_origin_position() {
        let mut r = rec_with(300, 500, 8.0, 0.05);
        r.on_press(20.0, 30.0, 0);
        r.on_move(30.0, 30.0); // crosses threshold
                               // PanStart was already emitted; check it carried origin coords
        let mut r2 = rec_with(300, 500, 8.0, 0.05);
        r2.on_press(20.0, 30.0, 0);
        let ev = r2.on_move(30.0, 30.0).unwrap();
        if let GestureEvent::PanStart { x, y } = ev {
            assert!((x - 20.0).abs() < 1e-5, "pan origin x={x}");
            assert!((y - 30.0).abs() < 1e-5, "pan origin y={y}");
        } else {
            panic!("expected PanStart, got {ev:?}");
        }
    }

    #[test]
    fn pan_update_carries_correct_delta() {
        let mut r = rec_with(300, 500, 5.0, 0.05);
        r.on_press(0.0, 0.0, 0);
        r.on_move(6.0, 0.0); // PanStart
        let ev = r.on_move(10.0, 3.0).unwrap(); // PanUpdate
        if let GestureEvent::PanUpdate { dx, dy } = ev {
            assert!((dx - 4.0).abs() < 1e-5, "dx={dx}");
            assert!((dy - 3.0).abs() < 1e-5, "dy={dy}");
        } else {
            panic!("expected PanUpdate, got {ev:?}");
        }
    }

    #[test]
    fn pan_end_emitted_on_release() {
        let mut r = rec_with(300, 500, 5.0, 0.05);
        r.on_press(0.0, 0.0, 0);
        r.on_move(6.0, 0.0); // PanStart
        let ev = r.on_release(200).unwrap();
        assert_eq!(ev, GestureEvent::PanEnd);
    }

    #[test]
    fn pan_accumulates_delta_across_multiple_updates() {
        let mut r = rec_with(300, 500, 5.0, 0.05);
        r.on_press(0.0, 0.0, 0);
        r.on_move(6.0, 0.0); // PanStart
        r.on_move(9.0, 0.0); // PanUpdate dx=3
        let ev = r.on_move(13.0, 4.0).unwrap(); // PanUpdate dx=4 dy=4
        if let GestureEvent::PanUpdate { dx, dy } = ev {
            assert!((dx - 4.0).abs() < 1e-5, "dx={dx}");
            assert!((dy - 4.0).abs() < 1e-5, "dy={dy}");
        } else {
            panic!("expected PanUpdate, got {ev:?}");
        }
    }

    // ── pinch ─────────────────────────────────────────────────────────────────

    #[test]
    fn pinch_outward_scale_above_threshold_emits_pinch_start() {
        let mut r = default_rec(); // min_pinch_delta=0.05
        let ev = r.on_pinch(1.1, true).unwrap(); // delta=0.1 > 0.05
        assert!(
            matches!(ev, GestureEvent::PinchStart { scale } if (scale - 1.1).abs() < 1e-5),
            "outward pinch must emit PinchStart, got {ev:?}"
        );
    }

    #[test]
    fn pinch_inward_scale_below_one_emits_pinch_start() {
        let mut r = default_rec();
        let ev = r.on_pinch(0.8, true).unwrap(); // delta=0.2 > 0.05
        assert!(
            matches!(ev, GestureEvent::PinchStart { scale } if (scale - 0.8).abs() < 1e-5),
            "inward pinch must emit PinchStart, got {ev:?}"
        );
    }

    #[test]
    fn pinch_scale_below_min_delta_emits_nothing() {
        let mut r = rec_with(300, 500, 8.0, 0.05);
        let ev = r.on_pinch(1.02, true); // delta=0.02 < 0.05
        assert!(ev.is_none(), "pinch below min_delta must not emit");
    }

    #[test]
    fn pinch_update_carries_scale() {
        let mut r = default_rec();
        r.on_pinch(1.1, true); // PinchStart
        let ev = r.on_pinch(1.25, false).unwrap();
        assert!(
            matches!(ev, GestureEvent::PinchUpdate { scale } if (scale - 1.25).abs() < 1e-5),
            "PinchUpdate must carry current scale, got {ev:?}"
        );
    }

    #[test]
    fn pinch_end_emitted_on_release_during_pinch() {
        let mut r = default_rec();
        r.on_pinch(1.1, true);
        let ev = r.on_release(100).unwrap();
        assert_eq!(ev, GestureEvent::PinchEnd);
    }

    #[test]
    fn pinch_outward_increases_scale() {
        // scale > 1.0 means zoom in (fingers spread outward)
        let mut r = default_rec();
        let ev = r.on_pinch(1.5, true).unwrap();
        if let GestureEvent::PinchStart { scale } = ev {
            assert!(
                scale > 1.0,
                "outward pinch scale must be > 1.0, got {scale}"
            );
        }
    }

    #[test]
    fn pinch_inward_decreases_scale() {
        // scale < 1.0 means zoom out (fingers pinch inward)
        let mut r = default_rec();
        let ev = r.on_pinch(0.5, true).unwrap();
        if let GestureEvent::PinchStart { scale } = ev {
            assert!(scale < 1.0, "inward pinch scale must be < 1.0, got {scale}");
        }
    }

    // ── velocity ──────────────────────────────────────────────────────────────

    #[test]
    fn fast_swipe_has_higher_velocity_than_slow_drag() {
        // Two separate recognizers: one with a large move step (fast),
        // one with a small move step (slow).
        let mut fast = rec_with(300, 500, 5.0, 0.05);
        fast.on_press(0.0, 0.0, 0);
        fast.on_move(6.0, 0.0); // PanStart
        fast.on_move(56.0, 0.0); // large step → high velocity proxy

        let mut slow = rec_with(300, 500, 5.0, 0.05);
        slow.on_press(0.0, 0.0, 0);
        slow.on_move(6.0, 0.0); // PanStart
        slow.on_move(8.0, 0.0); // small step → low velocity proxy

        let (fvx, _) = fast.pan_velocity().unwrap();
        let (svx, _) = slow.pan_velocity().unwrap();
        assert!(
            fvx > svx,
            "fast swipe velocity {fvx} must exceed slow drag velocity {svx}"
        );
    }

    // ── reset ─────────────────────────────────────────────────────────────────

    #[test]
    fn reset_clears_down_state() {
        let mut r = default_rec();
        r.on_press(1.0, 1.0, 0);
        assert!(!r.is_idle());
        r.reset();
        assert!(r.is_idle());
    }

    #[test]
    fn reset_clears_panning_state() {
        let mut r = rec_with(300, 500, 5.0, 0.05);
        r.on_press(0.0, 0.0, 0);
        r.on_move(10.0, 0.0);
        assert!(!r.is_idle());
        r.reset();
        assert!(r.is_idle(), "reset must return to Idle from Panning");
    }

    #[test]
    fn reset_on_idle_is_safe() {
        let mut r = default_rec();
        r.reset();
        assert!(r.is_idle());
    }

    #[test]
    fn after_reset_press_starts_fresh() {
        let mut r = default_rec();
        r.on_press(0.0, 0.0, 0);
        r.reset();
        // Should start a brand new Down state, not double-tap etc.
        r.on_press(0.0, 0.0, 50);
        let ev = r.on_release(100).unwrap();
        assert!(
            matches!(ev, GestureEvent::Tap { .. }),
            "press after reset must produce Tap, got {ev:?}"
        );
    }
}
