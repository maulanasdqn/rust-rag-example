//! Core traits for memory management

use async_trait::async_trait;
use rag_errors::AppError;
use uuid::Uuid;

use crate::{Conversation, Message};

/// Trait for managing conversations and messages
#[async_trait]
pub trait ConversationStore: Send + Sync {
    /// Create a new conversation
    async fn create_conversation(
        &self,
        user_id: Option<&str>,
        title: Option<&str>,
    ) -> Result<Conversation, AppError>;

    /// Get a conversation by ID
    async fn get_conversation(&self, id: Uuid) -> Result<Option<Conversation>, AppError>;

    /// List all conversations for a user
    async fn list_conversations(
        &self,
        user_id: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Conversation>, AppError>;

    /// Update conversation title
    async fn update_conversation_title(&self, id: Uuid, title: &str) -> Result<(), AppError>;

    /// Delete a conversation and all its messages
    async fn delete_conversation(&self, id: Uuid) -> Result<(), AppError>;

    /// Add a message to a conversation
    async fn add_message(&self, conversation_id: Uuid, message: Message)
        -> Result<Uuid, AppError>;

    /// Get messages for a conversation
    async fn get_messages(
        &self,
        conversation_id: Uuid,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Message>, AppError>;

    /// Get the most recent messages for context
    async fn get_recent_messages(
        &self,
        conversation_id: Uuid,
        limit: usize,
    ) -> Result<Vec<Message>, AppError>;

    /// Count messages in a conversation
    async fn count_messages(&self, conversation_id: Uuid) -> Result<usize, AppError>;
}

/// Memory item returned from long-term memory queries
#[derive(Debug, Clone)]
pub struct MemoryItem {
    pub id: Uuid,
    pub content: String,
    pub user_id: Option<String>,
    pub source_conversation: Option<Uuid>,
    pub relevance_score: f32,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Trait for long-term memory with semantic search
#[async_trait]
pub trait LongTermMemory: Send + Sync {
    /// Store a memory with its embedding
    async fn store_memory(
        &self,
        content: &str,
        embedding: Vec<f32>,
        user_id: Option<&str>,
        source_conversation: Option<Uuid>,
    ) -> Result<Uuid, AppError>;

    /// Recall memories similar to the query embedding
    async fn recall(
        &self,
        query_embedding: Vec<f32>,
        user_id: Option<&str>,
        top_k: usize,
    ) -> Result<Vec<MemoryItem>, AppError>;

    /// Delete a specific memory
    async fn delete_memory(&self, id: Uuid) -> Result<(), AppError>;

    /// Delete all memories for a user
    async fn delete_user_memories(&self, user_id: &str) -> Result<usize, AppError>;
}
