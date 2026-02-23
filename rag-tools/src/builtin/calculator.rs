//! Calculator tool for evaluating mathematical expressions

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::traits::{Tool, ToolError, ToolResult};

/// Tool for evaluating mathematical expressions
pub struct CalculatorTool;

impl CalculatorTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CalculatorTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for CalculatorTool {
    fn name(&self) -> &str {
        "calculator"
    }

    fn description(&self) -> &str {
        "Evaluate mathematical expressions. Supports basic arithmetic (+, -, *, /), \
         exponentiation (^), parentheses, and common functions like sin, cos, tan, sqrt, log, etc."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "expression": {
                    "type": "string",
                    "description": "The mathematical expression to evaluate (e.g., '2 + 2', 'sqrt(16)', '3 * (4 + 5)')"
                }
            },
            "required": ["expression"]
        })
    }

    fn validate_args(&self, args: &Value) -> Result<(), ToolError> {
        if args.get("expression").is_none() {
            return Err(ToolError::new("Missing required parameter: expression"));
        }
        if !args["expression"].is_string() {
            return Err(ToolError::new("Parameter 'expression' must be a string"));
        }
        Ok(())
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, ToolError> {
        let expression = args["expression"]
            .as_str()
            .ok_or_else(|| ToolError::new("Missing expression"))?;

        // Use meval to evaluate the expression
        match meval::eval_str(expression) {
            Ok(result) => {
                let output = if result.fract() == 0.0 && result.abs() < 1e15 {
                    // Display as integer if it's a whole number
                    format!("{} = {}", expression, result as i64)
                } else {
                    format!("{} = {}", expression, result)
                };

                Ok(ToolResult::success_with_data(
                    output,
                    json!({ "result": result }),
                ))
            }
            Err(e) => Ok(ToolResult::failure(format!(
                "Failed to evaluate '{}': {}",
                expression, e
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_arithmetic() {
        let tool = CalculatorTool::new();

        let result = tool.execute(json!({"expression": "2 + 2"})).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("4"));

        let result = tool
            .execute(json!({"expression": "10 * 5"}))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("50"));
    }

    #[tokio::test]
    async fn test_functions() {
        let tool = CalculatorTool::new();

        let result = tool
            .execute(json!({"expression": "sqrt(16)"}))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("4"));
    }

    #[tokio::test]
    async fn test_invalid_expression() {
        let tool = CalculatorTool::new();

        let result = tool
            .execute(json!({"expression": "invalid"}))
            .await
            .unwrap();
        assert!(!result.success);
    }
}
