//! Message types for conversation history

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Role of the message sender
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// System prompt message
    System,
    /// User message
    User,
    /// Assistant response
    Assistant,
    /// Tool/function result
    Tool,
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageRole::System => write!(f, "system"),
            MessageRole::User => write!(f, "user"),
            MessageRole::Assistant => write!(f, "assistant"),
            MessageRole::Tool => write!(f, "tool"),
        }
    }
}

impl std::str::FromStr for MessageRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "system" => Ok(MessageRole::System),
            "user" => Ok(MessageRole::User),
            "assistant" => Ok(MessageRole::Assistant),
            "tool" => Ok(MessageRole::Tool),
            _ => Err(format!("Unknown role: {}", s)),
        }
    }
}

/// A tool call made by the assistant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique ID for this tool call
    pub id: String,
    /// Name of the tool being called
    pub name: String,
    /// Arguments as JSON
    pub arguments: serde_json::Value,
}

/// Result from a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// ID of the tool call this is responding to
    pub tool_call_id: String,
    /// Name of the tool
    pub name: String,
    /// Output from the tool
    pub output: String,
    /// Whether the tool execution was successful
    pub success: bool,
}

/// A message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique message ID
    pub id: Uuid,
    /// ID of the conversation this message belongs to
    pub conversation_id: Uuid,
    /// Role of the sender
    pub role: MessageRole,
    /// Text content of the message
    pub content: String,
    /// Tool calls made by this message (for assistant messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// Tool results (for tool messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_results: Option<Vec<ToolResult>>,
    /// When the message was created
    pub created_at: DateTime<Utc>,
}

impl Message {
    /// Create a new user message
    pub fn user(conversation_id: Uuid, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            conversation_id,
            role: MessageRole::User,
            content: content.into(),
            tool_calls: None,
            tool_results: None,
            created_at: Utc::now(),
        }
    }

    /// Create a new assistant message
    pub fn assistant(conversation_id: Uuid, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            conversation_id,
            role: MessageRole::Assistant,
            content: content.into(),
            tool_calls: None,
            tool_results: None,
            created_at: Utc::now(),
        }
    }

    /// Create a new assistant message with tool calls
    pub fn assistant_with_tools(
        conversation_id: Uuid,
        content: impl Into<String>,
        tool_calls: Vec<ToolCall>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            conversation_id,
            role: MessageRole::Assistant,
            content: content.into(),
            tool_calls: Some(tool_calls),
            tool_results: None,
            created_at: Utc::now(),
        }
    }

    /// Create a new system message
    pub fn system(conversation_id: Uuid, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            conversation_id,
            role: MessageRole::System,
            content: content.into(),
            tool_calls: None,
            tool_results: None,
            created_at: Utc::now(),
        }
    }

    /// Create a new tool result message
    pub fn tool(conversation_id: Uuid, results: Vec<ToolResult>) -> Self {
        Self {
            id: Uuid::new_v4(),
            conversation_id,
            role: MessageRole::Tool,
            content: String::new(),
            tool_calls: None,
            tool_results: Some(results),
            created_at: Utc::now(),
        }
    }

    /// Estimate token count for this message (rough approximation)
    pub fn estimate_tokens(&self) -> usize {
        // Rough estimate: ~4 characters per token
        let content_tokens = self.content.len() / 4;
        let tool_tokens = self
            .tool_calls
            .as_ref()
            .map(|calls| {
                calls
                    .iter()
                    .map(|c| c.arguments.to_string().len() / 4 + c.name.len())
                    .sum()
            })
            .unwrap_or(0);
        let result_tokens = self
            .tool_results
            .as_ref()
            .map(|results| results.iter().map(|r| r.output.len() / 4).sum())
            .unwrap_or(0);

        content_tokens + tool_tokens + result_tokens + 4 // +4 for role overhead
    }
}
