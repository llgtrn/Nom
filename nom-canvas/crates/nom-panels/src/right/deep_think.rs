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
    pub confidence: f32,  // 0.0–1.0
    pub counterevidence: Vec<String>,
    pub refined_from: Option<String>,
    pub is_expanded: bool,
}

impl ThinkingStep {
    pub fn new(hypothesis: impl Into<String>, confidence: f32) -> Self {
        Self { hypothesis: hypothesis.into(), evidence: vec![], confidence: confidence.clamp(0.0, 1.0), counterevidence: vec![], refined_from: None, is_expanded: true }
    }

    pub fn confidence_label(&self) -> &'static str {
        if self.confidence >= 0.8 { "HIGH" }
        else if self.confidence >= 0.5 { "MED" }
        else { "LOW" }
    }

    pub fn toggle_expand(&mut self) { self.is_expanded = !self.is_expanded; }
}

pub enum ThinkState { Idle, Streaming, Complete, Interrupted(String) }

pub struct DeepThinkPanel {
    pub steps: Vec<ThinkingStep>,
    pub cards: Vec<ThinkCard>,
    pub state: ThinkState,
    pub intent: String,
}

impl DeepThinkPanel {
    pub fn new() -> Self {
        Self { steps: vec![], cards: vec![], state: ThinkState::Idle, intent: String::new() }
    }

    pub fn begin(&mut self, intent: impl Into<String>) {
        self.intent = intent.into();
        self.steps.clear();
        self.state = ThinkState::Streaming;
    }

    pub fn push_step(&mut self, step: ThinkingStep) {
        self.steps.push(step);
    }

    pub fn complete(&mut self) { self.state = ThinkState::Complete; }
    pub fn interrupt(&mut self, reason: impl Into<String>) { self.state = ThinkState::Interrupted(reason.into()); }

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
            let color = if is_active { tokens::FOCUS } else { tokens::BG2 };
            scene.push_quad(fill_quad(0.0, y, width, 22.0, color));
        }
        // One Quad per ThinkCard — stacked vertically with EDGE_MED border.
        let card_h = 40.0;
        let card_margin = 4.0;
        for (i, _card) in self.cards.iter().enumerate() {
            let y = i as f32 * (card_h + card_margin) + 4.0;
            scene.push_quad(Quad {
                bounds: Bounds {
                    origin: Point { x: Pixels(4.0), y: Pixels(y) },
                    size: Size { width: Pixels(width - 8.0), height: Pixels(card_h) },
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
                content_mask: ContentMask { bounds: Bounds::default() },
            });
        }
        // Progress indicator quad at bottom.
        let fraction = if total == 0 { 0.0 } else {
            match self.state {
                ThinkState::Complete => 1.0,
                _ => total as f32 / (total as f32 + 1.0),
            }
        };
        let progress_w = width * fraction;
        scene.push_quad(fill_quad(0.0, height - 2.0, progress_w, 2.0, tokens::CTA));
    }
}

impl Default for DeepThinkPanel { fn default() -> Self { Self::new() } }

impl Panel for DeepThinkPanel {
    fn id(&self) -> &str { "deep-think" }
    fn title(&self) -> &str { "Deep Thinking" }
    fn default_size(&self) -> f32 { 320.0 }
    fn position(&self) -> DockPosition { DockPosition::Right }
    fn activation_priority(&self) -> u32 { 20 }
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
            make_step("hypothesis_1: refine answer", 0.7, vec!["obs_a".to_string()]),
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
            assert!(quad.border_color.is_some(), "card quad must have a border color");
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
}
