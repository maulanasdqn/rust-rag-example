//! Context window management for conversations

use crate::Message;

/// Manages the context window for LLM calls
pub struct ContextManager {
    /// Maximum tokens allowed in context
    max_tokens: usize,
    /// Reserve tokens for the response
    response_reserve: usize,
}

impl Default for ContextManager {
    fn default() -> Self {
        Self {
            max_tokens: 8000,    // Default for GPT-4
            response_reserve: 1000,
        }
    }
}

impl ContextManager {
    /// Create a new context manager
    pub fn new(max_tokens: usize, response_reserve: usize) -> Self {
        Self {
            max_tokens,
            response_reserve,
        }
    }

    /// Calculate available tokens for context
    pub fn available_tokens(&self) -> usize {
        self.max_tokens.saturating_sub(self.response_reserve)
    }

    /// Trim messages to fit within the context window
    /// Keeps the system message and most recent messages
    pub fn trim_messages(&self, messages: Vec<Message>, system_prompt: &str) -> Vec<Message> {
        let available = self.available_tokens();
        let system_tokens = system_prompt.len() / 4 + 10; // Rough estimate

        if system_tokens >= available {
            // System prompt alone exceeds limit, return empty
            return vec![];
        }

        let remaining = available - system_tokens;
        let mut trimmed = Vec::new();
        let mut used_tokens = 0;

        // Process messages from most recent to oldest
        for message in messages.into_iter().rev() {
            let msg_tokens = message.estimate_tokens();
            if used_tokens + msg_tokens <= remaining {
                used_tokens += msg_tokens;
                trimmed.push(message);
            } else {
                break;
            }
        }

        // Reverse to get chronological order
        trimmed.reverse();
        trimmed
    }

    /// Build context messages for an LLM call
    pub fn build_context(
        &self,
        messages: Vec<Message>,
        system_prompt: &str,
        additional_context: Option<&str>,
    ) -> Vec<ContextMessage> {
        let mut context_messages = vec![ContextMessage {
            role: "system".to_string(),
            content: if let Some(ctx) = additional_context {
                format!("{}\n\n{}", system_prompt, ctx)
            } else {
                system_prompt.to_string()
            },
        }];

        let trimmed = self.trim_messages(messages, system_prompt);

        for msg in trimmed {
            context_messages.push(ContextMessage {
                role: msg.role.to_string(),
                content: msg.content,
            });
        }

        context_messages
    }
}

/// A simplified message for LLM context
#[derive(Debug, Clone)]
pub struct ContextMessage {
    pub role: String,
    pub content: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_trim_messages_within_limit() {
        let manager = ContextManager::new(1000, 100);
        let conv_id = Uuid::new_v4();

        let messages = vec![
            Message::user(conv_id, "Hello"),
            Message::assistant(conv_id, "Hi there!"),
        ];

        let trimmed = manager.trim_messages(messages.clone(), "You are a helpful assistant.");
        assert_eq!(trimmed.len(), 2);
    }

    #[test]
    fn test_trim_messages_exceeds_limit() {
        let manager = ContextManager::new(100, 20);
        let conv_id = Uuid::new_v4();

        let messages = vec![
            Message::user(conv_id, "A".repeat(200)),
            Message::assistant(conv_id, "B".repeat(200)),
            Message::user(conv_id, "Short"),
        ];

        let trimmed = manager.trim_messages(messages, "System");
        // Only the most recent messages that fit should be kept
        assert!(trimmed.len() < 3);
    }
}
