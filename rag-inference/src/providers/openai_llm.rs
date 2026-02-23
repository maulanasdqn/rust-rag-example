use crate::LlmProvider;
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
    },
    Client,
};
use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use rag_database::SearchResult;
use rag_errors::AppError;
use tracing::instrument;

pub struct OpenAILlm {
    client: Client<OpenAIConfig>,
    model: String,
    system_prompt: String,
}

impl OpenAILlm {
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

    fn build_messages(
        &self,
        query: &str,
        context: &[SearchResult],
    ) -> Result<Vec<ChatCompletionRequestMessage>, AppError> {
        let context_text = context
            .iter()
            .enumerate()
            .map(|(i, r)| format!("[{}] {}", i + 1, r.chunk.content))
            .collect::<Vec<_>>()
            .join("\n\n");

        let user_message = format!("Context:\n{}\n\nQuestion: {}", context_text, query);

        Ok(vec![
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
        ])
    }
}

#[async_trait]
impl LlmProvider for OpenAILlm {
    #[instrument(skip(self, context), fields(context_count = context.len()))]
    async fn generate(&self, query: &str, context: &[SearchResult]) -> Result<String, AppError> {
        let messages = self.build_messages(query, context)?;

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

    #[instrument(skip(self, context), fields(context_count = context.len()))]
    async fn generate_stream(
        &self,
        query: &str,
        context: &[SearchResult],
    ) -> Result<BoxStream<'static, Result<String, AppError>>, AppError> {
        let messages = self.build_messages(query, context)?;

        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model)
            .messages(messages)
            .build()
            .map_err(|e| AppError::OpenAIError(e.to_string()))?;

        let stream = self
            .client
            .chat()
            .create_stream(request)
            .await
            .map_err(|e| AppError::OpenAIError(e.to_string()))?;

        let mapped_stream = stream.map(|result| {
            result
                .map_err(|e| AppError::OpenAIError(e.to_string()))
                .and_then(|response| {
                    response
                        .choices
                        .first()
                        .and_then(|c| c.delta.content.clone())
                        .ok_or_else(|| AppError::OpenAIError("No content".to_string()))
                })
        });

        Ok(Box::pin(mapped_stream))
    }
}
