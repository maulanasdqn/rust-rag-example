//! rag-memory - Conversation and long-term memory management
//!
//! This crate provides memory capabilities for the RAG application:
//! - Conversation history management
//! - Message storage and retrieval
//! - Long-term memory with semantic search
//! - Context window management

pub mod context_manager;
pub mod conversation;
pub mod message;
pub mod stores;
pub mod traits;

pub use context_manager::ContextManager;
pub use conversation::Conversation;
pub use message::{Message, MessageRole, ToolCall, ToolResult};
pub use stores::SurrealConversationStore;
pub use traits::{ConversationStore, LongTermMemory};
