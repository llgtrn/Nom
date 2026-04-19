#![deny(unsafe_code)]

use std::collections::HashMap;
use std::fmt;

// ============================================================================
// Schema Types
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum ParamType {
    String,
    Number,
    Boolean,
    Array,
    Object,
}

impl fmt::Display for ParamType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParamType::String => write!(f, "string"),
            ParamType::Number => write!(f, "number"),
            ParamType::Boolean => write!(f, "boolean"),
            ParamType::Array => write!(f, "array"),
            ParamType::Object => write!(f, "object"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParamSchema {
    pub name: String,
    pub param_type: ParamType,
    pub required: bool,
    pub description: Option<String>,
}

impl ParamSchema {
    pub fn new(name: impl Into<String>, param_type: ParamType, required: bool) -> Self {
        Self {
            name: name.into(),
            param_type,
            required,
            description: None,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ParamSchema>,
}

impl ToolSchema {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters: Vec::new(),
        }
    }

    pub fn with_param(mut self, param: ParamSchema) -> Self {
        self.parameters.push(param);
        self
    }
}

// ============================================================================
// Error Type
// ============================================================================

#[derive(Debug)]
pub enum ToolError {
    MissingParameter(String),
    InvalidParameterType {
        name: String,
        expected: ParamType,
        got: String,
    },
    PermissionDenied(String),
    ExecutionFailed(String),
    ToolNotFound(String),
}

impl fmt::Display for ToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolError::MissingParameter(name) => {
                write!(f, "missing required parameter: {}", name)
            }
            ToolError::InvalidParameterType {
                name,
                expected,
                got,
            } => {
                write!(
                    f,
                    "parameter '{}' expected type {} but got {}",
                    name, expected, got
                )
            }
            ToolError::PermissionDenied(msg) => write!(f, "permission denied: {}", msg),
            ToolError::ExecutionFailed(msg) => write!(f, "execution failed: {}", msg),
            ToolError::ToolNotFound(name) => write!(f, "tool not found: {}", name),
        }
    }
}

impl std::error::Error for ToolError {}

// ============================================================================
// Permission System
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PermissionLevel {
    ReadOnly,
    FileWrite,
    Network,
    Execute,
}

#[derive(Debug, Clone)]
pub struct ToolContext {
    pub cwd: std::path::PathBuf,
    pub allow_file_write: bool,
    pub allow_network: bool,
    pub allow_execute: bool,
}

impl Default for ToolContext {
    fn default() -> Self {
        Self {
            cwd: std::env::current_dir().unwrap_or_default(),
            allow_file_write: true,
            allow_network: true,
            allow_execute: false,
        }
    }
}

pub struct PermissionGate;

impl PermissionGate {
    pub fn check(
        tool: &dyn Tool,
        args: &serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<(), ToolError> {
        let level = tool.requires_permission(args);
        let allowed = match level {
            PermissionLevel::ReadOnly => true,
            PermissionLevel::FileWrite => ctx.allow_file_write,
            PermissionLevel::Network => ctx.allow_network,
            PermissionLevel::Execute => ctx.allow_execute,
        };
        if allowed {
            Ok(())
        } else {
            Err(ToolError::PermissionDenied(format!(
                "tool '{}' requires {:?} permission, which is not granted",
                tool.name(),
                level
            )))
        }
    }
}

// ============================================================================
// Tool Trait
// ============================================================================

pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn schema(&self) -> ToolSchema;
    fn requires_permission(&self, _args: &serde_json::Value) -> PermissionLevel {
        PermissionLevel::ReadOnly
    }
    fn execute(&self, args: serde_json::Value) -> Result<String, ToolError>;
}

// ============================================================================
// Built-in Tools
// ============================================================================

pub struct SearchTool;

impl Tool for SearchTool {
    fn name(&self) -> &str {
        "search"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new("search", "Perform a search query").with_param(
            ParamSchema::new("query", ParamType::String, true)
                .with_description("The search query string"),
        )
    }

    fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::MissingParameter("query".to_string()))?;
        Ok(format!("search results for: {}", query))
    }
}

pub struct CodeTool;

impl Tool for CodeTool {
    fn name(&self) -> &str {
        "code"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new("code", "Execute or analyze code")
            .with_param(
                ParamSchema::new("code", ParamType::String, true)
                    .with_description("The code to execute or analyze"),
            )
            .with_param(
                ParamSchema::new("language", ParamType::String, false)
                    .with_description("Optional language hint"),
            )
    }

    fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let code = args
            .get("code")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::MissingParameter("code".to_string()))?;
        let lang = args
            .get("language")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        Ok(format!("executed {} code: {}", lang, code))
    }
}

pub struct BrowseTool;

impl Tool for BrowseTool {
    fn name(&self) -> &str {
        "browse"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new("browse", "Fetch content from a URL").with_param(
            ParamSchema::new("url", ParamType::String, true).with_description("The URL to fetch"),
        )
    }

    fn requires_permission(&self, _args: &serde_json::Value) -> PermissionLevel {
        PermissionLevel::Network
    }

    fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::MissingParameter("url".to_string()))?;
        Ok(format!("fetched content from: {}", url))
    }
}

pub struct CalcTool;

impl Tool for CalcTool {
    fn name(&self) -> &str {
        "calculate"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new("calculate", "Evaluate a mathematical expression").with_param(
            ParamSchema::new("expression", ParamType::String, true)
                .with_description("A mathematical expression to evaluate"),
        )
    }

    fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let expr = args
            .get("expression")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::MissingParameter("expression".to_string()))?;

        // Try literal number first.
        if let Ok(n) = expr.parse::<f64>() {
            return Ok(n.to_string());
        }

        // Very basic two-operand evaluator for demonstration.
        let ops = ['+', '-', '*', '/'];
        for op in ops {
            if let Some((lhs, rhs)) = expr.split_once(op) {
                let lhs = lhs.trim().parse::<f64>().map_err(|_| {
                    ToolError::ExecutionFailed(format!("invalid number: {}", lhs.trim()))
                })?;
                let rhs = rhs.trim().parse::<f64>().map_err(|_| {
                    ToolError::ExecutionFailed(format!("invalid number: {}", rhs.trim()))
                })?;
                let result = match op {
                    '+' => lhs + rhs,
                    '-' => lhs - rhs,
                    '*' => lhs * rhs,
                    '/' => {
                        if rhs == 0.0 {
                            return Err(ToolError::ExecutionFailed(
                                "division by zero".to_string(),
                            ));
                        }
                        lhs / rhs
                    }
                    _ => unreachable!(),
                };
                return Ok(result.to_string());
            }
        }

        Err(ToolError::ExecutionFailed(format!(
            "unable to evaluate expression: {}",
            expr
        )))
    }
}

pub struct FileTool;

impl Tool for FileTool {
    fn name(&self) -> &str {
        "file"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new("file", "Read from or write to a file")
            .with_param(
                ParamSchema::new("operation", ParamType::String, true)
                    .with_description("Either 'read' or 'write'"),
            )
            .with_param(
                ParamSchema::new("path", ParamType::String, true)
                    .with_description("The file path"),
            )
            .with_param(
                ParamSchema::new("content", ParamType::String, false)
                    .with_description("Content to write (required for write operation)"),
            )
    }

    fn requires_permission(&self, args: &serde_json::Value) -> PermissionLevel {
        match args.get("operation").and_then(|v| v.as_str()) {
            Some("write") => PermissionLevel::FileWrite,
            _ => PermissionLevel::ReadOnly,
        }
    }

    fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let operation = args
            .get("operation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::MissingParameter("operation".to_string()))?;
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::MissingParameter("path".to_string()))?;

        match operation {
            "read" => std::fs::read_to_string(path)
                .map_err(|e| ToolError::ExecutionFailed(format!("read error: {}", e))),
            "write" => {
                let content = args
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::MissingParameter("content".to_string()))?;
                std::fs::write(path, content)
                    .map(|_| "written".to_string())
                    .map_err(|e| ToolError::ExecutionFailed(format!("write error: {}", e)))
            }
            other => Err(ToolError::ExecutionFailed(format!(
                "unknown operation: {}. Expected 'read' or 'write'",
                other
            ))),
        }
    }
}

// ============================================================================
// ToolRegistry
// ============================================================================

pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(SearchTool));
        registry.register(Box::new(CodeTool));
        registry.register(Box::new(BrowseTool));
        registry.register(Box::new(CalcTool));
        registry.register(Box::new(FileTool));
        registry
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    pub fn list_tools(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }

    pub fn execute(
        &self,
        name: &str,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<String, ToolError> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| ToolError::ToolNotFound(name.to_string()))?;
        PermissionGate::check(tool.as_ref(), &args, ctx)?;
        validate_args(&tool.schema(), &args)?;
        tool.execute(args)
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn validate_args(schema: &ToolSchema, args: &serde_json::Value) -> Result<(), ToolError> {
    let obj = args.as_object().ok_or_else(|| {
        ToolError::ExecutionFailed("arguments must be a JSON object".to_string())
    })?;

    for param in &schema.parameters {
        let present = obj.contains_key(&param.name);
        if param.required && !present {
            return Err(ToolError::MissingParameter(param.name.clone()));
        }
        if present {
            let value = &obj[&param.name];
            let valid = match param.param_type {
                ParamType::String => value.is_string(),
                ParamType::Number => value.is_number(),
                ParamType::Boolean => value.is_boolean(),
                ParamType::Array => value.is_array(),
                ParamType::Object => value.is_object(),
            };
            if !valid {
                return Err(ToolError::InvalidParameterType {
                    name: param.name.clone(),
                    expected: param.param_type.clone(),
                    got: format!("{:?}", value),
                });
            }
        }
    }

    Ok(())
}

// ============================================================================
// ToolHarness (convenience wrapper)
// ============================================================================

pub struct ToolHarness {
    registry: ToolRegistry,
    context: ToolContext,
}

impl ToolHarness {
    pub fn new() -> Self {
        Self {
            registry: ToolRegistry::with_defaults(),
            context: ToolContext::default(),
        }
    }

    pub fn with_context(context: ToolContext) -> Self {
        Self {
            registry: ToolRegistry::with_defaults(),
            context,
        }
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.registry.register(tool);
    }

    pub fn invoke(&self, name: &str, args: serde_json::Value) -> Result<String, ToolError> {
        self.registry.execute(name, args, &self.context)
    }

    pub fn list_tools(&self) -> Vec<&str> {
        self.registry.list_tools()
    }
}

impl Default for ToolHarness {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_tool_schema() {
        let tool = SearchTool;
        let schema = tool.schema();
        assert_eq!(schema.name, "search");
        assert_eq!(schema.parameters.len(), 1);
        assert_eq!(schema.parameters[0].name, "query");
        assert_eq!(schema.parameters[0].param_type, ParamType::String);
        assert!(schema.parameters[0].required);
    }

    #[test]
    fn test_search_tool_execution() {
        let tool = SearchTool;
        let args = serde_json::json!({ "query": "rust programming" });
        let result = tool.execute(args).unwrap();
        assert!(result.contains("rust programming"));
    }

    #[test]
    fn test_code_tool_execution() {
        let tool = CodeTool;
        let args = serde_json::json!({
            "code": "println!(\"hello\");",
            "language": "rust"
        });
        let result = tool.execute(args).unwrap();
        assert!(result.contains("rust"));
        assert!(result.contains("println!"));
    }

    #[test]
    fn test_browse_tool_requires_network_permission() {
        let tool = BrowseTool;
        let args = serde_json::json!({ "url": "https://example.com" });
        assert_eq!(tool.requires_permission(&args), PermissionLevel::Network);
    }

    #[test]
    fn test_browse_tool_execution() {
        let tool = BrowseTool;
        let args = serde_json::json!({ "url": "https://example.com" });
        let result = tool.execute(args).unwrap();
        assert!(result.contains("example.com"));
    }

    #[test]
    fn test_calc_tool_parses_number() {
        let tool = CalcTool;
        let args = serde_json::json!({ "expression": "42" });
        assert_eq!(tool.execute(args).unwrap(), "42");
    }

    #[test]
    fn test_calc_tool_adds() {
        let tool = CalcTool;
        let args = serde_json::json!({ "expression": "1 + 2" });
        assert_eq!(tool.execute(args).unwrap(), "3");
    }

    #[test]
    fn test_calc_tool_invalid_expression() {
        let tool = CalcTool;
        let args = serde_json::json!({ "expression": "not a number" });
        assert!(tool.execute(args).is_err());
    }

