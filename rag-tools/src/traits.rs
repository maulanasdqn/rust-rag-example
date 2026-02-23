//! Core trait definitions for tools

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

/// Error type for tool execution failures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolError {
    pub message: String,
    pub recoverable: bool,
}

impl fmt::Display for ToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ToolError {}

impl ToolError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            recoverable: false,
        }
    }

    pub fn recoverable(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            recoverable: true,
        }
    }
}

/// Result of a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// The output of the tool as a string
    pub output: String,
    /// Whether the execution was successful
    pub success: bool,
    /// Optional structured data from the tool
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl ToolResult {
    /// Create a successful result with just output text
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            output: output.into(),
            success: true,
            data: None,
        }
    }

    /// Create a successful result with structured data
    pub fn success_with_data(output: impl Into<String>, data: Value) -> Self {
        Self {
            output: output.into(),
            success: true,
            data: Some(data),
        }
    }

    /// Create a failure result
    pub fn failure(output: impl Into<String>) -> Self {
        Self {
            output: output.into(),
            success: false,
            data: None,
        }
    }
}

/// Trait for defining callable tools that can be used by agents
#[async_trait]
pub trait Tool: Send + Sync {
    /// The unique name of this tool
    fn name(&self) -> &str;

    /// A description of what this tool does (shown to the LLM)
    fn description(&self) -> &str;

    /// JSON Schema for the tool's parameters
    fn parameters_schema(&self) -> Value;

    /// Execute the tool with the given arguments
    async fn execute(&self, args: Value) -> Result<ToolResult, ToolError>;

    /// Whether this tool requires confirmation before execution
    fn requires_confirmation(&self) -> bool {
        false
    }

    /// Validate arguments before execution
    fn validate_args(&self, args: &Value) -> Result<(), ToolError> {
        // Default implementation does no validation
        let _ = args;
        Ok(())
    }
}

/// A tool call request from the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique ID for this tool call
    pub id: String,
    /// Name of the tool to call
    pub name: String,
    /// Arguments for the tool
    pub arguments: Value,
}

/// Result of a tool call to be sent back to the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    /// ID of the tool call this is responding to
    pub tool_call_id: String,
    /// Name of the tool
    pub name: String,
    /// Result of the execution
    pub result: ToolResult,
}

impl ToolCallResult {
    pub fn new(tool_call_id: String, name: String, result: ToolResult) -> Self {
        Self {
            tool_call_id,
            name,
            result,
        }
    }
}

/// Format for OpenAI-compatible function definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

impl FunctionDefinition {
    pub fn from_tool<T: Tool + ?Sized>(tool: &T) -> Self {
        Self {
            name: tool.name().to_string(),
            description: tool.description().to_string(),
            parameters: tool.parameters_schema(),
        }
    }
}

/// OpenAI-compatible tool definition wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDefinition,
}

impl ToolDefinition {
    pub fn from_tool<T: Tool + ?Sized>(tool: &T) -> Self {
        Self {
            tool_type: "function".to_string(),
            function: FunctionDefinition::from_tool(tool),
        }
    }
}
