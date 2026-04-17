#![deny(unsafe_code)]
use crate::dock::{fill_quad, DockPosition, Panel};
use nom_gpui::scene::Scene;
use nom_theme::tokens;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatRole {
    User,
    Assistant,
    System,
    Tool,
}

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
        Self {
            tool_name: tool_name.into(),
            input_summary: input_summary.into(),
            output_summary: None,
            duration_ms: None,
            is_expanded: false,
        }
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
        Self {
            id: id.into(),
            role: ChatRole::User,
            content: content.into(),
            tool_cards: vec![],
            is_streaming: false,
        }
    }

    pub fn assistant_streaming(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            role: ChatRole::Assistant,
            content: String::new(),
            tool_cards: vec![],
            is_streaming: true,
        }
    }

    pub fn append_delta(&mut self, delta: &str) {
        if self.is_streaming {
            self.content.push_str(delta);
        }
    }

    pub fn finalize(&mut self) {
        self.is_streaming = false;
    }
}

pub struct ChatSidebarPanel {
    pub messages: Vec<ChatMessage>,
    pub pending_tool: Option<ToolCard>,
    pub scroll_to_bottom: bool,
}

impl ChatSidebarPanel {
    pub fn new() -> Self {
        Self {
            messages: vec![],
            pending_tool: None,
            scroll_to_bottom: false,
        }
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

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    pub fn paint_scene(&self, width: f32, height: f32, scene: &mut Scene) {
        // Panel background.
        scene.push_quad(fill_quad(0.0, 0.0, width, height, tokens::BG));

        for (i, msg) in self.messages.iter().enumerate() {
            let y = i as f32 * 60.0 + 8.0;
            let bubble_w = width * 0.75;
            let (bx, color) = match msg.role {
                ChatRole::User => (width - bubble_w - 8.0, tokens::CTA),
                _ => (8.0, tokens::BG2),
            };
            scene.push_quad(fill_quad(bx, y, bubble_w, 44.0, color));

            // Tool-card strips beneath the message.
            for (j, _card) in msg.tool_cards.iter().enumerate() {
                let card_y = y + 46.0 + j as f32 * 22.0;
                scene.push_quad(fill_quad(8.0, card_y, width - 16.0, 20.0, tokens::BORDER));
            }
        }
    }
}

impl Default for ChatSidebarPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel for ChatSidebarPanel {
    fn id(&self) -> &str {
        "chat-sidebar"
    }
    fn title(&self) -> &str {
        "Assistant"
    }
    fn default_size(&self) -> f32 {
        320.0
    }
    fn position(&self) -> DockPosition {
        DockPosition::Right
    }
    fn activation_priority(&self) -> u32 {
        10
    }
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
    fn chat_sidebar_new_empty() {
        let panel = ChatSidebarPanel::new();
        assert_eq!(panel.message_count(), 0);
    }

    #[test]
    fn chat_sidebar_add_message() {
        let mut panel = ChatSidebarPanel::new();
        panel.push_message(ChatMessage::user("u1", "hello"));
        assert_eq!(panel.message_count(), 1);
    }

    #[test]
    fn chat_sidebar_message_role() {
        let mut panel = ChatSidebarPanel::new();
        panel.push_message(ChatMessage::user("u1", "hello"));
        panel.push_message(ChatMessage::assistant_streaming("a1"));
        assert_eq!(panel.messages[0].role, ChatRole::User);
        assert_eq!(panel.messages[1].role, ChatRole::Assistant);
    }

    #[test]
    fn chat_sidebar_clear() {
        let mut panel = ChatSidebarPanel::new();
        panel.push_message(ChatMessage::user("u1", "hello"));
        panel.push_message(ChatMessage::user("u2", "world"));
        panel.messages.clear();
        assert_eq!(panel.message_count(), 0);
    }

    #[test]
    fn chat_panel_paint_bubbles() {
        let mut panel = ChatSidebarPanel::new();
        panel.push_message(ChatMessage::user("u1", "hello"));
        panel.push_message(ChatMessage::assistant_streaming("a1"));
        panel.append_to_last(" world");
        panel.finalize_last();
        let mut scene = Scene::new();
        panel.paint_scene(320.0, 600.0, &mut scene);
        // bg + 2 bubble quads.
        assert_eq!(scene.quads.len(), 3);
        let bg = &scene.quads[0];
        assert_eq!(bg.bounds.size.width, nom_gpui::types::Pixels(320.0));
        assert_eq!(bg.bounds.size.height, nom_gpui::types::Pixels(600.0));
    }
}
