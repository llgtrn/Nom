#![deny(unsafe_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatRole {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    pub timestamp_ms: u64,
}

pub struct ChatPanel {
    pub messages: Vec<ChatMessage>,
    pub input_draft: String,
}

impl ChatPanel {
    pub fn new() -> Self {
        Self {
            messages: vec![],
            input_draft: String::new(),
        }
    }

    pub fn push_message(mut self, msg: ChatMessage) -> Self {
        self.messages.push(msg);
        self
    }

    pub fn set_draft(mut self, text: &str) -> Self {
        self.input_draft = text.to_string();
        self
    }

    pub fn clear_draft(mut self) -> Self {
        self.input_draft.clear();
        self
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    pub fn last_assistant_message(&self) -> Option<&ChatMessage> {
        self.messages
            .iter()
            .rev()
            .find(|m| m.role == ChatRole::Assistant)
    }
}

impl Default for ChatPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_panel_new_is_empty() {
        let panel = ChatPanel::new();
        assert_eq!(panel.message_count(), 0);
        assert!(panel.input_draft.is_empty());
    }

    #[test]
    fn chat_panel_push_message() {
        let msg = ChatMessage {
            role: ChatRole::User,
            content: "hello".to_string(),
            timestamp_ms: 1000,
        };
        let panel = ChatPanel::new().push_message(msg);
        assert_eq!(panel.message_count(), 1);
        assert_eq!(panel.messages[0].content, "hello");
    }

    #[test]
    fn chat_panel_draft_lifecycle() {
        let panel = ChatPanel::new().set_draft("draft text").clear_draft();
        assert!(panel.input_draft.is_empty());

        let panel2 = ChatPanel::new().set_draft("keep this");
        assert_eq!(panel2.input_draft, "keep this");
    }

    #[test]
    fn chat_panel_last_assistant_message() {
        let panel = ChatPanel::new()
            .push_message(ChatMessage {
                role: ChatRole::User,
                content: "question".to_string(),
                timestamp_ms: 100,
            })
            .push_message(ChatMessage {
                role: ChatRole::Assistant,
                content: "answer".to_string(),
                timestamp_ms: 200,
            })
            .push_message(ChatMessage {
                role: ChatRole::User,
                content: "follow-up".to_string(),
                timestamp_ms: 300,
            });

        let last = panel.last_assistant_message();
        assert!(last.is_some());
        assert_eq!(last.unwrap().content, "answer");

        let empty = ChatPanel::new();
        assert!(empty.last_assistant_message().is_none());
    }
}
