//! Multi-agent coordinator

use rag_errors::AppError;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, instrument};

use crate::context::AgentContext;
use crate::traits::{Agent, AgentResponse};

/// Coordinator for multiple agents
pub struct AgentCoordinator {
    agents: HashMap<String, Arc<dyn Agent>>,
    default_agent: Option<String>,
}

impl AgentCoordinator {
    /// Create a new coordinator
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
            default_agent: None,
        }
    }

    /// Register an agent
    pub fn register<A: Agent + 'static>(&mut self, agent: A) {
        let name = agent.name().to_string();
        self.agents.insert(name, Arc::new(agent));
    }

    /// Register an agent from Arc
    pub fn register_arc(&mut self, agent: Arc<dyn Agent>) {
        let name = agent.name().to_string();
        self.agents.insert(name, agent);
    }

    /// Set the default agent
    pub fn set_default(&mut self, name: &str) -> Result<(), AppError> {
        if !self.agents.contains_key(name) {
            return Err(AppError::Internal(format!(
                "Agent '{}' not found",
                name
            )));
        }
        self.default_agent = Some(name.to_string());
        Ok(())
    }

    /// Get an agent by name
    pub fn get_agent(&self, name: &str) -> Option<Arc<dyn Agent>> {
        self.agents.get(name).cloned()
    }

    /// Get all agent names
    pub fn agent_names(&self) -> Vec<&str> {
        self.agents.keys().map(|s| s.as_str()).collect()
    }

    /// Select the best agent for a query
    #[instrument(skip(self))]
    pub fn select_agent(&self, query: &str) -> Option<Arc<dyn Agent>> {
        if self.agents.is_empty() {
            return None;
        }

        // Score each agent
        let mut best_score = -1.0f32;
        let mut best_agent: Option<Arc<dyn Agent>> = None;

        for agent in self.agents.values() {
            let score = agent.can_handle(query);
            if score > best_score {
                best_score = score;
                best_agent = Some(agent.clone());
            }
        }

        // If no agent scored well, use default
        if best_score < 0.5 {
            if let Some(ref default_name) = self.default_agent {
                return self.agents.get(default_name).cloned();
            }
        }

        best_agent
    }

    /// Execute a query with automatic agent selection
    #[instrument(skip(self, context))]
    pub async fn execute(
        &self,
        context: &AgentContext,
        query: &str,
    ) -> Result<AgentResponse, AppError> {
        let agent = self.select_agent(query).ok_or_else(|| {
            AppError::Internal("No suitable agent found for query".to_string())
        })?;

        info!("Selected agent '{}' for query", agent.name());
        agent.execute(context, query).await
    }

    /// Execute a query with a specific agent
    #[instrument(skip(self, context))]
    pub async fn execute_with_agent(
        &self,
        agent_name: &str,
        context: &AgentContext,
        query: &str,
    ) -> Result<AgentResponse, AppError> {
        let agent = self.agents.get(agent_name).ok_or_else(|| {
            AppError::Internal(format!("Agent '{}' not found", agent_name))
        })?;

        info!("Executing query with agent '{}'", agent_name);
        agent.execute(context, query).await
    }

    /// Allow one agent to delegate to another
    #[instrument(skip(self, context))]
    pub async fn delegate(
        &self,
        from_agent: &str,
        to_agent: &str,
        context: &AgentContext,
        task: &str,
    ) -> Result<AgentResponse, AppError> {
        info!("Agent '{}' delegating to '{}'", from_agent, to_agent);

        let agent = self.agents.get(to_agent).ok_or_else(|| {
            AppError::Internal(format!("Agent '{}' not found for delegation", to_agent))
        })?;

        agent.execute(context, task).await
    }

    /// Get information about all registered agents
    pub fn get_agent_info(&self) -> Vec<AgentInfo> {
        self.agents
            .values()
            .map(|agent| AgentInfo {
                name: agent.name().to_string(),
                description: agent.description().to_string(),
            })
            .collect()
    }
}

impl Default for AgentCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about an agent
#[derive(Debug, Clone)]
pub struct AgentInfo {
    pub name: String,
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Tests would require mock agents
}
