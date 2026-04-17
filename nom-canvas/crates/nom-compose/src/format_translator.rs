//! Unified chat-message schema + translators for multi-vendor LLM APIs.
//!
//! Variants covered (wire format only — no HTTP):
//!   - Anthropic messages v1 (role + content blocks, system at top-level)
//!   - OpenAI chat completions v1 (role + content + optional tool_calls)
//!   - Google Gemini generate_content v1 (role + parts)
//!
//! Identifiers use neutral names; provider-specific strings only appear inside
//! the encode/decode helpers.
#![deny(unsafe_code)]

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProviderFormat {
    Anthropic,
    Openai,
    Google,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

impl Role {
    pub fn as_anthropic(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::User => "user",
            Self::Assistant => "assistant",
            // Anthropic merges tool results under user
            Self::Tool => "user",
        }
    }

    pub fn as_openai(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::Tool => "tool",
        }
    }

    pub fn as_google(self) -> &'static str {
        match self {
            // Google prepends instructions to the first user turn
            Self::System => "user",
            Self::User => "user",
            Self::Assistant => "model",
            Self::Tool => "function",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments_json: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnifiedMessage {
    pub role: Role,
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
}

impl UnifiedMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self { role: Role::System, content: content.into(), tool_calls: vec![] }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self { role: Role::User, content: content.into(), tool_calls: vec![] }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self { role: Role::Assistant, content: content.into(), tool_calls: vec![] }
    }

    pub fn assistant_with_tools(content: impl Into<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self { role: Role::Assistant, content: content.into(), tool_calls }
    }

    pub fn tool(call_id: impl Into<String>, output: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: format!("[tool:{}]{}", call_id.into(), output.into()),
            tool_calls: vec![],
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TranslationError {
    #[error("cannot encode message with role {0:?} for {1:?}")]
    UnsupportedRole(Role, ProviderFormat),
    #[error("empty message history")]
    EmptyHistory,
    #[error("malformed tool_call at index {0}")]
    MalformedToolCall(usize),
}

pub struct FormatTranslator;

impl FormatTranslator {
    /// Translate a message history from unified → provider wire strings.
    /// Returns a Vec of `(role_string, serialized_content)` pairs.
    pub fn encode(
        messages: &[UnifiedMessage],
        target: ProviderFormat,
    ) -> Result<Vec<(String, String)>, TranslationError> {
        if messages.is_empty() {
            return Err(TranslationError::EmptyHistory);
        }
        let mut out = Vec::with_capacity(messages.len());
        for (i, m) in messages.iter().enumerate() {
            let role_str = match target {
                ProviderFormat::Anthropic => m.role.as_anthropic(),
                ProviderFormat::Openai => m.role.as_openai(),
                ProviderFormat::Google => m.role.as_google(),
            };
            // Encode tool_calls as JSON-ish suffix on content for the unified
            // shape — each provider actually serialises this differently but
            // for the translator's contract we use a canonical inline form.
            let mut content = m.content.clone();
            if !m.tool_calls.is_empty() {
                for (j, tc) in m.tool_calls.iter().enumerate() {
                    if tc.name.is_empty() {
                        return Err(TranslationError::MalformedToolCall(i * 1000 + j));
                    }
                    content.push_str(&format!(
                        "\n[tool_call:{}:{}:{}]",
                        tc.id, tc.name, tc.arguments_json
                    ));
                }
            }
            out.push((role_str.to_string(), content));
        }
        Ok(out)
    }

    /// Inverse (best-effort) — used primarily for round-trip property tests.
    pub fn decode(pairs: &[(String, String)], source: ProviderFormat) -> Vec<UnifiedMessage> {
        pairs
            .iter()
            .map(|(role_str, content)| {
                let role = match source {
                    ProviderFormat::Anthropic => match role_str.as_str() {
                        "system" => Role::System,
                        "assistant" => Role::Assistant,
                        _ => Role::User,
                    },
                    ProviderFormat::Openai => match role_str.as_str() {
                        "system" => Role::System,
                        "assistant" => Role::Assistant,
                        "tool" => Role::Tool,
                        _ => Role::User,
                    },
                    ProviderFormat::Google => match role_str.as_str() {
                        "model" => Role::Assistant,
                        "function" => Role::Tool,
                        _ => Role::User,
                    },
                };
                UnifiedMessage { role, content: content.clone(), tool_calls: vec![] }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_empty_returns_empty_history_error() {
        let result = FormatTranslator::encode(&[], ProviderFormat::Openai);
        assert!(matches!(result, Err(TranslationError::EmptyHistory)));
    }

    #[test]
    fn encode_system_user_assistant_to_openai() {
        let messages = vec![
            UnifiedMessage::system("You are helpful."),
            UnifiedMessage::user("Hello"),
            UnifiedMessage::assistant("Hi there"),
        ];
        let pairs = FormatTranslator::encode(&messages, ProviderFormat::Openai).unwrap();
        assert_eq!(pairs.len(), 3);
        assert_eq!(pairs[0].0, "system");
        assert_eq!(pairs[1].0, "user");
        assert_eq!(pairs[2].0, "assistant");
    }

    #[test]
    fn encode_assistant_with_tool_call_includes_inline_marker() {
        let tc = ToolCall {
            id: "call_1".to_string(),
            name: "get_weather".to_string(),
            arguments_json: r#"{"city":"Paris"}"#.to_string(),
        };
        let msg = UnifiedMessage::assistant_with_tools("Using tool", vec![tc]);
        let pairs = FormatTranslator::encode(&[msg], ProviderFormat::Openai).unwrap();
        assert!(pairs[0].1.contains("[tool_call:call_1:get_weather:"));
    }

    #[test]
    fn encode_tool_call_with_empty_name_returns_malformed_error() {
        let tc = ToolCall {
            id: "call_bad".to_string(),
            name: "".to_string(),
            arguments_json: "{}".to_string(),
        };
        let msg = UnifiedMessage::assistant_with_tools("bad", vec![tc]);
        let result = FormatTranslator::encode(&[msg], ProviderFormat::Openai);
        assert!(matches!(result, Err(TranslationError::MalformedToolCall(0))));
    }

    #[test]
    fn decode_anthropic_roles() {
        let pairs = vec![
            ("system".to_string(), "sys".to_string()),
            ("assistant".to_string(), "ans".to_string()),
            ("user".to_string(), "usr".to_string()),
        ];
        let msgs = FormatTranslator::decode(&pairs, ProviderFormat::Anthropic);
        assert_eq!(msgs[0].role, Role::System);
        assert_eq!(msgs[1].role, Role::Assistant);
        assert_eq!(msgs[2].role, Role::User);
    }

    #[test]
    fn decode_google_model_and_function_roles() {
        let pairs = vec![
            ("model".to_string(), "response".to_string()),
            ("function".to_string(), "result".to_string()),
            ("user".to_string(), "question".to_string()),
        ];
        let msgs = FormatTranslator::decode(&pairs, ProviderFormat::Google);
        assert_eq!(msgs[0].role, Role::Assistant);
        assert_eq!(msgs[1].role, Role::Tool);
        assert_eq!(msgs[2].role, Role::User);
    }

    #[test]
    fn anthropic_tool_role_maps_to_user_string() {
        assert_eq!(Role::Tool.as_anthropic(), "user");
    }

    #[test]
    fn google_assistant_role_maps_to_model_string() {
        assert_eq!(Role::Assistant.as_google(), "model");
    }

    #[test]
    fn round_trip_openai_preserves_role_and_content() {
        let original = vec![
            UnifiedMessage::system("Sys prompt"),
            UnifiedMessage::user("Hello world"),
            UnifiedMessage::assistant("Response here"),
        ];
        let encoded = FormatTranslator::encode(&original, ProviderFormat::Openai).unwrap();
        let decoded = FormatTranslator::decode(&encoded, ProviderFormat::Openai);
        assert_eq!(decoded[0].role, Role::System);
        assert_eq!(decoded[0].content, "Sys prompt");
        assert_eq!(decoded[1].role, Role::User);
        assert_eq!(decoded[1].content, "Hello world");
        assert_eq!(decoded[2].role, Role::Assistant);
        assert_eq!(decoded[2].content, "Response here");
    }

    #[test]
    fn tool_builder_prefixes_content_with_call_id() {
        let msg = UnifiedMessage::tool("call_42", "some output");
        assert_eq!(msg.content, "[tool:call_42]some output");
        assert_eq!(msg.role, Role::Tool);
    }
}
