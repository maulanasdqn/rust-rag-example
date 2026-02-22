use crate::application::LlmProvider;
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
    },
    Client,
};
use async_trait::async_trait;
use rag_database::SearchResult;
use rag_errors::AppError;
use tracing::instrument;

pub struct LlmService {
    client: Client<OpenAIConfig>,
    model: String,
    system_prompt: String,
}

impl LlmService {
    pub fn new(api_key: &str, api_base: &str, model: &str, system_prompt: &str) -> Self {
        let config = OpenAIConfig::new()
            .with_api_key(api_key)
            .with_api_base(api_base);
        let client = Client::with_config(config);
        Self {
            client,
            model: model.to_string(),
            system_prompt: system_prompt.to_string(),
        }
    }
}

#[async_trait]
impl LlmProvider for LlmService {
    #[instrument(skip(self, context), fields(context_count = context.len()))]
    async fn generate(&self, query: &str, context: &[SearchResult]) -> Result<String, AppError> {
        let context_text = context
            .iter()
            .enumerate()
            .map(|(i, r)| format!("[{}] {}", i + 1, r.chunk.content))
            .collect::<Vec<_>>()
            .join("\n\n");

        let user_message = format!("Context:\n{}\n\nQuestion: {}", context_text, query);

        let messages: Vec<ChatCompletionRequestMessage> = vec![
            ChatCompletionRequestSystemMessageArgs::default()
                .content(&self.system_prompt)
                .build()
                .map_err(|e| AppError::OpenAIError(e.to_string()))?
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(user_message)
                .build()
                .map_err(|e| AppError::OpenAIError(e.to_string()))?
                .into(),
        ];

        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model)
            .messages(messages)
            .build()
            .map_err(|e| AppError::OpenAIError(e.to_string()))?;

        let response = self
            .client
            .chat()
            .create(request)
            .await
            .map_err(|e| AppError::OpenAIError(e.to_string()))?;

        response
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .ok_or_else(|| AppError::OpenAIError("No response content".to_string()))
    }
}
