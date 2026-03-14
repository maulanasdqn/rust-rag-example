//! rag-tools - Tool definitions and execution for RAG
//!
//! This crate provides:
//! - Tool trait for defining callable tools
//! - Tool registry for managing available tools
//! - Tool executor for running tools safely
//! - Built-in tools (calculator, datetime, web_search)

pub mod builtin;
pub mod executor;
pub mod registry;
pub mod traits;

pub use builtin::{CalculatorTool, DateTimeTool, WebSearchTool};
pub use executor::ToolExecutor;
pub use registry::ToolRegistry;
pub use traits::{Tool, ToolCall, ToolCallResult, ToolDefinition, ToolError, ToolResult};
