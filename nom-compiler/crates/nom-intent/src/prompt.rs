//! ReAct prompt formatting — auto-inject tool descriptions into system prompt.
//! Pattern: abstract thought-action-observation loop formatter.

/// Tool metadata for prompt injection
#[derive(Debug, Clone)]
pub struct ToolMetadata {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
}

#[derive(Debug, Clone)]
pub struct ToolParameter {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    pub description: String,
}

/// Format a ReAct system prompt with tool descriptions
pub fn format_react_prompt(tools: &[ToolMetadata], task: &str) -> String {
    let mut prompt = String::new();

    prompt.push_str("You are an agent that solves tasks using a thought-action-observation loop.\n\n");
    prompt.push_str("## Available Tools\n\n");

    for tool in tools {
        prompt.push_str(&format!("### {}\n", tool.name));
        prompt.push_str(&format!("{}\n", tool.description));
        if !tool.parameters.is_empty() {
            prompt.push_str("Parameters:\n");
            for param in &tool.parameters {
                let req = if param.required { "required" } else { "optional" };
                prompt.push_str(&format!("  - {} ({}, {}): {}\n", param.name, param.param_type, req, param.description));
            }
        }
        prompt.push('\n');
    }

    prompt.push_str("## Response Format\n\n");
    prompt.push_str("Thought: [your reasoning about what to do next]\n");
    prompt.push_str("Action: [tool_name]\n");
    prompt.push_str("Action Input: {\"param\": \"value\"}\n\n");
    prompt.push_str("After receiving an observation, continue the loop or conclude:\n");
    prompt.push_str("Thought: [final reasoning]\n");
    prompt.push_str("Answer: [final answer to the task]\n\n");
    prompt.push_str(&format!("## Task\n\n{}\n", task));

    prompt
}

/// Parse an LLM response to extract action + params (with regex fallback)
pub fn parse_action_response(response: &str) -> Option<(String, String)> {
    // Try structured format first: "Action: tool_name\nAction Input: {...}"
    let mut action = None;
    let mut input = None;

    for line in response.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("Action:") {
            action = Some(rest.trim().to_string());
        } else if let Some(rest) = trimmed.strip_prefix("Action Input:") {
            input = Some(rest.trim().to_string());
        }
    }

    if let Some(act) = action {
        Some((act, input.unwrap_or_default()))
    } else {
        // Regex fallback: look for JSON-like tool invocation
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool_registry::ToolRegistry;

    #[test]
    fn format_prompt_includes_all_tools() {
        let tools = ToolRegistry::with_defaults().export_metadata();
        let prompt = format_react_prompt(&tools, "Find a function for hashing");
        assert!(prompt.contains("Query"));
        assert!(prompt.contains("Render"));
        assert!(prompt.contains("Verify"));
        assert!(prompt.contains("Find a function for hashing"));
    }

    #[test]
    fn parse_action_extracts_tool_and_input() {
        let response =
            "Thought: I need to search\nAction: Query\nAction Input: {\"query\": \"hash\"}";
        let (action, input) = parse_action_response(response).unwrap();
        assert_eq!(action, "Query");
        assert!(input.contains("hash"));
    }

    #[test]
    fn parse_action_returns_none_for_answer() {
        let response = "Thought: Done\nAnswer: The hash function is sha256";
        assert!(parse_action_response(response).is_none());
    }
}
