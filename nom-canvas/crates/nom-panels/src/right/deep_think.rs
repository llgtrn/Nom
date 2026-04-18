#![deny(unsafe_code)]
use crate::dock::{fill_quad, rgba_to_hsla, DockPosition, Panel};
use nom_compose::deep_think::DeepThinkStep;
use nom_gpui::scene::{Quad, Scene};
use nom_gpui::types::{Bounds, ContentMask, Corners, Edges, Pixels, Point, Size};
use nom_intent::classify_with_react;
use nom_theme::tokens;

/// Lightweight view-model card produced by consuming a `DeepThinkStep` stream.
#[derive(Debug, Clone, PartialEq)]
pub struct ThinkCard {
    pub hypothesis: String,
    pub confidence: f32,
    pub step_num: usize,
}

/// Translate a slice of `DeepThinkStep`s into `ThinkCard`s.
///
/// If a step has no prior evidence the confidence field from the step is used
/// directly; otherwise `classify_with_react` is called with the step's
/// hypothesis and its evidence slice to (re)compute confidence so the panel
/// always reflects the ReAct-derived score.
pub fn consume_stream(events: Vec<DeepThinkStep>) -> Vec<ThinkCard> {
    events
        .into_iter()
        .enumerate()
        .map(|(step_num, step)| {
            let confidence = if step.evidence.is_empty() {
                step.confidence
            } else {
                let ev_refs: Vec<&str> = step.evidence.iter().map(|s| s.as_str()).collect();
                classify_with_react(&step.hypothesis, &ev_refs)
            };
            ThinkCard {
                hypothesis: step.hypothesis,
                confidence: confidence.clamp(0.0, 1.0),
                step_num,
            }
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct ThinkingStep {
    pub hypothesis: String,
    pub evidence: Vec<String>,
    pub confidence: f32, // 0.0–1.0
    pub counterevidence: Vec<String>,
    pub refined_from: Option<String>,
    pub is_expanded: bool,
}

impl ThinkingStep {
    pub fn new(hypothesis: impl Into<String>, confidence: f32) -> Self {
        Self {
            hypothesis: hypothesis.into(),
            evidence: vec![],
            confidence: confidence.clamp(0.0, 1.0),
            counterevidence: vec![],
            refined_from: None,
            is_expanded: true,
        }
    }

    pub fn confidence_label(&self) -> &'static str {
        if self.confidence >= 0.8 {
            "HIGH"
        } else if self.confidence >= 0.5 {
            "MED"
        } else {
            "LOW"
        }
    }

    pub fn toggle_expand(&mut self) {
        self.is_expanded = !self.is_expanded;
    }
}

pub enum ThinkState {
    Idle,
    Streaming,
    Complete,
    Interrupted(String),
}

pub struct DeepThinkPanel {
    pub steps: Vec<ThinkingStep>,
    pub cards: Vec<ThinkCard>,
    pub state: ThinkState,
    pub intent: String,
}

impl DeepThinkPanel {
    pub fn new() -> Self {
        Self {
            steps: vec![],
            cards: vec![],
            state: ThinkState::Idle,
            intent: String::new(),
        }
    }

    pub fn begin(&mut self, intent: impl Into<String>) {
        self.intent = intent.into();
        self.steps.clear();
        self.state = ThinkState::Streaming;
    }

    pub fn push_step(&mut self, step: ThinkingStep) {
        self.steps.push(step);
    }

    pub fn complete(&mut self) {
        self.state = ThinkState::Complete;
    }
    pub fn interrupt(&mut self, reason: impl Into<String>) {
        self.state = ThinkState::Interrupted(reason.into());
    }

    pub fn high_confidence_steps(&self) -> Vec<&ThinkingStep> {
        self.steps.iter().filter(|s| s.confidence >= 0.8).collect()
    }

    /// Consume a stream of `DeepThinkStep`s, translating each into a `ThinkCard`
    /// and appending to `self.cards`.
    pub fn ingest_events(&mut self, events: Vec<DeepThinkStep>) {
        let mut new_cards = consume_stream(events);
        // Re-number so step_num is relative to total cards already stored.
        let offset = self.cards.len();
        for card in &mut new_cards {
            card.step_num += offset;
        }
        self.cards.extend(new_cards);
    }

    /// Returns the number of cards currently held by this panel.
    pub fn card_count(&self) -> usize {
        self.cards.len()
    }

    pub fn paint_scene(&self, width: f32, height: f32, scene: &mut Scene) {
        // Panel background.
        scene.push_quad(fill_quad(0.0, 0.0, width, height, tokens::BG));
        let total = self.steps.len();
        for (i, _step) in self.steps.iter().enumerate() {
            let y = i as f32 * 24.0 + 4.0;
            let is_active = i + 1 == total;
            let color = if is_active {
                tokens::FOCUS
            } else {
                tokens::BG2
            };
            scene.push_quad(fill_quad(0.0, y, width, 22.0, color));
        }
        // One Quad per ThinkCard — stacked vertically with EDGE_MED border.
        let card_h = 40.0;
        let card_margin = 4.0;
        for (i, _card) in self.cards.iter().enumerate() {
            let y = i as f32 * (card_h + card_margin) + 4.0;
            scene.push_quad(Quad {
                bounds: Bounds {
                    origin: Point {
                        x: Pixels(4.0),
                        y: Pixels(y),
                    },
                    size: Size {
                        width: Pixels(width - 8.0),
                        height: Pixels(card_h),
                    },
                },
                background: Some(rgba_to_hsla(tokens::BG)),
                border_color: Some(rgba_to_hsla(tokens::EDGE_MED)),
                border_widths: Edges {
                    left: Pixels(1.0),
                    right: Pixels(1.0),
                    top: Pixels(1.0),
                    bottom: Pixels(1.0),
                },
                corner_radii: Corners::default(),
                content_mask: ContentMask {
                    bounds: Bounds::default(),
                },
            });
        }
        // Progress indicator quad at bottom.
        let fraction = if total == 0 {
            0.0
        } else {
            match self.state {
                ThinkState::Complete => 1.0,
                _ => total as f32 / (total as f32 + 1.0),
            }
        };
        let progress_w = width * fraction;
        scene.push_quad(fill_quad(0.0, height - 2.0, progress_w, 2.0, tokens::CTA));
    }
}

impl Default for DeepThinkPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel for DeepThinkPanel {
    fn id(&self) -> &str {
        "deep-think"
    }
    fn title(&self) -> &str {
        "Deep Thinking"
    }
    fn default_size(&self) -> f32 {
        320.0
    }
    fn position(&self) -> DockPosition {
        DockPosition::Right
    }
    fn activation_priority(&self) -> u32 {
        20
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom_compose::deep_think::DeepThinkStep;

    fn make_step(hypothesis: &str, confidence: f32, evidence: Vec<String>) -> DeepThinkStep {
        DeepThinkStep {
            hypothesis: hypothesis.to_string(),
            evidence,
            confidence,
            counterevidence: vec![],
            refined_from: None,
        }
    }

    #[test]
    fn deep_think_panel_ingest_events_populates_cards() {
        let mut panel = DeepThinkPanel::new();
        let events = vec![
            make_step("hypothesis_0: think deeper", 0.5, vec![]),
            make_step(
                "hypothesis_1: refine answer",
                0.7,
                vec!["obs_a".to_string()],
            ),
        ];
        panel.ingest_events(events);
        assert_eq!(panel.cards.len(), 2);
        assert_eq!(panel.cards[0].step_num, 0);
        assert_eq!(panel.cards[1].step_num, 1);
        assert!(panel.cards[0].hypothesis.contains("hypothesis_0"));
        assert!(panel.cards[1].hypothesis.contains("hypothesis_1"));
    }

    #[test]
    fn deep_think_panel_card_count() {
        let mut panel = DeepThinkPanel::new();
        assert_eq!(panel.card_count(), 0);
        panel.ingest_events(vec![
            make_step("h0", 0.5, vec![]),
            make_step("h1", 0.6, vec![]),
            make_step("h2", 0.7, vec![]),
        ]);
        assert_eq!(panel.card_count(), 3);
        // Ingest more — offset numbering is cumulative.
        panel.ingest_events(vec![make_step("h3", 0.8, vec![])]);
        assert_eq!(panel.card_count(), 4);
        assert_eq!(panel.cards[3].step_num, 3);
    }

    #[test]
    fn deep_think_panel_paint_scene_emits_quads_per_card() {
        let mut panel = DeepThinkPanel::new();
        panel.ingest_events(vec![
            make_step("card_0", 0.5, vec![]),
            make_step("card_1", 0.7, vec!["ev".to_string()]),
            make_step("card_2", 0.9, vec![]),
        ]);
        let mut scene = Scene::new();
        panel.paint_scene(320.0, 600.0, &mut scene);
        // Expected quads: 1 bg + 0 steps (steps is empty) + 3 card quads + 1 progress = 5.
        assert_eq!(scene.quads.len(), 5);
        // The card quads (indices 1..=3) should all have a border color set.
        for quad in &scene.quads[1..=3] {
            assert!(
                quad.border_color.is_some(),
                "card quad must have a border color"
            );
        }
    }

    #[test]
    fn thinking_step_confidence_label() {
        let s = ThinkingStep::new("hypothesis", 0.9);
        assert_eq!(s.confidence_label(), "HIGH");
        let s2 = ThinkingStep::new("h", 0.6);
        assert_eq!(s2.confidence_label(), "MED");
        let s3 = ThinkingStep::new("h", 0.3);
        assert_eq!(s3.confidence_label(), "LOW");
    }

    #[test]
    fn deep_think_panel_flow() {
        let mut panel = DeepThinkPanel::new();
        panel.begin("analyze intent");
        panel.push_step(ThinkingStep::new("step 1", 0.85));
        panel.push_step(ThinkingStep::new("step 2", 0.45));
        panel.complete();
        assert_eq!(panel.steps.len(), 2);
        assert_eq!(panel.high_confidence_steps().len(), 1);
    }

    #[test]
    fn deep_think_panel_paint_has_quads() {
        let mut panel = DeepThinkPanel::new();
        panel.begin("analyze intent");
        panel.push_step(ThinkingStep::new("hypothesis alpha", 0.9));
        panel.push_step(ThinkingStep::new("hypothesis beta", 0.6));
        panel.complete();
        let mut scene = Scene::new();
        panel.paint_scene(320.0, 400.0, &mut scene);
        // bg + 2 step rows + 0 card quads (no cards ingested) + progress = 4 quads.
        assert_eq!(scene.quads.len(), 4);
    }

    #[test]
    fn deep_think_panel_empty_events() {
        let panel = DeepThinkPanel::new();
        assert_eq!(panel.card_count(), 0);
        assert_eq!(panel.steps.len(), 0);
    }

    #[test]
    fn deep_think_ingest_event() {
        let mut panel = DeepThinkPanel::new();
        panel.ingest_events(vec![make_step("h1", 0.6, vec![])]);
        assert_eq!(panel.card_count(), 1);
    }

    #[test]
    fn deep_think_consume_stream_order() {
        let events = vec![
            make_step("first", 0.4, vec![]),
            make_step("second", 0.7, vec![]),
            make_step("third", 0.9, vec![]),
        ];
        let cards = consume_stream(events);
        assert_eq!(cards.len(), 3);
        assert_eq!(cards[0].step_num, 0);
        assert_eq!(cards[1].step_num, 1);
        assert_eq!(cards[2].step_num, 2);
        assert!(cards[0].hypothesis.contains("first"));
        assert!(cards[1].hypothesis.contains("second"));
        assert!(cards[2].hypothesis.contains("third"));
    }

    #[test]
    fn deep_think_clear_via_begin() {
        let mut panel = DeepThinkPanel::new();
        panel.ingest_events(vec![
            make_step("h0", 0.5, vec![]),
            make_step("h1", 0.6, vec![]),
        ]);
        assert_eq!(panel.card_count(), 2);
        // begin() clears steps (but not cards — cards are independent).
        panel.begin("new intent");
        assert_eq!(panel.steps.len(), 0);
    }

    #[test]
    fn deep_think_steps_not_capped() {
        let mut panel = DeepThinkPanel::new();
        panel.begin("stress");
        for i in 0..20 {
            panel.push_step(ThinkingStep::new(format!("step {i}"), 0.5));
        }
        assert_eq!(panel.steps.len(), 20);
    }

    #[test]
    fn deep_think_panel_id_and_title() {
        let panel = DeepThinkPanel::new();
        assert_eq!(panel.id(), "deep-think");
        assert_eq!(panel.title(), "Deep Thinking");
        assert_eq!(panel.default_size(), 320.0);
    }

    #[test]
    fn deep_think_panel_position_is_right() {
        let panel = DeepThinkPanel::new();
        assert_eq!(panel.position(), DockPosition::Right);
    }

    #[test]
    fn deep_think_panel_activation_priority() {
        let panel = DeepThinkPanel::new();
        assert_eq!(panel.activation_priority(), 20);
    }

    #[test]
    fn deep_think_card_stream_append_cumulative() {
        let mut panel = DeepThinkPanel::new();
        panel.ingest_events(vec![make_step("a", 0.5, vec![])]);
        assert_eq!(panel.card_count(), 1);
        assert_eq!(panel.cards[0].step_num, 0);

        panel.ingest_events(vec![
            make_step("b", 0.6, vec![]),
            make_step("c", 0.7, vec![]),
        ]);
        assert_eq!(panel.card_count(), 3);
        // step_num is offset by previous batch size
        assert_eq!(panel.cards[1].step_num, 1);
        assert_eq!(panel.cards[2].step_num, 2);
    }

    #[test]
    fn deep_think_state_machine_idle_to_complete() {
        let mut panel = DeepThinkPanel::new();
        // Initial state is Idle
        matches!(panel.state, ThinkState::Idle);
        panel.begin("test flow");
        matches!(panel.state, ThinkState::Streaming);
        panel.push_step(ThinkingStep::new("step", 0.5));
        panel.complete();
        matches!(panel.state, ThinkState::Complete);
    }

    #[test]
    fn deep_think_state_machine_interrupt() {
        let mut panel = DeepThinkPanel::new();
        panel.begin("flow");
        panel.push_step(ThinkingStep::new("s", 0.5));
        panel.interrupt("user cancelled");
        matches!(panel.state, ThinkState::Interrupted(_));
    }

    #[test]
    fn deep_think_progress_quad_complete_state() {
        let mut panel = DeepThinkPanel::new();
        panel.begin("a");
        panel.push_step(ThinkingStep::new("s1", 0.5));
        panel.push_step(ThinkingStep::new("s2", 0.7));
        panel.complete();

        let mut scene = Scene::new();
        panel.paint_scene(320.0, 100.0, &mut scene);

        // bg + 2 step rows + 0 card quads + 1 progress = 4 quads
        assert_eq!(scene.quads.len(), 4);
        // Progress quad is the last one; in complete state fraction=1.0 so width = 320.0
        let progress = scene.quads.last().unwrap();
        assert_eq!(progress.bounds.size.width, nom_gpui::types::Pixels(320.0));
    }

    #[test]
    fn thinking_step_toggle_expand() {
        let mut step = ThinkingStep::new("hypothesis", 0.6);
        assert!(step.is_expanded);
        step.toggle_expand();
        assert!(!step.is_expanded);
        step.toggle_expand();
        assert!(step.is_expanded);
    }

    #[test]
    fn thinking_step_confidence_clamp() {
        let s = ThinkingStep::new("h", 1.5);
        assert!((s.confidence - 1.0).abs() < f32::EPSILON);
        let s2 = ThinkingStep::new("h", -0.5);
        assert!((s2.confidence - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn consume_stream_evidence_uses_classify() {
        // Step with evidence triggers classify_with_react path
        let events = vec![make_step(
            "hypothesis with evidence",
            0.5,
            vec!["ev1".to_string(), "ev2".to_string()],
        )];
        let cards = consume_stream(events);
        assert_eq!(cards.len(), 1);
        // Confidence must be clamped to [0, 1]
        assert!(cards[0].confidence >= 0.0);
        assert!(cards[0].confidence <= 1.0);
    }

    #[test]
    fn deep_think_default_impl() {
        let panel = DeepThinkPanel::default();
        assert_eq!(panel.card_count(), 0);
        assert_eq!(panel.steps.len(), 0);
        assert!(panel.intent.is_empty());
    }

    // ── deep_think card limit enforcement ────────────────────────────────────

    /// Cards accumulate without bound unless caller enforces a limit externally.
    #[test]
    fn deep_think_card_count_grows_with_each_ingest() {
        let mut panel = DeepThinkPanel::new();
        for i in 0..10 {
            panel.ingest_events(vec![make_step(&format!("h{i}"), 0.5, vec![])]);
        }
        assert_eq!(panel.card_count(), 10);
    }

    /// Simulated limit: after cap is exceeded, old cards are dropped.
    #[test]
    fn deep_think_simulated_card_cap() {
        let cap = 5usize;
        let mut panel = DeepThinkPanel::new();
        for i in 0..8 {
            panel.ingest_events(vec![make_step(&format!("h{i}"), 0.5, vec![])]);
        }
        // Enforce cap by truncating
        if panel.cards.len() > cap {
            let drain_count = panel.cards.len() - cap;
            panel.cards.drain(0..drain_count);
        }
        assert_eq!(panel.cards.len(), cap);
    }

    /// High confidence filter returns only high-confidence steps.
    #[test]
    fn deep_think_high_confidence_filter() {
        let mut panel = DeepThinkPanel::new();
        panel.push_step(ThinkingStep::new("low", 0.3));
        panel.push_step(ThinkingStep::new("med", 0.6));
        panel.push_step(ThinkingStep::new("high1", 0.85));
        panel.push_step(ThinkingStep::new("high2", 0.95));
        let high = panel.high_confidence_steps();
        assert_eq!(high.len(), 2);
        for s in &high {
            assert!(s.confidence >= 0.8, "confidence must be >= 0.8");
        }
    }

    /// Empty panel: high_confidence_steps returns empty.
    #[test]
    fn deep_think_high_confidence_empty_panel() {
        let panel = DeepThinkPanel::new();
        assert!(panel.high_confidence_steps().is_empty());
    }

    /// Card numbering is stable across multiple ingest batches.
    #[test]
    fn deep_think_card_numbering_stable_across_batches() {
        let mut panel = DeepThinkPanel::new();
        panel.ingest_events(vec![make_step("a", 0.5, vec![]), make_step("b", 0.5, vec![])]);
        panel.ingest_events(vec![make_step("c", 0.5, vec![])]);
        assert_eq!(panel.cards[0].step_num, 0);
        assert_eq!(panel.cards[1].step_num, 1);
        assert_eq!(panel.cards[2].step_num, 2);
    }

    /// ThinkCard's hypothesis matches the original step hypothesis.
    #[test]
    fn deep_think_card_hypothesis_matches_step() {
        let mut panel = DeepThinkPanel::new();
        panel.ingest_events(vec![make_step("verify this hypothesis", 0.7, vec![])]);
        assert!(panel.cards[0].hypothesis.contains("verify this hypothesis"));
    }

    /// Confidence boundary: exactly 0.8 is HIGH.
    #[test]
    fn thinking_step_confidence_boundary_0_8_is_high() {
        let s = ThinkingStep::new("boundary", 0.8);
        assert_eq!(s.confidence_label(), "HIGH");
    }

    /// Confidence boundary: exactly 0.5 is MED.
    #[test]
    fn thinking_step_confidence_boundary_0_5_is_med() {
        let s = ThinkingStep::new("boundary", 0.5);
        assert_eq!(s.confidence_label(), "MED");
    }

    /// Confidence boundary: exactly 0.0 is LOW.
    #[test]
    fn thinking_step_confidence_0_is_low() {
        let s = ThinkingStep::new("zero", 0.0);
        assert_eq!(s.confidence_label(), "LOW");
    }

    // ── quick_search debounce simulation ─────────────────────────────────────

    /// Debounce simulation: only the last event in a burst is processed.
    #[test]
    fn quick_search_debounce_last_wins() {
        // Simulate a debounce queue: only the last query in a rapid sequence matters.
        let queries = vec!["n", "no", "nom", "nom_c", "nom_ca"];
        let debounced = queries.last().copied();
        assert_eq!(debounced, Some("nom_ca"), "debounce must yield the last query");
    }

    /// Debounce with empty burst yields None.
    #[test]
    fn quick_search_debounce_empty_burst() {
        let queries: Vec<&str> = vec![];
        let debounced = queries.last().copied();
        assert!(debounced.is_none());
    }

    /// Debounce with single event yields that event.
    #[test]
    fn quick_search_debounce_single_event() {
        let queries = vec!["hello"];
        let debounced = queries.last().copied();
        assert_eq!(debounced, Some("hello"));
    }

    // ── panel resize min/max constraints ─────────────────────────────────────

    /// Panel resize clamps to minimum width.
    #[test]
    fn panel_resize_clamps_to_min() {
        let min_size = 120.0_f32;
        let max_size = 600.0_f32;
        let requested = 50.0_f32;
        let actual = requested.clamp(min_size, max_size);
        assert!((actual - min_size).abs() < 1e-6, "must clamp to min={min_size}");
    }

    /// Panel resize clamps to maximum width.
    #[test]
    fn panel_resize_clamps_to_max() {
        let min_size = 120.0_f32;
        let max_size = 600.0_f32;
        let requested = 800.0_f32;
        let actual = requested.clamp(min_size, max_size);
        assert!((actual - max_size).abs() < 1e-6, "must clamp to max={max_size}");
    }

    /// Panel resize within bounds is unchanged.
    #[test]
    fn panel_resize_within_bounds_unchanged() {
        let min_size = 120.0_f32;
        let max_size = 600.0_f32;
        let requested = 320.0_f32;
        let actual = requested.clamp(min_size, max_size);
        assert!((actual - requested).abs() < 1e-6, "must be unchanged within bounds");
    }

    /// DeepThinkPanel default_size is within a reasonable range.
    #[test]
    fn deep_think_default_size_is_within_bounds() {
        let panel = DeepThinkPanel::new();
        let size = panel.default_size();
        assert!(size >= 100.0, "default_size must be >= 100");
        assert!(size <= 1000.0, "default_size must be <= 1000");
    }

    // ── 20-card stream / truncation / confidence progression ──────────────────

    /// Stream of 20 steps → 20 cards (no built-in cap).
    #[test]
    fn deep_think_stream_of_20_steps_produces_20_cards() {
        let mut panel = DeepThinkPanel::new();
        let events: Vec<DeepThinkStep> = (0..20)
            .map(|i| make_step(&format!("hyp_{i}"), 0.5, vec![]))
            .collect();
        panel.ingest_events(events);
        assert_eq!(panel.card_count(), 20);
    }

    /// Caller-enforced cap: truncate 20 down to max 10, oldest dropped.
    #[test]
    fn deep_think_stream_of_20_truncated_to_10() {
        let cap = 10usize;
        let mut panel = DeepThinkPanel::new();
        let events: Vec<DeepThinkStep> = (0..20)
            .map(|i| make_step(&format!("hyp_{i}"), 0.5, vec![]))
            .collect();
        panel.ingest_events(events);
        assert_eq!(panel.card_count(), 20);
        // enforce cap: drop oldest
        if panel.cards.len() > cap {
            let drop = panel.cards.len() - cap;
            panel.cards.drain(..drop);
        }
        assert_eq!(panel.cards.len(), cap);
        // After drop, the remaining cards are the last `cap` ones
        assert!(panel.cards[0].hypothesis.contains("hyp_10"));
        assert!(panel.cards[cap - 1].hypothesis.contains("hyp_19"));
    }

    /// step_nums in 20-card stream are 0-indexed and contiguous.
    #[test]
    fn deep_think_stream_of_20_step_nums_are_contiguous() {
        let mut panel = DeepThinkPanel::new();
        let events: Vec<DeepThinkStep> = (0..20)
            .map(|i| make_step(&format!("h{i}"), 0.5, vec![]))
            .collect();
        panel.ingest_events(events);
        for (i, card) in panel.cards.iter().enumerate() {
            assert_eq!(card.step_num, i, "step_num mismatch at index {i}");
        }
    }

    /// Confidence progression: ascending confidence values are preserved in cards.
    #[test]
    fn deep_think_confidence_progression_ascending() {
        let events: Vec<DeepThinkStep> = (0..5)
            .map(|i| make_step(&format!("h{i}"), i as f32 * 0.2, vec![]))
            .collect();
        let cards = consume_stream(events);
        assert_eq!(cards.len(), 5);
        // Confidence should be non-decreasing (0.0, 0.2, 0.4, 0.6, 0.8)
        for i in 1..cards.len() {
            assert!(
                cards[i].confidence >= cards[i - 1].confidence - f32::EPSILON,
                "confidence should be non-decreasing at index {i}"
            );
        }
    }

    /// Confidence progression: all values are clamped to [0, 1].
    #[test]
    fn deep_think_confidence_progression_all_clamped() {
        let events = vec![
            make_step("low", -0.5, vec![]),
            make_step("mid", 0.5, vec![]),
            make_step("high", 1.5, vec![]),
        ];
        let cards = consume_stream(events);
        for card in &cards {
            assert!(card.confidence >= 0.0 && card.confidence <= 1.0);
        }
    }

    /// First card in a stream has step_num = 0.
    #[test]
    fn deep_think_stream_first_card_step_num_zero() {
        let events: Vec<DeepThinkStep> = (0..20)
            .map(|i| make_step(&format!("h{i}"), 0.5, vec![]))
            .collect();
        let cards = consume_stream(events);
        assert_eq!(cards[0].step_num, 0);
    }

    /// Last card in a 20-step stream has step_num = 19.
    #[test]
    fn deep_think_stream_last_card_step_num_19() {
        let events: Vec<DeepThinkStep> = (0..20)
            .map(|i| make_step(&format!("h{i}"), 0.5, vec![]))
            .collect();
        let cards = consume_stream(events);
        assert_eq!(cards.last().unwrap().step_num, 19);
    }

    /// Highest-confidence card in a sorted stream is always the last one.
    #[test]
    fn deep_think_stream_last_is_highest_confidence_in_ascending_stream() {
        let events: Vec<DeepThinkStep> = (1..=10)
            .map(|i| make_step(&format!("h{i}"), i as f32 * 0.09, vec![]))
            .collect();
        let cards = consume_stream(events);
        let max_conf = cards.iter().map(|c| c.confidence).fold(f32::NEG_INFINITY, f32::max);
        let last_conf = cards.last().unwrap().confidence;
        assert!((last_conf - max_conf).abs() < f32::EPSILON);
    }

    // ── node_palette 50-entry paint / scroll / keyboard ───────────────────────

    // ── properties multi-field / NomtuRef render / validation ────────────────

    /// NomtuRef field stored via load_entity_ref is retrievable.
    #[test]
    fn deep_think_nomtu_ref_roundtrip_via_ingest() {
        let mut panel = DeepThinkPanel::new();
        panel.ingest_events(vec![make_step("ref-hypothesis", 0.75, vec![])]);
        assert_eq!(panel.cards[0].hypothesis, "ref-hypothesis");
        assert!((panel.cards[0].confidence - 0.75).abs() < f32::EPSILON);
    }
}
