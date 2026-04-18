//! Robust LLM output parser for ReAct agent actions.
//! Pattern: LlamaIndex output_parser.py — JSON schema + regex fallback.

use serde::{Deserialize, Serialize};

/// Structured action parsed from LLM output
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParsedAction {
    pub thought: String,
    pub action: ActionKind,
}

/// Action variants the agent can take
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActionKind {
    ToolCall { tool: String, input: String },
    Answer(String),
    Reject(String),
}

/// Parse LLM text output into a structured action.
/// Tries JSON first, then falls back to line-by-line regex parsing.
pub fn parse_llm_output(text: &str) -> Result<ParsedAction, ParseError> {
    // Attempt 1: Try JSON block extraction
    if let Some(json_result) = try_json_parse(text) {
        return Ok(json_result);
    }

    // Attempt 2: Line-by-line structured format
    if let Some(structured) = try_structured_parse(text) {
        return Ok(structured);
    }

    // Attempt 3: If text contains "Answer:" treat as final answer
    if let Some(answer) = extract_answer(text) {
        return Ok(ParsedAction {
            thought: extract_thought(text).unwrap_or_default(),
            action: ActionKind::Answer(answer),
        });
    }

    Err(ParseError::Unparseable(text.to_string()))
}

/// Try to extract and parse a JSON block from the output
fn try_json_parse(text: &str) -> Option<ParsedAction> {
    // Look for ```json ... ``` blocks
    let json_start = text
        .find("```json")
        .map(|i| i + 7)
        .or_else(|| text.find("```").map(|i| i + 3))?;
    let json_end = text[json_start..].find("```").map(|i| json_start + i)?;
    let json_str = text[json_start..json_end].trim();

    #[derive(Deserialize)]
    struct JsonAction {
        thought: Option<String>,
        action: Option<String>,
        action_input: Option<String>,
        answer: Option<String>,
    }

    let parsed: JsonAction = serde_json::from_str(json_str).ok()?;

    if let Some(answer) = parsed.answer {
        return Some(ParsedAction {
            thought: parsed.thought.unwrap_or_default(),
            action: ActionKind::Answer(answer),
        });
    }

    if let Some(action) = parsed.action {
        return Some(ParsedAction {
            thought: parsed.thought.unwrap_or_default(),
            action: ActionKind::ToolCall {
                tool: action,
                input: parsed.action_input.unwrap_or_default(),
            },
        });
    }

    None
}

/// Try structured "Thought: / Action: / Action Input:" format
fn try_structured_parse(text: &str) -> Option<ParsedAction> {
    let thought = extract_thought(text)?;

    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("Action:") {
            let tool = rest.trim().to_string();
            if tool.eq_ignore_ascii_case("reject") {
                let reason = extract_after_prefix(text, "Action Input:")
                    .unwrap_or_else(|| "no reason given".to_string());
                return Some(ParsedAction {
                    thought,
                    action: ActionKind::Reject(reason),
                });
            }
            let input = extract_after_prefix(text, "Action Input:").unwrap_or_default();
            return Some(ParsedAction {
                thought,
                action: ActionKind::ToolCall { tool, input },
            });
        }
    }

    None
}

fn extract_thought(text: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("Thought:") {
            return Some(rest.trim().to_string());
        }
    }
    None
}

fn extract_answer(text: &str) -> Option<String> {
    extract_after_prefix(text, "Answer:")
}

fn extract_after_prefix(text: &str, prefix: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return Some(rest.trim().to_string());
        }
    }
    None
}

/// Parse error types
#[derive(Debug, Clone)]
pub enum ParseError {
    Unparseable(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Unparseable(s) => {
                write!(f, "could not parse LLM output: {}", &s[..s.len().min(100)])
            }
        }
    }
}

impl std::error::Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_structured_tool_call() {
        let output =
            "Thought: I need to search\nAction: Query\nAction Input: {\"query\": \"hash\"}";
        let result = parse_llm_output(output).unwrap();
        assert_eq!(result.thought, "I need to search");
        assert!(matches!(result.action, ActionKind::ToolCall { ref tool, .. } if tool == "Query"));
    }

    #[test]
    fn parse_answer() {
        let output = "Thought: Done analyzing\nAnswer: The best function is sha256_hash";
        let result = parse_llm_output(output).unwrap();
        assert!(matches!(result.action, ActionKind::Answer(ref a) if a.contains("sha256")));
    }

    #[test]
    fn parse_json_block() {
        let output = "Here is my response:\n```json\n{\"thought\": \"searching\", \"action\": \"Query\", \"action_input\": \"hash\"}\n```";
        let result = parse_llm_output(output).unwrap();
        assert!(matches!(result.action, ActionKind::ToolCall { ref tool, .. } if tool == "Query"));
    }

    #[test]
    fn parse_reject() {
        let output = "Thought: This won't work\nAction: Reject\nAction Input: approach is flawed";
        let result = parse_llm_output(output).unwrap();
        assert!(matches!(result.action, ActionKind::Reject(ref r) if r.contains("flawed")));
    }

    #[test]
    fn unparseable_returns_error() {
        let output = "random gibberish without any structure";
        assert!(parse_llm_output(output).is_err());
    }
}
