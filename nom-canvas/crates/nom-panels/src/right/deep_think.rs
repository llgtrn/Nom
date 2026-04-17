#![deny(unsafe_code)]
use crate::dock::{DockPosition, Panel};
use crate::right::chat_sidebar::RenderPrimitive;

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

    pub fn render_bounds(&self, width: f32, height: f32) -> Vec<RenderPrimitive> {
        let mut out = Vec::new();
        // Panel background
        out.push(RenderPrimitive::Rect { x: 0.0, y: 0.0, w: width, h: height, color: 0x1e1e2e });
        let total = self.steps.len();
        for (i, step) in self.steps.iter().enumerate() {
            let y = i as f32 * 24.0 + 4.0;
            let is_active = i + 1 == total;
            if is_active {
                out.push(RenderPrimitive::Rect { x: 0.0, y, w: width, h: 22.0, color: 0x313244 });
            }
            out.push(RenderPrimitive::Text {
                x: 6.0,
                y: y + 6.0,
                text: format!("{}. {}", i + 1, step.hypothesis),
                size: 12.0,
                color: 0xcdd6f4,
            });
        }
        // Progress indicator line at bottom
        let fraction = if total == 0 { 0.0 } else {
            match self.state {
                ThinkState::Complete => 1.0,
                _ => total as f32 / (total as f32 + 1.0),
            }
        };
        let line_end_x = width * fraction;
        out.push(RenderPrimitive::Line { x1: 0.0, y1: height - 2.0, x2: line_end_x, y2: height - 2.0, color: 0x89b4fa });
        out
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

    #[test]
    fn deep_think_panel_render_has_steps() {
        let mut panel = DeepThinkPanel::new();
        panel.begin("analyze intent");
        panel.push_step(ThinkingStep::new("hypothesis alpha", 0.9));
        panel.push_step(ThinkingStep::new("hypothesis beta", 0.6));
        panel.complete();
        let prims = panel.render_bounds(320.0, 400.0);
        // Must have: 1 bg + 1 active highlight + 2 text + 1 progress line = 5 minimum
        assert!(prims.len() >= 5, "expected at least 5 primitives, got {}", prims.len());
        // Background is first
        assert_eq!(prims[0], RenderPrimitive::Rect { x: 0.0, y: 0.0, w: 320.0, h: 400.0, color: 0x1e1e2e });
        // Active step highlight (last step uses 0x313244)
        let has_highlight = prims.iter().any(|p| matches!(p, RenderPrimitive::Rect { color: 0x313244, .. }));
        assert!(has_highlight, "active step highlight not found");
        // Progress line present at bottom
        let has_line = prims.iter().any(|p| matches!(p, RenderPrimitive::Line { .. }));
        assert!(has_line, "progress line not found");
        // Text prims contain step numbers
        let text_contents: Vec<&str> = prims.iter().filter_map(|p| {
            if let RenderPrimitive::Text { text, .. } = p { Some(text.as_str()) } else { None }
        }).collect();
        assert!(text_contents.iter().any(|t| t.contains("1.")), "step 1 text not found");
        assert!(text_contents.iter().any(|t| t.contains("2.")), "step 2 text not found");
    }
}
