#![deny(unsafe_code)]
use crate::dock::{DockPosition, Panel};

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
}
