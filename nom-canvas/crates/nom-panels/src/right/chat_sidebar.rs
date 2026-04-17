#![deny(unsafe_code)]
use crate::dock::{DockPosition, Panel};
pub use crate::dock::RenderPrimitive;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatRole { User, Assistant, System, Tool }

#[derive(Debug, Clone)]
pub struct ToolCard {
    pub tool_name: String,
    pub input_summary: String,
    pub output_summary: Option<String>,
    pub duration_ms: Option<u64>,
    pub is_expanded: bool,
}

impl ToolCard {
    pub fn new(tool_name: impl Into<String>, input_summary: impl Into<String>) -> Self {
        Self { tool_name: tool_name.into(), input_summary: input_summary.into(), output_summary: None, duration_ms: None, is_expanded: false }
    }

    pub fn complete(&mut self, output: impl Into<String>, duration_ms: u64) {
        self.output_summary = Some(output.into());
        self.duration_ms = Some(duration_ms);
    }
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub id: String,
    pub role: ChatRole,
    pub content: String,
    pub tool_cards: Vec<ToolCard>,
    pub is_streaming: bool,
}

impl ChatMessage {
    pub fn user(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self { id: id.into(), role: ChatRole::User, content: content.into(), tool_cards: vec![], is_streaming: false }
    }

    pub fn assistant_streaming(id: impl Into<String>) -> Self {
        Self { id: id.into(), role: ChatRole::Assistant, content: String::new(), tool_cards: vec![], is_streaming: true }
    }

    pub fn append_delta(&mut self, delta: &str) {
        if self.is_streaming { self.content.push_str(delta); }
    }

    pub fn finalize(&mut self) { self.is_streaming = false; }
}

pub struct ChatSidebarPanel {
    pub messages: Vec<ChatMessage>,
    pub pending_tool: Option<ToolCard>,
    pub scroll_to_bottom: bool,
}

impl ChatSidebarPanel {
    pub fn new() -> Self {
        Self { messages: vec![], pending_tool: None, scroll_to_bottom: false }
    }

    pub fn push_message(&mut self, msg: ChatMessage) {
        self.messages.push(msg);
        self.scroll_to_bottom = true;
    }

    pub fn append_to_last(&mut self, delta: &str) {
        if let Some(last) = self.messages.last_mut() {
            last.append_delta(delta);
        }
    }

    pub fn finalize_last(&mut self) {
        if let Some(last) = self.messages.last_mut() {
            last.finalize();
        }
    }

    pub fn begin_tool(&mut self, tool_name: impl Into<String>, input_summary: impl Into<String>) {
        self.pending_tool = Some(ToolCard::new(tool_name, input_summary));
    }

    pub fn complete_tool(&mut self, output: impl Into<String>, duration_ms: u64) {
        if let Some(mut card) = self.pending_tool.take() {
            card.complete(output, duration_ms);
            if let Some(last) = self.messages.last_mut() {
                last.tool_cards.push(card);
            }
        }
    }

    pub fn message_count(&self) -> usize { self.messages.len() }

    pub fn render_bounds(&self, width: f32, height: f32) -> Vec<RenderPrimitive> {
        let mut out = Vec::new();
        // Panel background
        out.push(RenderPrimitive::Rect { x: 0.0, y: 0.0, w: width, h: height, color: 0x1e1e2e });
        for (i, msg) in self.messages.iter().enumerate() {
            let y = i as f32 * 60.0 + 8.0;
            let bubble_w = width * 0.75;
            let (bx, bg_color, text_color) = match msg.role {
                ChatRole::User => (width - bubble_w - 8.0, 0x89b4fa_u32, 0x1e1e2e_u32),
                _ => (8.0, 0x313244_u32, 0xcdd6f4_u32),
            };
            out.push(RenderPrimitive::Rect { x: bx, y, w: bubble_w, h: 44.0, color: bg_color });
            out.push(RenderPrimitive::Text {
                x: bx + 6.0,
                y: y + 14.0,
                text: msg.content.clone(),
                size: 13.0,
                color: text_color,
            });
            for card in &msg.tool_cards {
                let card_y = y + 46.0;
                out.push(RenderPrimitive::Rect { x: 8.0, y: card_y, w: width - 16.0, h: 20.0, color: 0x45475a });
                out.push(RenderPrimitive::Text {
                    x: 12.0,
                    y: card_y + 6.0,
                    text: format!("[tool: {}]", card.tool_name),
                    size: 11.0,
                    color: 0xcdd6f4,
                });
            }
        }
        out
    }
}

impl Default for ChatSidebarPanel { fn default() -> Self { Self::new() } }

impl Panel for ChatSidebarPanel {
    fn id(&self) -> &str { "chat-sidebar" }
    fn title(&self) -> &str { "Assistant" }
    fn default_size(&self) -> f32 { 320.0 }
    fn position(&self) -> DockPosition { DockPosition::Right }
    fn activation_priority(&self) -> u32 { 10 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_message_streaming() {
        let mut msg = ChatMessage::assistant_streaming("m1");
        assert!(msg.is_streaming);
        msg.append_delta("Hello");
        msg.append_delta(" world");
        msg.finalize();
        assert_eq!(msg.content, "Hello world");
        assert!(!msg.is_streaming);
    }

    #[test]
    fn tool_card_lifecycle() {
        let mut panel = ChatSidebarPanel::new();
        panel.push_message(ChatMessage::user("u1", "analyze this"));
        panel.push_message(ChatMessage::assistant_streaming("a1"));
        panel.begin_tool("stage1_tokenize", "source.nom");
        panel.complete_tool("17 tokens", 12);
        let last = panel.messages.last().unwrap();
        assert_eq!(last.tool_cards.len(), 1);
        assert_eq!(last.tool_cards[0].tool_name, "stage1_tokenize");
    }

    #[test]
    fn chat_panel_render_returns_messages() {
        let mut panel = ChatSidebarPanel::new();
        panel.push_message(ChatMessage::user("u1", "hello"));
        panel.push_message(ChatMessage::assistant_streaming("a1"));
        // finalize so content is set
        panel.append_to_last(" world");
        panel.finalize_last();
        let prims = panel.render_bounds(320.0, 600.0);
        // Must have at least: 1 bg + 2 bubble rects + 2 text prims
        assert!(prims.len() >= 5, "expected at least 5 primitives, got {}", prims.len());
        // First primitive is the background rect
        assert_eq!(prims[0], RenderPrimitive::Rect { x: 0.0, y: 0.0, w: 320.0, h: 600.0, color: 0x1e1e2e });
        // User bubble uses 0x89b4fa
        let has_user_bubble = prims.iter().any(|p| matches!(p, RenderPrimitive::Rect { color: 0x89b4fa, .. }));
        assert!(has_user_bubble, "user bubble not found");
        // Assistant bubble uses 0x313244
        let has_asst_bubble = prims.iter().any(|p| matches!(p, RenderPrimitive::Rect { color: 0x313244, .. }));
        assert!(has_asst_bubble, "assistant bubble not found");
    }
}
