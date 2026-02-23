//! Core trait definitions for agents

use async_trait::async_trait;
use rag_errors::AppError;
use rag_tools::{Tool, ToolCallResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::context::AgentContext;

/// A step in the agent's reasoning process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningStep {
    /// The type of step (thought, action, observation)
    pub step_type: StepType,
    /// Content of the step
    pub content: String,
    /// Timestamp of the step
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ReasoningStep {
    pub fn thought(content: impl Into<String>) -> Self {
        Self {
            step_type: StepType::Thought,
            content: content.into(),
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn action(content: impl Into<String>) -> Self {
        Self {
            step_type: StepType::Action,
            content: content.into(),
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn observation(content: impl Into<String>) -> Self {
        Self {
            step_type: StepType::Observation,
            content: content.into(),
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Type of reasoning step
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StepType {
    /// Internal thought/reasoning
    Thought,
    /// Action being taken (tool call)
    Action,
    /// Result of an action
    Observation,
}

/// Source information for the response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceInfo {
    pub document_id: Uuid,
    pub source_file: String,
    pub excerpt: String,
    pub relevance_score: f32,
}

/// Response from an agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    /// The final answer from the agent
    pub answer: String,
    /// The reasoning steps taken
    pub reasoning: Vec<ReasoningStep>,
    /// Tool calls made during execution
    pub tool_calls: Vec<ToolCallResult>,
    /// Sources used (if any)
    pub sources: Vec<SourceInfo>,
    /// Whether the agent completed successfully
    pub success: bool,
    /// Optional error message if failed
    pub error: Option<String>,
}

impl AgentResponse {
    /// Create a successful response
    pub fn success(answer: impl Into<String>) -> Self {
        Self {
            answer: answer.into(),
            reasoning: vec![],
            tool_calls: vec![],
            sources: vec![],
            success: true,
            error: None,
        }
    }

    /// Create a failed response
    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            answer: String::new(),
            reasoning: vec![],
            tool_calls: vec![],
            sources: vec![],
            success: false,
            error: Some(error.into()),
        }
    }

    /// Add reasoning steps
    pub fn with_reasoning(mut self, reasoning: Vec<ReasoningStep>) -> Self {
        self.reasoning = reasoning;
        self
    }

    /// Add tool calls
    pub fn with_tool_calls(mut self, tool_calls: Vec<ToolCallResult>) -> Self {
        self.tool_calls = tool_calls;
        self
    }

    /// Add sources
    pub fn with_sources(mut self, sources: Vec<SourceInfo>) -> Self {
        self.sources = sources;
        self
    }
}

/// Configuration for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Maximum number of iterations in the execution loop
    pub max_iterations: usize,
    /// Maximum tokens for context
    pub max_context_tokens: usize,
    /// Whether to include reasoning in the response
    pub include_reasoning: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            max_context_tokens: 8000,
            include_reasoning: true,
        }
    }
}

/// Trait for defining specialized agents
#[async_trait]
pub trait Agent: Send + Sync {
    /// The unique name of this agent
    fn name(&self) -> &str;

    /// A description of what this agent does
    fn description(&self) -> &str;

    /// The system prompt for this agent
    fn system_prompt(&self) -> &str;

    /// Get the tools available to this agent
    fn available_tools(&self) -> Vec<Arc<dyn Tool>>;

    /// Get the agent's configuration
    fn config(&self) -> &AgentConfig;

    /// Execute the agent with the given context and query
    async fn execute(
        &self,
        context: &AgentContext,
        query: &str,
    ) -> Result<AgentResponse, AppError>;

    /// Whether this agent can handle the given query
    fn can_handle(&self, query: &str) -> f32 {
        // Default: return 0.5 (neutral)
        let _ = query;
        0.5
    }
}
