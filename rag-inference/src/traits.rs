use async_trait::async_trait;
use futures::stream::BoxStream;
use rag_database::SearchResult;
use rag_errors::AppError;
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed_query(&self, text: &str) -> Result<Vec<f32>, AppError>;
    async fn embed_texts(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError>;
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn generate(&self, query: &str, context: &[SearchResult]) -> Result<String, AppError>;
    async fn generate_stream(
        &self,
        query: &str,
        context: &[SearchResult],
    ) -> Result<BoxStream<'static, Result<String, AppError>>, AppError>;

    /// Generate a streaming response with conversation history
    async fn generate_stream_with_history(
        &self,
        query: &str,
        context: &[SearchResult],
        history: Vec<ChatMessage>,
    ) -> Result<BoxStream<'static, Result<String, AppError>>, AppError>;

    /// Generate a response with tool/function calling support
    async fn generate_with_tools(
        &self,
        messages: Vec<ChatMessage>,
        tools: Vec<ToolDefinition>,
    ) -> Result<ChatCompletionResponse, AppError>;
}

/// A message in the chat conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallRequest>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn assistant_with_tool_calls(content: impl Into<String>, tool_calls: Vec<ToolCallRequest>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            tool_calls: Some(tool_calls),
            tool_call_id: None,
        }
    }

    pub fn tool(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Tool,
            content: content.into(),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}

/// Role of a message sender
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// Definition of a tool for the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDefinition,
}

impl ToolDefinition {
    pub fn function(name: impl Into<String>, description: impl Into<String>, parameters: serde_json::Value) -> Self {
        Self {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: name.into(),
                description: description.into(),
                parameters,
            },
        }
    }
}

/// Function definition within a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// A tool call request from the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub id: String,
    pub r#type: String,
    pub function: FunctionCall,
}

/// The function being called
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

/// Response from a chat completion with tools
#[derive(Debug, Clone)]
pub struct ChatCompletionResponse {
    pub content: String,
    pub tool_calls: Vec<ToolCallRequest>,
    pub finish_reason: FinishReason,
}

impl ChatCompletionResponse {
    pub fn message(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            tool_calls: vec![],
            finish_reason: FinishReason::Stop,
        }
    }

    pub fn with_tool_calls(content: impl Into<String>, tool_calls: Vec<ToolCallRequest>) -> Self {
        Self {
            content: content.into(),
            tool_calls,
            finish_reason: FinishReason::ToolCalls,
        }
    }
}

/// Reason why the model stopped generating
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinishReason {
    Stop,
    ToolCalls,
    Length,
    ContentFilter,
}

impl Default for FinishReason {
    fn default() -> Self {
        Self::Stop
    }
}
