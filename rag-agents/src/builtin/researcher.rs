//! Research agent for information gathering

use async_trait::async_trait;
use rag_errors::AppError;
use rag_tools::{CalculatorTool, DateTimeTool, Tool, WebSearchTool};
use std::sync::Arc;

use crate::context::AgentContext;
use crate::traits::{Agent, AgentConfig, AgentResponse};

/// Agent specialized for research and information gathering
pub struct ResearcherAgent {
    config: AgentConfig,
    tools: Vec<Arc<dyn Tool>>,
}

impl ResearcherAgent {
    pub fn new() -> Self {
        Self {
            config: AgentConfig::default(),
            tools: vec![
                Arc::new(CalculatorTool::new()),
                Arc::new(DateTimeTool::new()),
                Arc::new(WebSearchTool::new()),
            ],
        }
    }

    pub fn with_tools(tools: Vec<Arc<dyn Tool>>) -> Self {
        Self {
            config: AgentConfig::default(),
            tools,
        }
    }

    pub fn with_config(mut self, config: AgentConfig) -> Self {
        self.config = config;
        self
    }
}

impl Default for ResearcherAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Agent for ResearcherAgent {
    fn name(&self) -> &str {
        "researcher"
    }

    fn description(&self) -> &str {
        "An agent specialized for research and information gathering. Uses available tools \
         and document context to find and synthesize information to answer questions."
    }

    fn system_prompt(&self) -> &str {
        r#"You are a research assistant AI. Your job is to help users find and understand information.

## Guidelines

1. **Use Context First**: Always check the provided document context before using external tools.
2. **Be Thorough**: Gather enough information to provide a comprehensive answer.
3. **Cite Sources**: Reference where information came from when possible.
4. **Be Accurate**: Only state facts you can verify from the context or tools.
5. **Acknowledge Uncertainty**: If you're not sure about something, say so.

## Process

1. First, review any provided document context for relevant information.
2. If more information is needed, use available tools.
3. Synthesize the information into a clear, well-organized response.
4. Cite your sources and explain your reasoning.

## Response Format

Structure your response clearly with:
- A direct answer to the question
- Supporting details and evidence
- Sources used (if applicable)
- Any caveats or limitations"#
    }

    fn available_tools(&self) -> Vec<Arc<dyn Tool>> {
        self.tools.clone()
    }

    fn config(&self) -> &AgentConfig {
        &self.config
    }

    fn can_handle(&self, query: &str) -> f32 {
        let query_lower = query.to_lowercase();

        // Keywords that suggest research/information tasks
        let research_keywords = [
            "what", "who", "when", "where", "why", "how",
            "explain", "describe", "find", "search", "look up",
            "tell me about", "information about", "define",
            "research", "investigate", "learn about"
        ];

        let mut score: f32 = 0.5; // Base score

        for keyword in &research_keywords {
            if query_lower.contains(keyword) {
                score += 0.1;
            }
        }

        score.min(1.0)
    }

    async fn execute(
        &self,
        context: &AgentContext,
        query: &str,
    ) -> Result<AgentResponse, AppError> {
        // For now, provide a simple response based on context
        // The actual execution would use the AgentExecutionLoop with an LLM

        let mut answer = String::new();

        // Check if we have document context
        if !context.documents.is_empty() {
            answer.push_str("Based on the available documents:\n\n");

            for doc in &context.documents {
                answer.push_str(&format!(
                    "From '{}' (relevance: {:.2}):\n{}\n\n",
                    doc.source_file,
                    doc.score,
                    doc.content.chars().take(500).collect::<String>()
                ));
            }

            answer.push_str("\nTo provide a more complete answer, this would need to be processed by an LLM with function calling capabilities.");
        } else {
            answer = format!(
                "I understand you're asking: \"{}\"\n\n\
                 To answer this question, I would need to:\n\
                 1. Search the knowledge base for relevant documents\n\
                 2. Use available tools if needed (calculator, web search, etc.)\n\
                 3. Synthesize the information into a comprehensive response\n\n\
                 This functionality requires the full agent execution loop with LLM integration.",
                query
            );
        }

        Ok(AgentResponse::success(answer)
            .with_sources(context.get_source_info()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_researcher_can_handle() {
        let agent = ResearcherAgent::new();

        // Research-like queries should score high
        assert!(agent.can_handle("What is the capital of France?") > 0.5);
        assert!(agent.can_handle("Explain how photosynthesis works") > 0.5);
        assert!(agent.can_handle("Tell me about the history of Rome") > 0.5);

        // Generic queries should get base score
        assert!(agent.can_handle("hello") == 0.5);
    }

    #[tokio::test]
    async fn test_researcher_execute_no_context() {
        let agent = ResearcherAgent::new();
        let context = AgentContext::new();

        let result = agent.execute(&context, "What is 2+2?").await.unwrap();
        assert!(result.success);
        assert!(!result.answer.is_empty());
    }
}
