//! SurrealDB implementation of conversation storage

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rag_database::DbPool;
use rag_errors::AppError;
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;
use uuid::Uuid;

use crate::{
    traits::{ConversationStore, LongTermMemory, MemoryItem},
    Conversation, Message, MessageRole, ToolCall, ToolResult,
};

/// SurrealDB implementation of the ConversationStore trait
#[derive(Clone)]
pub struct SurrealConversationStore {
    db: DbPool,
}

impl SurrealConversationStore {
    /// Create a new SurrealDB conversation store
    pub fn new(db: DbPool) -> Self {
        Self { db }
    }

    /// Initialize the database schema for conversations and messages
    pub async fn init_schema(&self) -> Result<(), AppError> {
        // Create conversation table
        self.db
            .query(
                r#"
            DEFINE TABLE IF NOT EXISTS conversation SCHEMAFULL;
            DEFINE FIELD IF NOT EXISTS user_id ON conversation TYPE option<string>;
            DEFINE FIELD IF NOT EXISTS title ON conversation TYPE string;
            DEFINE FIELD IF NOT EXISTS message_count ON conversation TYPE int DEFAULT 0;
            DEFINE FIELD IF NOT EXISTS created_at ON conversation TYPE datetime DEFAULT time::now();
            DEFINE FIELD IF NOT EXISTS updated_at ON conversation TYPE datetime DEFAULT time::now();
            DEFINE INDEX IF NOT EXISTS idx_conversation_user ON conversation FIELDS user_id;
            "#,
            )
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to create conversation table: {}", e)))?;

        // Create message table
        self.db
            .query(
                r#"
            DEFINE TABLE IF NOT EXISTS message SCHEMAFULL;
            DEFINE FIELD IF NOT EXISTS conversation_id ON message TYPE string;
            DEFINE FIELD IF NOT EXISTS role ON message TYPE string;
            DEFINE FIELD IF NOT EXISTS content ON message TYPE string;
            DEFINE FIELD IF NOT EXISTS tool_calls ON message TYPE option<array>;
            DEFINE FIELD IF NOT EXISTS tool_results ON message TYPE option<array>;
            DEFINE FIELD IF NOT EXISTS created_at ON message TYPE datetime DEFAULT time::now();
            DEFINE INDEX IF NOT EXISTS idx_message_conversation ON message FIELDS conversation_id;
            "#,
            )
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to create message table: {}", e)))?;

        // Create memory table for long-term memory
        self.db
            .query(
                r#"
            DEFINE TABLE IF NOT EXISTS memory SCHEMAFULL;
            DEFINE FIELD IF NOT EXISTS user_id ON memory TYPE option<string>;
            DEFINE FIELD IF NOT EXISTS content ON memory TYPE string;
            DEFINE FIELD IF NOT EXISTS embedding ON memory TYPE array<float>;
            DEFINE FIELD IF NOT EXISTS source_conversation ON memory TYPE option<string>;
            DEFINE FIELD IF NOT EXISTS created_at ON memory TYPE datetime DEFAULT time::now();
            DEFINE INDEX IF NOT EXISTS idx_memory_user ON memory FIELDS user_id;
            "#,
            )
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to create memory table: {}", e)))?;

        Ok(())
    }
}

