//! rag-agents - Agentic system for RAG application
//!
//! This crate provides:
//! - Agent trait for defining specialized agents
//! - AgentCoordinator for multi-agent orchestration
//! - Agentic execution loop (think-act-observe)
//! - Built-in agents (researcher, analyst, coder)

pub mod builtin;
pub mod context;
pub mod coordinator;
pub mod execution_loop;
pub mod traits;

pub use builtin::ResearcherAgent;
pub use context::AgentContext;
pub use coordinator::AgentCoordinator;
pub use execution_loop::AgentExecutionLoop;
pub use traits::{Agent, AgentResponse, ReasoningStep};
