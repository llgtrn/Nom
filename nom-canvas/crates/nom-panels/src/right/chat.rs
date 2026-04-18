#![deny(unsafe_code)]

// ---------------------------------------------------------------------------
// ChatRole — who sent the message
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatRole {
    User,
    Assistant,
    System,
}

// ---------------------------------------------------------------------------
// ChatAttachment — payloads that can accompany a message
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatAttachment {
    ImageBytes { name: String, bytes: Vec<u8> },
    WebUrl(String),
    FilePath(String),
    NomxSource(String),
}

impl ChatAttachment {
    /// Human-readable kind label used in response stubs.
    pub fn kind_label(&self) -> &'static str {
        match self {
            ChatAttachment::ImageBytes { .. } => "image",
            ChatAttachment::WebUrl(_) => "web-url",
            ChatAttachment::FilePath(_) => "file",
            ChatAttachment::NomxSource(_) => "nomx-source",
        }
    }
}

// ---------------------------------------------------------------------------
// ChatMessage — a single turn in the conversation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    pub attachments: Vec<ChatAttachment>,
    pub timestamp_ms: u64,
}

impl ChatMessage {
    pub fn new_user(content: impl Into<String>, attachments: Vec<ChatAttachment>) -> Self {
        Self {
            role: ChatRole::User,
            content: content.into(),
            attachments,
            timestamp_ms: 0,
        }
    }

    pub fn new_assistant(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::Assistant,
            content: content.into(),
            attachments: vec![],
            timestamp_ms: 0,
        }
    }

    pub fn has_attachments(&self) -> bool {
        !self.attachments.is_empty()
    }
}

// ---------------------------------------------------------------------------
// CanvasMode — which center-panel mode is active
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanvasMode {
    /// Code / .nomx editing.
    Editor,
    /// Visual graph / node editing.
    Canvas,
    /// Render output preview.
    Preview,
    /// Document / markdown view.
    Document,
    /// Media composition.
    Compose,
}

// ---------------------------------------------------------------------------
// ChatDispatch — routes messages to the correct CanvasMode
// ---------------------------------------------------------------------------

pub struct ChatDispatch;

impl ChatDispatch {
    /// Infer the appropriate `CanvasMode` from the message content.
    pub fn infer_mode(message: &ChatMessage) -> CanvasMode {
        let lc = message.content.to_lowercase();
        if lc.contains("canvas") || lc.contains("graph") {
            CanvasMode::Canvas
        } else if lc.contains("preview") || lc.contains("render") {
            CanvasMode::Preview
        } else if lc.contains("document") || lc.contains(" doc") || lc.contains("write") {
            CanvasMode::Document
        } else if lc.contains("compose") || lc.contains("video") || lc.contains("image") {
            CanvasMode::Compose
        } else {
            CanvasMode::Editor
        }
    }

    /// Dispatch a message: returns `(inferred_mode, response_stub)`.
    pub fn dispatch(message: ChatMessage) -> (CanvasMode, String) {
        let mode = Self::infer_mode(&message);
        let response = if message.has_attachments() {
            let kind = message.attachments[0].kind_label();
            format!("Processing attachment: {} → generating nomx...", kind)
        } else {
            format!(
                "Understood: {} → switching to {:?} mode",
                message.content, mode
            )
        };
        (mode, response)
    }
}

// ---------------------------------------------------------------------------
// AiChatSession — stateful conversation with mode tracking
// ---------------------------------------------------------------------------

pub struct AiChatSession {
    pub messages: Vec<ChatMessage>,
    pub current_mode: CanvasMode,
}

impl AiChatSession {
    pub fn new() -> Self {
        Self {
            messages: vec![],
            current_mode: CanvasMode::Editor,
        }
    }

    /// Submit a user message, update the current mode, and record both the
    /// user message and the assistant response.  Returns `(new_mode, response)`.
    pub fn submit(&mut self, message: ChatMessage) -> (CanvasMode, String) {
        let (mode, response) = ChatDispatch::dispatch(message.clone());
        self.current_mode = mode.clone();
        self.messages.push(message);
        self.messages.push(ChatMessage::new_assistant(&response));
        (mode, response)
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    pub fn current_mode(&self) -> &CanvasMode {
        &self.current_mode
    }
}

impl Default for AiChatSession {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// ChatPanel — thin UI wrapper kept for backward compatibility
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- original ChatPanel tests (preserved) --------------------------------

