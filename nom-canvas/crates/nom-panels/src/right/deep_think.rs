#![deny(unsafe_code)]
use crate::dock::{DockPosition, Panel};

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
    pub state: ThinkState,
    pub intent: String,
}

impl DeepThinkPanel {
    pub fn new() -> Self {
        Self { steps: vec![], state: ThinkState::Idle, intent: String::new() }
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
}
