use crate::traits::{
    ChatCompletionResponse, ChatMessage, FinishReason, FunctionCall, MessageRole,
    ToolCallRequest, ToolDefinition,
};
use crate::LlmProvider;
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionMessageToolCall, ChatCompletionRequestAssistantMessageArgs,
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestToolMessageArgs, ChatCompletionRequestUserMessageArgs,
        ChatCompletionTool, ChatCompletionToolArgs, ChatCompletionToolType,
        CreateChatCompletionRequestArgs, FunctionObject,
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
    max_tokens: Option<u32>,
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
            max_tokens: None,
        }
    }

    /// Create a new LLM provider with max_tokens limit for cost control
    pub fn with_max_tokens(api_key: &str, api_base: &str, model: &str, system_prompt: &str, max_tokens: u32) -> Self {
        let config = OpenAIConfig::new()
            .with_api_key(api_key)
            .with_api_base(api_base);
        let client = Client::with_config(config);
        Self {
            client,
            model: model.to_string(),
            system_prompt: system_prompt.to_string(),
            max_tokens: Some(max_tokens),
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

        let mut request_builder = CreateChatCompletionRequestArgs::default();
        request_builder.model(&self.model).messages(messages);

        // Apply max_tokens limit for cost control
        if let Some(max_tokens) = self.max_tokens {
            request_builder.max_tokens(max_tokens);
        }

        let request = request_builder
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

        let mut request_builder = CreateChatCompletionRequestArgs::default();
        request_builder.model(&self.model).messages(messages);

        // Apply max_tokens limit for cost control
        if let Some(max_tokens) = self.max_tokens {
            request_builder.max_tokens(max_tokens);
        }

        let request = request_builder
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

    #[instrument(skip(self, context, history), fields(context_count = context.len(), history_count = history.len()))]
    async fn generate_stream_with_history(
        &self,
        query: &str,
        context: &[SearchResult],
        history: Vec<ChatMessage>,
    ) -> Result<BoxStream<'static, Result<String, AppError>>, AppError> {
        let context_text = context
            .iter()
            .enumerate()
            .map(|(i, r)| format!("[{}] {}", i + 1, r.chunk.content))
            .collect::<Vec<_>>()
            .join("\n\n");

        // Build messages with history
        let mut messages: Vec<ChatCompletionRequestMessage> = vec![
            ChatCompletionRequestSystemMessageArgs::default()
                .content(&self.system_prompt)
                .build()
                .map_err(|e| AppError::OpenAIError(e.to_string()))?
                .into(),
        ];

        // Add conversation history
        for msg in history {
            messages.push(self.convert_message(msg)?);
        }

        // Add the current query with context
        let user_message = format!("Context:\n{}\n\nQuestion: {}", context_text, query);
        messages.push(
            ChatCompletionRequestUserMessageArgs::default()
                .content(user_message)
                .build()
                .map_err(|e| AppError::OpenAIError(e.to_string()))?
                .into(),
        );

        let mut request_builder = CreateChatCompletionRequestArgs::default();
        request_builder.model(&self.model).messages(messages);

        // Apply max_tokens limit for cost control
        if let Some(max_tokens) = self.max_tokens {
            request_builder.max_tokens(max_tokens);
        }

        let request = request_builder
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

    #[instrument(skip(self, messages, tools), fields(message_count = messages.len(), tool_count = tools.len()))]
    async fn generate_with_tools(
        &self,
        messages: Vec<ChatMessage>,
        tools: Vec<ToolDefinition>,
    ) -> Result<ChatCompletionResponse, AppError> {
        // Convert our messages to async-openai messages
        let openai_messages: Vec<ChatCompletionRequestMessage> = messages
            .into_iter()
            .map(|msg| self.convert_message(msg))
            .collect::<Result<Vec<_>, _>>()?;

        // Convert tools to async-openai format
        let openai_tools: Vec<ChatCompletionTool> = tools
            .iter()
            .filter_map(|tool| {
                ChatCompletionToolArgs::default()
                    .r#type(ChatCompletionToolType::Function)
                    .function(FunctionObject {
                        name: tool.function.name.clone(),
                        description: Some(tool.function.description.clone()),
                        parameters: Some(tool.function.parameters.clone()),
                    })
                    .build()
                    .ok()
            })
            .collect();

        // Build the request
        let mut request_builder = CreateChatCompletionRequestArgs::default();
        request_builder.model(&self.model).messages(openai_messages);

        if !openai_tools.is_empty() {
            request_builder.tools(openai_tools);
        }

        // Apply max_tokens limit for cost control
        if let Some(max_tokens) = self.max_tokens {
            request_builder.max_tokens(max_tokens);
        }

        let request = request_builder
            .build()
            .map_err(|e| AppError::OpenAIError(e.to_string()))?;

        // Make the API call
        let response = self
            .client
            .chat()
            .create(request)
            .await
            .map_err(|e| AppError::OpenAIError(e.to_string()))?;

        // Extract the response
        let choice = response
            .choices
            .first()
            .ok_or_else(|| AppError::OpenAIError("No response choices".to_string()))?;

        let content = choice.message.content.clone().unwrap_or_default();

        // Extract tool calls if present
        let tool_calls: Vec<ToolCallRequest> = choice
            .message
            .tool_calls
            .as_ref()
            .map(|calls| {
                calls
                    .iter()
                    .map(|call| ToolCallRequest {
                        id: call.id.clone(),
                        r#type: "function".to_string(),
                        function: FunctionCall {
                            name: call.function.name.clone(),
                            arguments: call.function.arguments.clone(),
                        },
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Determine finish reason
        let finish_reason = match choice.finish_reason {
            Some(async_openai::types::FinishReason::Stop) => FinishReason::Stop,
            Some(async_openai::types::FinishReason::ToolCalls) => FinishReason::ToolCalls,
            Some(async_openai::types::FinishReason::Length) => FinishReason::Length,
            Some(async_openai::types::FinishReason::ContentFilter) => FinishReason::ContentFilter,
            _ => {
                if !tool_calls.is_empty() {
                    FinishReason::ToolCalls
                } else {
                    FinishReason::Stop
                }
            }
        };

        Ok(ChatCompletionResponse {
            content,
            tool_calls,
            finish_reason,
        })
    }
}

impl OpenAILlm {
    fn convert_message(&self, msg: ChatMessage) -> Result<ChatCompletionRequestMessage, AppError> {
        match msg.role {
            MessageRole::System => Ok(ChatCompletionRequestSystemMessageArgs::default()
                .content(&msg.content)
                .build()
                .map_err(|e| AppError::OpenAIError(e.to_string()))?
                .into()),
            MessageRole::User => Ok(ChatCompletionRequestUserMessageArgs::default()
                .content(msg.content.clone())
                .build()
                .map_err(|e| AppError::OpenAIError(e.to_string()))?
                .into()),
            MessageRole::Assistant => {
                let mut builder = ChatCompletionRequestAssistantMessageArgs::default();
                builder.content(&msg.content);

                // Add tool calls if present
                if let Some(tool_calls) = msg.tool_calls {
                    let openai_tool_calls: Vec<ChatCompletionMessageToolCall> = tool_calls
                        .into_iter()
                        .map(|tc| ChatCompletionMessageToolCall {
                            id: tc.id,
                            r#type: ChatCompletionToolType::Function,
                            function: async_openai::types::FunctionCall {
                                name: tc.function.name,
                                arguments: tc.function.arguments,
                            },
                        })
                        .collect();

                    if !openai_tool_calls.is_empty() {
                        builder.tool_calls(openai_tool_calls);
                    }
                }

                Ok(builder
                    .build()
                    .map_err(|e| AppError::OpenAIError(e.to_string()))?
                    .into())
            }
            MessageRole::Tool => {
                let tool_call_id = msg
                    .tool_call_id
                    .ok_or_else(|| AppError::OpenAIError("Tool message requires tool_call_id".to_string()))?;

                Ok(ChatCompletionRequestToolMessageArgs::default()
                    .tool_call_id(&tool_call_id)
                    .content(&msg.content)
                    .build()
                    .map_err(|e| AppError::OpenAIError(e.to_string()))?
                    .into())
            }
        }
    }
}