    #[test]
    fn chat_panel_new_is_empty() {
        let panel = ChatPanel::new();
        assert_eq!(panel.message_count(), 0);
        assert!(panel.input_draft.is_empty());
    }

    #[test]
    fn chat_panel_push_message() {
        let msg = ChatMessage::new_user("hello", vec![]);
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
            .push_message(ChatMessage::new_user("question", vec![]))
            .push_message(ChatMessage::new_assistant("answer"))
            .push_message(ChatMessage::new_user("follow-up", vec![]));

        let last = panel.last_assistant_message();
        assert!(last.is_some());
        assert_eq!(last.unwrap().content, "answer");

        let empty = ChatPanel::new();
        assert!(empty.last_assistant_message().is_none());
    }

    // -- new tests -----------------------------------------------------------

    #[test]
    fn chat_message_user() {
        let msg = ChatMessage::new_user("hello world", vec![]);
        assert_eq!(msg.role, ChatRole::User);
        assert_eq!(msg.content, "hello world");
        assert!(msg.attachments.is_empty());
    }

    #[test]
    fn chat_message_has_attachments() {
        let att = ChatAttachment::FilePath("/tmp/code.nomx".to_string());
        let msg = ChatMessage::new_user("attach this", vec![att]);
        assert!(msg.has_attachments());
        assert_eq!(msg.attachments.len(), 1);
    }

    #[test]
    fn chat_message_no_attachments() {
        let msg = ChatMessage::new_assistant("I understand");
        assert!(!msg.has_attachments());
    }

    #[test]
    fn infer_mode_canvas() {
        let msg = ChatMessage::new_user("show me the graph view", vec![]);
        assert_eq!(ChatDispatch::infer_mode(&msg), CanvasMode::Canvas);

        let msg2 = ChatMessage::new_user("open canvas editor", vec![]);
        assert_eq!(ChatDispatch::infer_mode(&msg2), CanvasMode::Canvas);
    }

    #[test]
    fn infer_mode_compose() {
        let msg = ChatMessage::new_user("compose a video clip", vec![]);
        assert_eq!(ChatDispatch::infer_mode(&msg), CanvasMode::Compose);

        let msg2 = ChatMessage::new_user("import image assets", vec![]);
        assert_eq!(ChatDispatch::infer_mode(&msg2), CanvasMode::Compose);
    }

    #[test]
    fn infer_mode_document() {
        let msg = ChatMessage::new_user("write a document about this", vec![]);
        assert_eq!(ChatDispatch::infer_mode(&msg), CanvasMode::Document);
    }

    #[test]
    fn infer_mode_editor_default() {
        let msg = ChatMessage::new_user("fix the function signature", vec![]);
        assert_eq!(ChatDispatch::infer_mode(&msg), CanvasMode::Editor);
    }

    #[test]
    fn dispatch_with_attachment() {
        let att = ChatAttachment::NomxSource("entry { }".to_string());
        let msg = ChatMessage::new_user("process this nomx", vec![att]);
        let (_mode, response) = ChatDispatch::dispatch(msg);
        assert!(response.contains("nomx-source"));
        assert!(response.contains("generating nomx"));
    }

    #[test]
    fn session_submit_changes_mode() {
        let mut session = AiChatSession::new();
        assert_eq!(session.current_mode(), &CanvasMode::Editor);

        let msg = ChatMessage::new_user("open the canvas view", vec![]);
        let (mode, _) = session.submit(msg);
        assert_eq!(mode, CanvasMode::Canvas);
        assert_eq!(session.current_mode(), &CanvasMode::Canvas);
    }

    #[test]
    fn session_message_count() {
        let mut session = AiChatSession::new();
        assert_eq!(session.message_count(), 0);

        session.submit(ChatMessage::new_user("first message", vec![]));
        // user message + assistant response = 2
        assert_eq!(session.message_count(), 2);

        session.submit(ChatMessage::new_user("second message", vec![]));
        assert_eq!(session.message_count(), 4);
    }
}
