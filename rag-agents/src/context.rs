//! Agent execution context

use rag_memory::{traits::MemoryItem, Message};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::traits::SourceInfo;

/// Context for agent execution
#[derive(Debug, Clone, Default)]
pub struct AgentContext {
    /// Current conversation ID (if any)
    pub conversation_id: Option<Uuid>,
    /// User ID (if any)
    pub user_id: Option<String>,
    /// Previous messages in the conversation
    pub messages: Vec<Message>,
    /// Relevant memories from long-term storage
    pub memories: Vec<MemoryItem>,
    /// Documents retrieved for this query
    pub documents: Vec<SearchResult>,
    /// Additional context as key-value pairs
    pub metadata: std::collections::HashMap<String, String>,
}

impl AgentContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a context with a conversation ID
    pub fn with_conversation(conversation_id: Uuid) -> Self {
        Self {
            conversation_id: Some(conversation_id),
            ..Default::default()
        }
    }

    /// Add a user ID to the context
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Add messages to the context
    pub fn with_messages(mut self, messages: Vec<Message>) -> Self {
        self.messages = messages;
        self
    }

    /// Add memories to the context
    pub fn with_memories(mut self, memories: Vec<MemoryItem>) -> Self {
        self.memories = memories;
        self
    }

    /// Add documents to the context
    pub fn with_documents(mut self, documents: Vec<SearchResult>) -> Self {
        self.documents = documents;
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Get formatted context for the LLM
    pub fn format_for_llm(&self) -> String {
        let mut parts = Vec::new();

        // Add document context
        if !self.documents.is_empty() {
            parts.push("## Relevant Documents\n".to_string());
            for (i, doc) in self.documents.iter().enumerate() {
                parts.push(format!(
                    "### Source {}: {} (relevance: {:.2})\n{}\n",
                    i + 1,
                    doc.source_file,
                    doc.score,
                    doc.content
                ));
            }
        }

        // Add memory context
        if !self.memories.is_empty() {
            parts.push("\n## Relevant Memories\n".to_string());
            for memory in &self.memories {
                parts.push(format!("- {}\n", memory.content));
            }
        }

        parts.join("\n")
    }

    /// Convert documents to source info
    pub fn get_source_info(&self) -> Vec<SourceInfo> {
        self.documents
            .iter()
            .map(|doc| SourceInfo {
                document_id: doc.document_id,
                source_file: doc.source_file.clone(),
                excerpt: doc.content.chars().take(200).collect(),
                relevance_score: doc.score,
            })
            .collect()
    }
}

/// A document search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub document_id: Uuid,
    pub source_file: String,
    pub content: String,
    pub score: f32,
}
