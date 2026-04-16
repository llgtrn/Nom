//! Tool metadata registry for ReAct agents.
//! Pattern: LlamaIndex tools/ — structured metadata enabling MCP tool export.

use serde::{Deserialize, Serialize};
use crate::prompt::ToolMetadata;

/// A registered tool with its metadata and execution capability
#[derive(Debug, Clone)]
pub struct RegisteredTool {
    pub metadata: ToolMetadata,
    pub category: ToolCategory,
}

/// Tool categories for organization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolCategory {
    Query,      // Dictionary/search tools
    Transform,  // Compilation/rendering tools
    Verify,     // Quality/verification tools
    Control,    // Agent control flow tools
}

/// The tool registry — stores all available tools
pub struct ToolRegistry {
    tools: Vec<RegisteredTool>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    /// Create a registry pre-loaded with nom-intent's default 5 tools
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        for (meta, cat) in default_tools() {
            registry.register(meta, cat);
        }
        registry
    }

    pub fn register(&mut self, metadata: ToolMetadata, category: ToolCategory) {
        self.tools.push(RegisteredTool { metadata, category });
    }

    pub fn get(&self, name: &str) -> Option<&RegisteredTool> {
        self.tools.iter().find(|t| t.metadata.name == name)
    }

    pub fn list(&self) -> &[RegisteredTool] {
        &self.tools
    }

    pub fn list_by_category(&self, cat: ToolCategory) -> Vec<&RegisteredTool> {
        self.tools.iter().filter(|t| t.category == cat).collect()
    }

    /// Export as Vec<ToolMetadata> for prompt injection
    pub fn export_metadata(&self) -> Vec<ToolMetadata> {
        self.tools.iter().map(|t| t.metadata.clone()).collect()
    }

    /// Export as MCP-compatible tool list (JSON-serializable)
    pub fn export_mcp_tools(&self) -> Vec<McpToolEntry> {
        self.tools.iter().map(|t| McpToolEntry {
            name: t.metadata.name.clone(),
            description: t.metadata.description.clone(),
            input_schema: McpInputSchema {
                schema_type: "object".to_string(),
                properties: t.metadata.parameters.iter().map(|p| {
                    (p.name.clone(), McpProperty {
                        param_type: p.param_type.clone(),
                        description: p.name.clone(),
                    })
                }).collect(),
                required: t.metadata.parameters.iter()
                    .filter(|p| p.required)
                    .map(|p| p.name.clone())
                    .collect(),
            },
        }).collect()
    }

    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// MCP-compatible tool entry for `initialize` response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolEntry {
    pub name: String,
    pub description: String,
    pub input_schema: McpInputSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpInputSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    pub properties: std::collections::HashMap<String, McpProperty>,
    pub required: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpProperty {
    #[serde(rename = "type")]
    pub param_type: String,
    pub description: String,
}

/// Default 5 tools matching nom-intent's built-in action enum
fn default_tools() -> Vec<(ToolMetadata, ToolCategory)> {
    use crate::prompt::ToolParameter;
    vec![
        (ToolMetadata {
            name: "Query".into(),
            description: "Search the dictionary for entities matching a query string".into(),
            parameters: vec![
                ToolParameter { name: "query".into(), param_type: "string".into(), required: true },
                ToolParameter { name: "kind".into(), param_type: "string".into(), required: false },
                ToolParameter { name: "limit".into(), param_type: "integer".into(), required: false },
            ],
        }, ToolCategory::Query),
        (ToolMetadata {
            name: "Render".into(),
            description: "Compile an entity to its target artifact (LLVM bitcode, media, etc.)".into(),
            parameters: vec![
                ToolParameter { name: "hash".into(), param_type: "string".into(), required: true },
            ],
        }, ToolCategory::Transform),
        (ToolMetadata {
            name: "Verify".into(),
            description: "Check that a rendered artifact meets quality and correctness criteria".into(),
            parameters: vec![
                ToolParameter { name: "hash".into(), param_type: "string".into(), required: true },
                ToolParameter { name: "threshold".into(), param_type: "number".into(), required: false },
            ],
        }, ToolCategory::Verify),
        (ToolMetadata {
            name: "Reject".into(),
            description: "Reject the current approach with a reason and try a different strategy".into(),
            parameters: vec![
                ToolParameter { name: "reason".into(), param_type: "string".into(), required: true },
            ],
        }, ToolCategory::Control),
        (ToolMetadata {
            name: "Answer".into(),
            description: "Provide the final answer to the task, ending the ReAct loop".into(),
            parameters: vec![
                ToolParameter { name: "answer".into(), param_type: "string".into(), required: true },
            ],
        }, ToolCategory::Control),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_registry_has_five_tools() {
        let reg = ToolRegistry::with_defaults();
        assert_eq!(reg.len(), 5);
    }

    #[test]
    fn lookup_by_name() {
        let reg = ToolRegistry::with_defaults();
        assert!(reg.get("Query").is_some());
        assert!(reg.get("NonExistent").is_none());
    }

    #[test]
    fn filter_by_category() {
        let reg = ToolRegistry::with_defaults();
        let control = reg.list_by_category(ToolCategory::Control);
        assert_eq!(control.len(), 2); // Reject + Answer
    }

    #[test]
    fn mcp_export_has_correct_structure() {
        let reg = ToolRegistry::with_defaults();
        let mcp = reg.export_mcp_tools();
        assert_eq!(mcp.len(), 5);
        let query = &mcp[0];
        assert_eq!(query.name, "Query");
        assert_eq!(query.input_schema.required.len(), 1);
    }
}
