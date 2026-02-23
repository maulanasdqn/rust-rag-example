//! Tool execution engine

use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{error, info, instrument};

use crate::registry::ToolRegistry;
use crate::traits::{Tool, ToolCall, ToolCallResult, ToolError, ToolResult};

/// Configuration for the tool executor
#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    /// Maximum execution time for a single tool call
    pub timeout: Duration,
    /// Whether to continue on errors
    pub continue_on_error: bool,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            continue_on_error: true,
        }
    }
}

/// Executor for running tools safely
#[derive(Clone)]
pub struct ToolExecutor {
    registry: ToolRegistry,
    config: ExecutorConfig,
}

impl ToolExecutor {
    /// Create a new executor with the given registry
    pub fn new(registry: ToolRegistry) -> Self {
        Self {
            registry,
            config: ExecutorConfig::default(),
        }
    }

    /// Create a new executor with custom configuration
    pub fn with_config(registry: ToolRegistry, config: ExecutorConfig) -> Self {
        Self { registry, config }
    }

    /// Get a reference to the tool registry
    pub fn registry(&self) -> &ToolRegistry {
        &self.registry
    }

    /// Get a mutable reference to the tool registry
    pub fn registry_mut(&mut self) -> &mut ToolRegistry {
        &mut self.registry
    }

    /// Execute a single tool call
    #[instrument(skip(self), fields(tool_name = %tool_call.name))]
    pub async fn execute(&self, tool_call: &ToolCall) -> ToolCallResult {
        info!("Executing tool: {}", tool_call.name);

        let tool = match self.registry.get(&tool_call.name) {
            Some(t) => t,
            None => {
                error!("Tool not found: {}", tool_call.name);
                return ToolCallResult::new(
                    tool_call.id.clone(),
                    tool_call.name.clone(),
                    ToolResult::failure(format!("Tool '{}' not found", tool_call.name)),
                );
            }
        };

        // Validate arguments
        if let Err(e) = tool.validate_args(&tool_call.arguments) {
            error!("Argument validation failed: {}", e);
            return ToolCallResult::new(
                tool_call.id.clone(),
                tool_call.name.clone(),
                ToolResult::failure(format!("Invalid arguments: {}", e)),
            );
        }

        // Execute with timeout
        let result = self.execute_with_timeout(tool, &tool_call.arguments).await;

        match result {
            Ok(tool_result) => {
                info!("Tool execution successful");
                ToolCallResult::new(tool_call.id.clone(), tool_call.name.clone(), tool_result)
            }
            Err(e) => {
                error!("Tool execution failed: {}", e);
                ToolCallResult::new(
                    tool_call.id.clone(),
                    tool_call.name.clone(),
                    ToolResult::failure(e.to_string()),
                )
            }
        }
    }

    /// Execute multiple tool calls, possibly in parallel
    pub async fn execute_all(&self, tool_calls: &[ToolCall]) -> Vec<ToolCallResult> {
        let futures: Vec<_> = tool_calls.iter().map(|tc| self.execute(tc)).collect();

        futures::future::join_all(futures).await
    }

    /// Execute a tool with timeout
    async fn execute_with_timeout(
        &self,
        tool: Arc<dyn Tool>,
        args: &serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        match timeout(self.config.timeout, tool.execute(args.clone())).await {
            Ok(result) => result,
            Err(_) => Err(ToolError::new(format!(
                "Tool execution timed out after {:?}",
                self.config.timeout
            ))),
        }
    }

    /// Execute a tool by name with given arguments
    pub async fn execute_by_name(
        &self,
        name: &str,
        args: serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let tool = self
            .registry
            .get(name)
            .ok_or_else(|| ToolError::new(format!("Tool '{}' not found", name)))?;

        tool.validate_args(&args)?;
        self.execute_with_timeout(tool, &args).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::CalculatorTool;

    #[tokio::test]
    async fn test_executor_execute_calculator() {
        let mut registry = ToolRegistry::new();
        registry.register(CalculatorTool::new());

        let executor = ToolExecutor::new(registry);

        let tool_call = ToolCall {
            id: "test-1".to_string(),
            name: "calculator".to_string(),
            arguments: serde_json::json!({
                "expression": "2 + 2"
            }),
        };

        let result = executor.execute(&tool_call).await;
        assert!(result.result.success);
        assert!(result.result.output.contains("4"));
    }

    #[tokio::test]
    async fn test_executor_tool_not_found() {
        let registry = ToolRegistry::new();
        let executor = ToolExecutor::new(registry);

        let tool_call = ToolCall {
            id: "test-1".to_string(),
            name: "nonexistent".to_string(),
            arguments: serde_json::json!({}),
        };

        let result = executor.execute(&tool_call).await;
        assert!(!result.result.success);
        assert!(result.result.output.contains("not found"));
    }
}
