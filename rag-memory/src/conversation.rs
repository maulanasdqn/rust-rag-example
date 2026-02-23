//! Conversation management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A conversation containing multiple messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    /// Unique conversation ID
    pub id: Uuid,
    /// Optional user ID who owns this conversation
    pub user_id: Option<String>,
    /// Title of the conversation
    pub title: String,
    /// Number of messages in the conversation
    pub message_count: usize,
    /// When the conversation was created
    pub created_at: DateTime<Utc>,
    /// When the conversation was last updated
    pub updated_at: DateTime<Utc>,
}

impl Conversation {
    /// Create a new conversation
    pub fn new(user_id: Option<String>, title: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            title: title.unwrap_or_else(|| "New Conversation".to_string()),
            message_count: 0,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a conversation with a specific ID
    pub fn with_id(id: Uuid, user_id: Option<String>, title: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id,
            user_id,
            title: title.unwrap_or_else(|| "New Conversation".to_string()),
            message_count: 0,
            created_at: now,
            updated_at: now,
        }
    }

    /// Generate a title from the first user message
    pub fn generate_title_from_message(message: &str) -> String {
        let truncated: String = message.chars().take(50).collect();
        if message.len() > 50 {
            format!("{}...", truncated)
        } else {
            truncated
        }
    }
}