/// Database record for a conversation
#[derive(Debug, Serialize, Deserialize)]
struct ConversationRecord {
    id: Thing,
    user_id: Option<String>,
    title: String,
    message_count: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<ConversationRecord> for Conversation {
    fn from(record: ConversationRecord) -> Self {
        // SurrealDB Thing.id can be in different formats, try to extract the UUID
        let id_str = record.id.id.to_string();
        tracing::info!("Raw SurrealDB conversation ID: '{}'", id_str);
        // Remove quotes if present (SurrealDB sometimes wraps strings in quotes)
        let id_str = id_str.trim_matches('"').trim_matches('\'');
        // Also try removing backticks which SurrealDB uses for string IDs
        let id_str = id_str.trim_matches('`');
        tracing::info!("Cleaned conversation ID: '{}'", id_str);
        let id = Uuid::parse_str(id_str).unwrap_or_else(|e| {
            tracing::error!("Failed to parse conversation UUID '{}': {}", id_str, e);
            Uuid::nil()
        });

        Conversation {
            id,
            user_id: record.user_id,
            title: record.title,
            message_count: record.message_count as usize,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }
}

/// Database record for a message
#[derive(Debug, Serialize, Deserialize)]
struct MessageRecord {
    id: Thing,
    conversation_id: String,
    role: String,
    content: String,
    tool_calls: Option<Vec<ToolCall>>,
    tool_results: Option<Vec<ToolResult>>,
    created_at: DateTime<Utc>,
}

impl MessageRecord {
    fn into_message(self) -> Message {
        // SurrealDB Thing.id can be in different formats, try to extract the UUID
        let id_str = self.id.id.to_string();
        let id_str = id_str.trim_matches('"').trim_matches('\'');
        let id = Uuid::parse_str(id_str).unwrap_or_else(|e| {
            tracing::warn!("Failed to parse message UUID '{}': {}", id_str, e);
            Uuid::nil()
        });

        let conv_id = Uuid::parse_str(&self.conversation_id).unwrap_or_else(|e| {
            tracing::warn!("Failed to parse conversation_id '{}': {}", self.conversation_id, e);
            Uuid::nil()
        });

        Message {
            id,
            conversation_id: conv_id,
            role: self.role.parse().unwrap_or(MessageRole::User),
            content: self.content,
            tool_calls: self.tool_calls,
            tool_results: self.tool_results,
            created_at: self.created_at,
        }
    }
}

#[async_trait]
impl ConversationStore for SurrealConversationStore {
    async fn create_conversation(
        &self,
        user_id: Option<&str>,
        title: Option<&str>,
    ) -> Result<Conversation, AppError> {
        let id = Uuid::new_v4();
        let title = title.unwrap_or("New Conversation");

        let record: Option<ConversationRecord> = self
            .db
            .create(("conversation", id.to_string()))
            .content(serde_json::json!({
                "user_id": user_id,
                "title": title,
                "message_count": 0,
            }))
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to create conversation: {}", e)))?;

        record
            .map(Conversation::from)
            .ok_or_else(|| AppError::DatabaseError("Failed to create conversation".to_string()))
    }

    async fn get_conversation(&self, id: Uuid) -> Result<Option<Conversation>, AppError> {
        let record: Option<ConversationRecord> = self
            .db
            .select(("conversation", id.to_string()))
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to get conversation: {}", e)))?;

        Ok(record.map(Conversation::from))
    }

    async fn list_conversations(
        &self,
        user_id: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Conversation>, AppError> {
        let query = if user_id.is_some() {
            format!(
                "SELECT * FROM conversation WHERE user_id = $user_id ORDER BY updated_at DESC LIMIT {} START {}",
                limit, offset
            )
        } else {
            format!(
                "SELECT * FROM conversation ORDER BY updated_at DESC LIMIT {} START {}",
                limit, offset
            )
        };

        let mut result = self
            .db
            .query(&query)
            .bind(("user_id", user_id.map(|s| s.to_string())))
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to list conversations: {}", e)))?;

        let records: Vec<ConversationRecord> = result
            .take(0)
            .map_err(|e| AppError::DatabaseError(format!("Failed to parse conversations: {}", e)))?;

        Ok(records.into_iter().map(Conversation::from).collect())
    }

    async fn update_conversation_title(&self, id: Uuid, title: &str) -> Result<(), AppError> {
        let _: Option<ConversationRecord> = self
            .db
            .update(("conversation", id.to_string()))
            .merge(serde_json::json!({
                "title": title,
                "updated_at": Utc::now(),
            }))
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to update conversation: {}", e)))?;

        Ok(())
    }

    async fn delete_conversation(&self, id: Uuid) -> Result<(), AppError> {
        // Delete all messages first
        self.db
            .query("DELETE FROM message WHERE conversation_id = $conv_id")
            .bind(("conv_id", id.to_string()))
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to delete messages: {}", e)))?;

        // Delete the conversation
        let _: Option<ConversationRecord> = self
            .db
            .delete(("conversation", id.to_string()))
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to delete conversation: {}", e)))?;

        Ok(())
    }

    async fn add_message(
        &self,
        conversation_id: Uuid,
        message: Message,
    ) -> Result<Uuid, AppError> {
        let id = message.id;

        let _: Option<MessageRecord> = self
            .db
            .create(("message", id.to_string()))
            .content(serde_json::json!({
                "conversation_id": conversation_id.to_string(),
                "role": message.role.to_string(),
                "content": message.content,
                "tool_calls": message.tool_calls,
                "tool_results": message.tool_results,
            }))
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to add message: {}", e)))?;

        // Update conversation message count and timestamp
        self.db
            .query(
                "UPDATE conversation SET message_count = message_count + 1, updated_at = time::now() WHERE id = $id",
            )
            .bind(("id", format!("conversation:{}", conversation_id)))
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to update conversation: {}", e)))?;

        Ok(id)
    }

    async fn get_messages(
        &self,
        conversation_id: Uuid,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Message>, AppError> {
        let query = format!(
            "SELECT * FROM message WHERE conversation_id = $conv_id ORDER BY created_at ASC LIMIT {} START {}",
            limit, offset
        );

        let mut result = self
            .db
            .query(&query)
            .bind(("conv_id", conversation_id.to_string()))
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to get messages: {}", e)))?;

        let records: Vec<MessageRecord> = result
            .take(0)
            .map_err(|e| AppError::DatabaseError(format!("Failed to parse messages: {}", e)))?;

        Ok(records.into_iter().map(MessageRecord::into_message).collect())
    }

    async fn get_recent_messages(
        &self,
        conversation_id: Uuid,
        limit: usize,
    ) -> Result<Vec<Message>, AppError> {
        // Get the most recent messages in reverse order, then reverse the result
        let query = format!(
            "SELECT * FROM message WHERE conversation_id = $conv_id ORDER BY created_at DESC LIMIT {}",
            limit
        );

        let mut result = self
            .db
            .query(&query)
            .bind(("conv_id", conversation_id.to_string()))
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to get recent messages: {}", e)))?;

        let records: Vec<MessageRecord> = result
            .take(0)
            .map_err(|e| AppError::DatabaseError(format!("Failed to parse messages: {}", e)))?;

        let mut messages: Vec<Message> =
            records.into_iter().map(MessageRecord::into_message).collect();
        messages.reverse(); // Return in chronological order
        Ok(messages)
    }

    async fn count_messages(&self, conversation_id: Uuid) -> Result<usize, AppError> {
        let mut result = self
            .db
            .query("SELECT count() FROM message WHERE conversation_id = $conv_id GROUP ALL")
            .bind(("conv_id", conversation_id.to_string()))
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to count messages: {}", e)))?;

        #[derive(Deserialize)]
        struct CountResult {
            count: i64,
        }

        let count: Option<CountResult> = result
            .take(0)
            .map_err(|e| AppError::DatabaseError(format!("Failed to parse count: {}", e)))?;

        Ok(count.map(|c| c.count as usize).unwrap_or(0))
    }
}

/// Database record for a memory
#[derive(Debug, Serialize, Deserialize)]
struct MemoryRecord {
    id: Thing,
    user_id: Option<String>,
    content: String,
    embedding: Vec<f32>,
    source_conversation: Option<String>,
    created_at: DateTime<Utc>,
}

#[async_trait]
impl LongTermMemory for SurrealConversationStore {
    async fn store_memory(
        &self,
        content: &str,
        embedding: Vec<f32>,
        user_id: Option<&str>,
        source_conversation: Option<Uuid>,
    ) -> Result<Uuid, AppError> {
        let id = Uuid::new_v4();

        let _: Option<MemoryRecord> = self
            .db
            .create(("memory", id.to_string()))
            .content(serde_json::json!({
                "user_id": user_id,
                "content": content,
                "embedding": embedding,
                "source_conversation": source_conversation.map(|id| id.to_string()),
            }))
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to store memory: {}", e)))?;

        Ok(id)
    }

    async fn recall(
        &self,
        query_embedding: Vec<f32>,
        user_id: Option<&str>,
        top_k: usize,
    ) -> Result<Vec<MemoryItem>, AppError> {
        // Use cosine similarity for vector search
        let query = if user_id.is_some() {
            format!(
                r#"
                SELECT *,
                    vector::similarity::cosine(embedding, $query_embedding) AS score
                FROM memory
                WHERE user_id = $user_id
                ORDER BY score DESC
                LIMIT {}
                "#,
                top_k
            )
        } else {
            format!(
                r#"
                SELECT *,
                    vector::similarity::cosine(embedding, $query_embedding) AS score
                FROM memory
                ORDER BY score DESC
                LIMIT {}
                "#,
                top_k
            )
        };

        let mut result = self
            .db
            .query(&query)
            .bind(("query_embedding", query_embedding))
            .bind(("user_id", user_id.map(|s| s.to_string())))
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to recall memories: {}", e)))?;

        #[derive(Deserialize)]
        struct MemoryWithScore {
            id: Thing,
            user_id: Option<String>,
            content: String,
            source_conversation: Option<String>,
            created_at: DateTime<Utc>,
            score: f32,
        }

        let records: Vec<MemoryWithScore> = result
            .take(0)
            .map_err(|e| AppError::DatabaseError(format!("Failed to parse memories: {}", e)))?;

        Ok(records
            .into_iter()
            .map(|r| MemoryItem {
                id: Uuid::parse_str(&r.id.id.to_string()).unwrap_or_else(|_| Uuid::new_v4()),
                content: r.content,
                user_id: r.user_id,
                source_conversation: r
                    .source_conversation
                    .and_then(|s| Uuid::parse_str(&s).ok()),
                relevance_score: r.score,
                created_at: r.created_at,
            })
            .collect())
    }

    async fn delete_memory(&self, id: Uuid) -> Result<(), AppError> {
        let _: Option<MemoryRecord> = self
            .db
            .delete(("memory", id.to_string()))
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to delete memory: {}", e)))?;

        Ok(())
    }

    async fn delete_user_memories(&self, user_id: &str) -> Result<usize, AppError> {
        let mut result = self
            .db
            .query("DELETE FROM memory WHERE user_id = $user_id RETURN BEFORE")
            .bind(("user_id", user_id.to_string()))
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to delete user memories: {}", e)))?;

        // Count deleted records
        let deleted: Vec<MemoryRecord> = result.take(0).unwrap_or_default();

        Ok(deleted.len())
    }
}
