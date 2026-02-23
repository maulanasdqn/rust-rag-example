//! Web search tool (placeholder implementation)

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::traits::{Tool, ToolError, ToolResult};

/// Configuration for the web search tool
#[derive(Clone)]
pub struct WebSearchConfig {
    /// API key for the search provider
    pub api_key: Option<String>,
    /// Maximum number of results to return
    pub max_results: usize,
}

impl Default for WebSearchConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            max_results: 5,
        }
    }
}

/// Tool for searching the web
pub struct WebSearchTool {
    config: WebSearchConfig,
}

impl WebSearchTool {
    pub fn new() -> Self {
        Self {
            config: WebSearchConfig::default(),
        }
    }

    pub fn with_config(config: WebSearchConfig) -> Self {
        Self { config }
    }

    pub fn with_api_key(api_key: impl Into<String>) -> Self {
        Self {
            config: WebSearchConfig {
                api_key: Some(api_key.into()),
                ..Default::default()
            },
        }
    }
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web for information. Returns relevant search results with titles, \
         snippets, and URLs."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                },
                "num_results": {
                    "type": "integer",
                    "description": "Number of results to return (default: 5, max: 10)"
                }
            },
            "required": ["query"]
        })
    }

    fn validate_args(&self, args: &Value) -> Result<(), ToolError> {
        if args.get("query").is_none() {
            return Err(ToolError::new("Missing required parameter: query"));
        }
        if !args["query"].is_string() {
            return Err(ToolError::new("Parameter 'query' must be a string"));
        }
        if let Some(query) = args["query"].as_str() {
            if query.trim().is_empty() {
                return Err(ToolError::new("Query cannot be empty"));
            }
        }
        Ok(())
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, ToolError> {
        let query = args["query"]
            .as_str()
            .ok_or_else(|| ToolError::new("Missing query"))?;

        let num_results = args
            .get("num_results")
            .and_then(|v| v.as_u64())
            .map(|n| n.min(10) as usize)
            .unwrap_or(self.config.max_results);

        // Check if API key is configured
        if self.config.api_key.is_none() {
            // Return a mock response for now
            return self.mock_search(query, num_results);
        }

        // TODO: Implement actual web search using an API (e.g., SerpAPI, Bing, etc.)
        // For now, return a placeholder message
        self.mock_search(query, num_results)
    }
}

impl WebSearchTool {
    fn mock_search(&self, query: &str, num_results: usize) -> Result<ToolResult, ToolError> {
        // Generate mock results
        let results: Vec<Value> = (1..=num_results)
            .map(|i| {
                json!({
                    "title": format!("Result {} for '{}'", i, query),
                    "snippet": format!("This is a mock search result snippet for your query about '{}'...", query),
                    "url": format!("https://example.com/result{}", i)
                })
            })
            .collect();

        let output = format!(
            "Found {} results for '{}' (mock data - configure API key for real results):\n\n{}",
            num_results,
            query,
            results
                .iter()
                .map(|r| format!(
                    "• {}\n  {}\n  {}",
                    r["title"].as_str().unwrap_or(""),
                    r["snippet"].as_str().unwrap_or(""),
                    r["url"].as_str().unwrap_or("")
                ))
                .collect::<Vec<_>>()
                .join("\n\n")
        );

        Ok(ToolResult::success_with_data(
            output,
            json!({
                "query": query,
                "num_results": num_results,
                "results": results,
                "is_mock": true
            }),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_web_search_mock() {
        let tool = WebSearchTool::new();

        let result = tool
            .execute(json!({"query": "rust programming"}))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("rust programming"));

        let data = result.data.unwrap();
        assert_eq!(data["is_mock"], true);
        assert_eq!(data["num_results"], 5);
    }

    #[tokio::test]
    async fn test_web_search_custom_num_results() {
        let tool = WebSearchTool::new();

        let result = tool
            .execute(json!({"query": "test", "num_results": 3}))
            .await
            .unwrap();

        assert!(result.success);

        let data = result.data.unwrap();
        assert_eq!(data["num_results"], 3);
        assert_eq!(data["results"].as_array().unwrap().len(), 3);
    }

    #[tokio::test]
    async fn test_web_search_validation() {
        let tool = WebSearchTool::new();

        // Missing query
        let err = tool.validate_args(&json!({}));
        assert!(err.is_err());

        // Empty query
        let err = tool.validate_args(&json!({"query": ""}));
        assert!(err.is_err());
    }
}