    #[test]
    fn test_file_tool_read() {
        let tool = FileTool;
        let args = serde_json::json!({ "operation": "read", "path": "Cargo.toml" });
        let result = tool.execute(args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_file_tool_write_and_read() {
        let tool = FileTool;
        let tmp = std::env::temp_dir().join("nom_compose_test.txt");
        let path = tmp.to_str().unwrap();

        // Write
        let write_args = serde_json::json!({
            "operation": "write",
            "path": path,
            "content": "hello world"
        });
        assert_eq!(tool.execute(write_args).unwrap(), "written");

        // Read back
        let read_args = serde_json::json!({ "operation": "read", "path": path });
        assert_eq!(tool.execute(read_args).unwrap(), "hello world");

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_file_tool_permission_readonly_vs_write() {
        let tool = FileTool;
        let read_args = serde_json::json!({ "operation": "read", "path": "." });
        let write_args =
            serde_json::json!({ "operation": "write", "path": ".", "content": "x" });
        assert_eq!(tool.requires_permission(&read_args), PermissionLevel::ReadOnly);
        assert_eq!(tool.requires_permission(&write_args), PermissionLevel::FileWrite);
    }

    #[test]
    fn test_registry_default_tools() {
        let registry = ToolRegistry::with_defaults();
        let tools = registry.list_tools();
        assert!(tools.contains(&"search"));
        assert!(tools.contains(&"code"));
        assert!(tools.contains(&"browse"));
        assert!(tools.contains(&"calculate"));
        assert!(tools.contains(&"file"));
    }

    #[test]
    fn test_registry_execute_with_validation() {
        let registry = ToolRegistry::with_defaults();
        let ctx = ToolContext::default();

        // Valid args
        let args = serde_json::json!({ "query": "test" });
        let result = registry.execute("search", args, &ctx).unwrap();
        assert!(result.contains("test"));

        // Missing required parameter
        let args = serde_json::json!({});
        let err = registry.execute("search", args, &ctx).unwrap_err();
        assert!(matches!(err, ToolError::MissingParameter(_)));
    }

    #[test]
    fn test_registry_tool_not_found() {
        let registry = ToolRegistry::new();
        let ctx = ToolContext::default();
        let err = registry
            .execute("missing", serde_json::json!({}), &ctx)
            .unwrap_err();
        assert!(matches!(err, ToolError::ToolNotFound(_)));
    }

    #[test]
    fn test_permission_gate_blocks_network() {
        let registry = ToolRegistry::with_defaults();
        let mut ctx = ToolContext::default();
        ctx.allow_network = false;

        let args = serde_json::json!({ "url": "https://example.com" });
        let err = registry.execute("browse", args, &ctx).unwrap_err();
        assert!(matches!(err, ToolError::PermissionDenied(_)));
    }

    #[test]
    fn test_permission_gate_blocks_file_write() {
        let registry = ToolRegistry::with_defaults();
        let mut ctx = ToolContext::default();
        ctx.allow_file_write = false;

        let args = serde_json::json!({
            "operation": "write",
            "path": "test.txt",
            "content": "data"
        });
        let err = registry.execute("file", args, &ctx).unwrap_err();
        assert!(matches!(err, ToolError::PermissionDenied(_)));
    }

    #[test]
    fn test_permission_gate_allows_file_read() {
        let registry = ToolRegistry::with_defaults();
        let mut ctx = ToolContext::default();
        ctx.allow_file_write = false;

        let args = serde_json::json!({ "operation": "read", "path": "Cargo.toml" });
        assert!(registry.execute("file", args, &ctx).is_ok());
    }

    #[test]
    fn test_harness_register_and_invoke() {
        struct EchoTool;
        impl Tool for EchoTool {
            fn name(&self) -> &str {
                "echo"
            }
            fn schema(&self) -> ToolSchema {
                ToolSchema::new("echo", "Echo input back").with_param(ParamSchema::new(
                    "message",
                    ParamType::String,
                    true,
                ))
            }
            fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
                let msg = args.get("message").and_then(|v| v.as_str()).unwrap_or("");
                Ok(msg.to_string())
            }
        }

        let mut harness = ToolHarness::new();
        harness.register(Box::new(EchoTool));
        let args = serde_json::json!({ "message": "hello" });
        assert_eq!(harness.invoke("echo", args).unwrap(), "hello");
    }

    #[test]
    fn test_harness_list_tools() {
        let harness = ToolHarness::new();
        let tools = harness.list_tools();
        assert!(tools.contains(&"search"));
        assert!(tools.contains(&"calculate"));
        assert!(tools.contains(&"file"));
    }

    #[test]
    fn test_invalid_parameter_type() {
        let registry = ToolRegistry::with_defaults();
        let ctx = ToolContext::default();
        let args = serde_json::json!({ "query": 123 });
        let err = registry.execute("search", args, &ctx).unwrap_err();
        assert!(matches!(err, ToolError::InvalidParameterType { .. }));
    }

    #[test]
    fn test_validate_args_non_object() {
        let schema = ToolSchema::new("test", "test");
        let args = serde_json::json!("string");
        let err = validate_args(&schema, &args).unwrap_err();
        assert!(matches!(err, ToolError::ExecutionFailed(_)));
    }
}
