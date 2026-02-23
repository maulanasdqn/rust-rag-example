//! Agentic execution loop (think-act-observe)

use rag_errors::AppError;
use rag_tools::{ToolCall, ToolExecutor, ToolRegistry};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument, warn};

use crate::context::AgentContext;
use crate::traits::{Agent, AgentConfig, AgentResponse, ReasoningStep};

/// Configuration for the execution loop
#[derive(Debug, Clone)]
pub struct ExecutionLoopConfig {
    /// Maximum number of iterations
    pub max_iterations: usize,
    /// Whether to stop on first successful answer
    pub stop_on_answer: bool,
}

impl Default for ExecutionLoopConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            stop_on_answer: true,
        }
    }
}

impl From<&AgentConfig> for ExecutionLoopConfig {
    fn from(config: &AgentConfig) -> Self {
        Self {
            max_iterations: config.max_iterations,
            stop_on_answer: true,
        }
    }
}

/// The agentic execution loop
pub struct AgentExecutionLoop {
    config: ExecutionLoopConfig,
    tool_executor: ToolExecutor,
}

impl AgentExecutionLoop {
    /// Create a new execution loop
    pub fn new(tool_registry: ToolRegistry) -> Self {
        Self {
            config: ExecutionLoopConfig::default(),
            tool_executor: ToolExecutor::new(tool_registry),
        }
    }

    /// Create with custom configuration
    pub fn with_config(tool_registry: ToolRegistry, config: ExecutionLoopConfig) -> Self {
        Self {
            config,
            tool_executor: ToolExecutor::new(tool_registry),
        }
    }

    /// Run the execution loop for an agent
    #[instrument(skip(self, agent, context, llm_callback))]
    pub async fn run<F, Fut>(
        &self,
        agent: &dyn Agent,
        context: &AgentContext,
        query: &str,
        llm_callback: F,
    ) -> Result<AgentResponse, AppError>
    where
        F: Fn(LlmRequest) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<LlmResponse, AppError>> + Send,
    {
        info!("Starting execution loop for agent: {}", agent.name());

        let mut reasoning_steps = Vec::new();
        let mut tool_call_results = Vec::new();
        let mut messages = Vec::new();

        // Build initial system message
        let system_prompt = self.build_system_prompt(agent, context);

        // Add user query
        messages.push(LlmMessage {
            role: "system".to_string(),
            content: system_prompt,
        });

        messages.push(LlmMessage {
            role: "user".to_string(),
            content: query.to_string(),
        });

        // Iteration loop
        for iteration in 0..self.config.max_iterations {
            debug!("Iteration {}/{}", iteration + 1, self.config.max_iterations);

            // Call the LLM
            let response = llm_callback(LlmRequest {
                messages: messages.clone(),
                tools: self
                    .tool_executor
                    .registry()
                    .get_tool_definitions(),
            })
            .await?;

            // Check for final answer
            if response.finish_reason == FinishReason::Stop {
                info!("Agent provided final answer");

                // Add the thought step
                if !response.content.is_empty() {
                    reasoning_steps.push(ReasoningStep::thought(&response.content));
                }

                return Ok(AgentResponse::success(&response.content)
                    .with_reasoning(reasoning_steps)
                    .with_tool_calls(tool_call_results)
                    .with_sources(context.get_source_info()));
            }

            // Handle tool calls
            if response.finish_reason == FinishReason::ToolCalls && !response.tool_calls.is_empty()
            {
                // Record the thought/action
                if !response.content.is_empty() {
                    reasoning_steps.push(ReasoningStep::thought(&response.content));
                }

                // Execute each tool call
                for tool_call in &response.tool_calls {
                    reasoning_steps.push(ReasoningStep::action(format!(
                        "Calling tool '{}' with args: {}",
                        tool_call.name, tool_call.arguments
                    )));

                    let result = self.tool_executor.execute(tool_call).await;

                    reasoning_steps.push(ReasoningStep::observation(format!(
                        "Tool '{}' returned: {}",
                        tool_call.name, result.result.output
                    )));

                    // Add tool result to messages
                    messages.push(LlmMessage {
                        role: "assistant".to_string(),
                        content: response.content.clone(),
                    });

                    messages.push(LlmMessage {
                        role: "tool".to_string(),
                        content: serde_json::to_string(&result).unwrap_or_default(),
                    });

                    tool_call_results.push(result);
                }
            } else {
                // Unexpected finish reason
                warn!(
                    "Unexpected finish reason: {:?}",
                    response.finish_reason
                );

                if !response.content.is_empty() {
                    return Ok(AgentResponse::success(&response.content)
                        .with_reasoning(reasoning_steps)
                        .with_tool_calls(tool_call_results)
                        .with_sources(context.get_source_info()));
                }
            }
        }

        // Reached max iterations
        warn!("Reached maximum iterations ({})", self.config.max_iterations);

        Ok(AgentResponse::failure(format!(
            "Agent reached maximum iterations ({}) without providing a final answer",
            self.config.max_iterations
        ))
        .with_reasoning(reasoning_steps)
        .with_tool_calls(tool_call_results))
    }

    /// Build the system prompt with context
    fn build_system_prompt(&self, agent: &dyn Agent, context: &AgentContext) -> String {
        let mut prompt = agent.system_prompt().to_string();

        // Add context if available
        let formatted_context = context.format_for_llm();
        if !formatted_context.is_empty() {
            prompt.push_str("\n\n## Context\n");
            prompt.push_str(&formatted_context);
        }

        // Add tool usage instructions
        let tools = agent.available_tools();
        if !tools.is_empty() {
            prompt.push_str("\n\n## Available Tools\n");
            prompt.push_str(
                "You have access to the following tools. Use them when needed to complete tasks:\n\n",
            );
            for tool in tools {
                prompt.push_str(&format!("- **{}**: {}\n", tool.name(), tool.description()));
            }
        }

        prompt
    }
}

/// Request to the LLM
#[derive(Debug, Clone)]
pub struct LlmRequest {
    pub messages: Vec<LlmMessage>,
    pub tools: Vec<rag_tools::ToolDefinition>,
}

/// A message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: String,
    pub content: String,
}

/// Response from the LLM
#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub finish_reason: FinishReason,
}

/// Why the LLM stopped generating
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinishReason {
    /// Normal stop (final answer)
    Stop,
    /// Wants to call tools
    ToolCalls,
    /// Hit token limit
    Length,
    /// Other reason
    Other,
}
