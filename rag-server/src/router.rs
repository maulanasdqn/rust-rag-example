use axum::{
    extract::{DefaultBodyLimit, Multipart, Path, State},
    http::{header, HeaderName, HeaderValue, Method},
    middleware,
    response::sse::{Event, KeepAlive, Sse},
    routing::{delete, get, post},
    Json, Router,
};
use futures::stream::StreamExt;
use rag_config::SecuritySettings;
use rag_errors::AppError;
use rag_memory::{Conversation, Message};
use rag_types::{DeleteResponse, DocumentInfo, DocumentListResponse, QueryResponse, SourceInfo, UploadResponse};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use tokio_stream::wrappers::ReceiverStream;
use tower_http::{
    cors::{AllowOrigin, Any, CorsLayer},
    trace::TraceLayer,
};
use uuid::Uuid;

use crate::security::{auth_middleware, rate_limit_middleware, SecurityState};
use crate::use_cases::AppState;

#[derive(serde::Deserialize)]
pub struct QueryRequest {
    pub question: String,
}

#[derive(serde::Deserialize)]
pub struct ChatStreamRequest {
    pub question: String,
    #[serde(default)]
    pub messages: Vec<ChatMessageRequest>,
}

#[derive(serde::Deserialize, Clone)]
pub struct ChatMessageRequest {
    pub role: String,
    pub content: String,
}

#[derive(serde::Serialize)]
pub struct ChatStreamEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sources: Option<Vec<SourceInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub fn create_router(state: AppState, security_settings: SecuritySettings) -> Router {
    // Create security state
    let security_state = SecurityState::new(security_settings.clone());

    // Configure CORS based on settings
    let x_api_key = HeaderName::from_static("x-api-key");
    let cors = if security_settings.allowed_origins.is_empty() {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
            .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, x_api_key])
    } else {
        let origins: Vec<HeaderValue> = security_settings
            .allowed_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        CorsLayer::new()
            .allow_origin(AllowOrigin::list(origins))
            .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
            .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, HeaderName::from_static("x-api-key")])
    };

    // Clone security state for different middleware layers
    let security_for_rate_limit = security_state.clone();
    let security_for_auth = security_state.clone();

    Router::new()
        // Health check (no auth required)
        .route("/api/health", get(health_handler))
        // Document endpoints
        .route("/api/documents/documents", post(upload_document_handler))
        .route("/api/documents/documents", get(list_documents_handler))
        .route("/api/documents/documents/:id", delete(delete_document_handler))
        // Query endpoints
        .route("/api/query", post(query_handler))
        .route("/api/chat/stream", post(chat_stream_handler))
        // Conversation endpoints
        .route("/api/conversations", post(create_conversation_handler))
        .route("/api/conversations", get(list_conversations_handler))
        .route("/api/conversations/:id", get(get_conversation_handler))
        .route("/api/conversations/:id", delete(delete_conversation_handler))
        .route("/api/conversations/:id/messages", get(get_messages_handler))
        .route("/api/conversations/:id/messages", post(add_message_handler))
        // Tool endpoints
        .route("/api/tools", get(list_tools_handler))
        .route("/api/tools/:name/execute", post(execute_tool_handler))
        // Agent endpoints
        .route("/api/agents", get(list_agents_handler))
        .route("/api/agents/execute", post(execute_agent_handler))
        // Security middleware (applied in reverse order)
        .layer(middleware::from_fn_with_state(security_for_auth, auth_middleware))
        .layer(middleware::from_fn_with_state(security_for_rate_limit, rate_limit_middleware))
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024)) // 10MB limit (reduced from 100MB)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Health check endpoint (bypasses auth for monitoring)
async fn health_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn upload_document_handler(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, AppError> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::InvalidDocumentFormat(e.to_string()))?
    {
        let filename = field
            .file_name()
            .map(|s| s.to_string())
            .ok_or_else(|| AppError::InvalidDocumentFormat("No filename provided".to_string()))?;

        let content = field
            .bytes()
            .await
            .map_err(|e| AppError::InvalidDocumentFormat(e.to_string()))?;

        let document_id = state
            .upload_document
            .execute(&content, &filename)
            .await?;

        return Ok(Json(UploadResponse {
            document_id,
            message: format!("Document '{}' uploaded and processed successfully", filename),
        }));
    }

    Err(AppError::InvalidDocumentFormat("No file provided".to_string()))
}

async fn list_documents_handler(
    State(state): State<AppState>,
) -> Result<Json<DocumentListResponse>, AppError> {
    let document_ids = state.list_documents.execute().await?;

    let mut documents = Vec::new();
    for id in document_ids {
        if let Some(info) = state.query_documents.get_document_info(id).await? {
            documents.push(DocumentInfo {
                id: info.id,
                name: info.source_file,
            });
        }
    }

    Ok(Json(DocumentListResponse { documents }))
}

async fn delete_document_handler(
    State(state): State<AppState>,
    Path(document_id): Path<Uuid>,
) -> Result<Json<DeleteResponse>, AppError> {
    state.delete_document.execute(document_id).await?;
    Ok(Json(DeleteResponse {
        message: format!("Document {} deleted successfully", document_id),
    }))
}

async fn query_handler(
    State(state): State<AppState>,
    Json(request): Json<QueryRequest>,
) -> Result<Json<QueryResponse>, AppError> {
    // Validate input
    let validated = state.security.validate_query(&request.question)
        .map_err(|e| AppError::InvalidDocumentFormat(e.error))?;

    // Log if injection was detected (but still process the request)
    if let Some(warning) = &validated.injection_warning {
        tracing::warn!(warning = %warning, "Processing request despite injection warning");
    }

    let result = state.query_documents.execute(&validated.content).await?;

    Ok(Json(QueryResponse {
        answer: result.answer,
        sources: result
            .sources
            .into_iter()
            .map(|s| SourceInfo {
                document_id: s.document_id,
                source_file: s.source_file,
                excerpt: s.chunk_content,
                relevance_score: s.score,
            })
            .collect(),
    }))
}

async fn chat_stream_handler(
    State(state): State<AppState>,
    Json(request): Json<ChatStreamRequest>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(100);

    // Validate the question before spawning the async task
    let validated_question = match state.security.validate_query(&request.question) {
        Ok(v) => v,
        Err(e) => {
            // Send error event for invalid input
            let tx_clone = tx.clone();
            tokio::spawn(async move {
                let event = ChatStreamEvent {
                    event_type: "error".to_string(),
                    content: None,
                    sources: None,
                    error: Some(e.error),
                };
                let _ = tx_clone
                    .send(Ok(Event::default()
                        .json_data(&event)
                        .unwrap_or_else(|_| Event::default())))
                    .await;
            });
            return Sse::new(ReceiverStream::new(rx)).keep_alive(KeepAlive::default());
        }
    };

    // Validate all message contents
    let mut validated_messages = Vec::new();
    for msg in &request.messages {
        match state.security.validate_message(&msg.content) {
            Ok(v) => validated_messages.push((msg.role.clone(), v.content)),
            Err(e) => {
                let tx_clone = tx.clone();
                tokio::spawn(async move {
                    let event = ChatStreamEvent {
                        event_type: "error".to_string(),
                        content: None,
                        sources: None,
                        error: Some(format!("Invalid message content: {}", e.error)),
                    };
                    let _ = tx_clone
                        .send(Ok(Event::default()
                            .json_data(&event)
                            .unwrap_or_else(|_| Event::default())))
                        .await;
                });
                return Sse::new(ReceiverStream::new(rx)).keep_alive(KeepAlive::default());
            }
        }
    }

    // Log injection warnings
    if let Some(warning) = &validated_question.injection_warning {
        tracing::warn!(warning = %warning, "Processing chat request despite injection warning");
    }

    let question = validated_question.content;

    tokio::spawn(async move {
        // Convert messages to ChatMessage format (using validated content)
        let history: Vec<rag_inference::ChatMessage> = validated_messages
            .into_iter()
            .map(|(role, content)| match role.as_str() {
                "user" => rag_inference::ChatMessage::user(content),
                "assistant" => rag_inference::ChatMessage::assistant(content),
                // Don't allow arbitrary system messages from client - security risk
                "system" => {
                    tracing::warn!("Client attempted to send system message, converting to user message");
                    rag_inference::ChatMessage::user(content)
                },
                _ => rag_inference::ChatMessage::user(content),
            })
            .collect();

        // Use history-aware method if there's history, otherwise use regular method
        let result = if history.is_empty() {
            state.query_documents.execute_stream(&question).await
        } else {
            state.query_documents.execute_stream_with_history(&question, history).await
        };

        match result {
            Ok((mut stream, sources)) => {
                // Send sources event
                let sources_event = ChatStreamEvent {
                    event_type: "sources".to_string(),
                    content: None,
                    sources: Some(
                        sources
                            .iter()
                            .map(|s| SourceInfo {
                                document_id: s.document_id,
                                source_file: s.source_file.clone(),
                                excerpt: s.chunk_content.clone(),
                                relevance_score: s.score,
                            })
                            .collect(),
                    ),
                    error: None,
                };
                let _ = tx
                    .send(Ok(Event::default()
                        .json_data(&sources_event)
                        .unwrap_or_else(|_| Event::default())))
                    .await;

                // Stream content chunks
                while let Some(result) = stream.next().await {
                    match result {
                        Ok(content) => {
                            let event = ChatStreamEvent {
                                event_type: "content".to_string(),
                                content: Some(content),
                                sources: None,
                                error: None,
                            };
                            if tx
                                .send(Ok(Event::default()
                                    .json_data(&event)
                                    .unwrap_or_else(|_| Event::default())))
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                        Err(e) => {
                            let event = ChatStreamEvent {
                                event_type: "error".to_string(),
                                content: None,
                                sources: None,
                                error: Some(e.to_string()),
                            };
                            let _ = tx
                                .send(Ok(Event::default()
                                    .json_data(&event)
                                    .unwrap_or_else(|_| Event::default())))
                                .await;
                            break;
                        }
                    }
                }

                // Send done event
                let done_event = ChatStreamEvent {
                    event_type: "done".to_string(),
                    content: None,
                    sources: None,
                    error: None,
                };
                let _ = tx
                    .send(Ok(Event::default()
                        .json_data(&done_event)
                        .unwrap_or_else(|_| Event::default())))
                    .await;
            }
            Err(e) => {
                let event = ChatStreamEvent {
                    event_type: "error".to_string(),
                    content: None,
                    sources: None,
                    error: Some(e.to_string()),
                };
                let _ = tx
                    .send(Ok(Event::default()
                        .json_data(&event)
                        .unwrap_or_else(|_| Event::default())))
                    .await;
            }
        }
    });

    Sse::new(ReceiverStream::new(rx)).keep_alive(KeepAlive::default())
}

// ============= Conversation Endpoints =============

#[derive(Deserialize)]
pub struct CreateConversationRequest {
    pub title: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Serialize)]
pub struct ConversationResponse {
    pub id: Uuid,
    pub title: String,
    pub user_id: Option<String>,
    pub message_count: usize,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Conversation> for ConversationResponse {
    fn from(c: Conversation) -> Self {
        Self {
            id: c.id,
            title: c.title,
            user_id: c.user_id,
            message_count: c.message_count,
            created_at: c.created_at.to_rfc3339(),
            updated_at: c.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Serialize)]
pub struct ConversationListResponse {
    pub conversations: Vec<ConversationResponse>,
}

async fn create_conversation_handler(
    State(state): State<AppState>,
    Json(request): Json<CreateConversationRequest>,
) -> Result<Json<ConversationResponse>, AppError> {
    let conversation = state
        .conversation_store
        .create_conversation(request.user_id.as_deref(), request.title.as_deref())
        .await?;

    Ok(Json(conversation.into()))
}

async fn list_conversations_handler(
    State(state): State<AppState>,
) -> Result<Json<ConversationListResponse>, AppError> {
    let conversations = state
        .conversation_store
        .list_conversations(None, 50, 0)
        .await?;

    Ok(Json(ConversationListResponse {
        conversations: conversations.into_iter().map(Into::into).collect(),
    }))
}

async fn get_conversation_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ConversationResponse>, AppError> {
    let conversation = state
        .conversation_store
        .get_conversation(id)
        .await?
        .ok_or_else(|| AppError::DocumentNotFound(format!("Conversation {} not found", id)))?;

    Ok(Json(conversation.into()))
}

async fn delete_conversation_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<DeleteResponse>, AppError> {
    state.conversation_store.delete_conversation(id).await?;
    Ok(Json(DeleteResponse {
        message: format!("Conversation {} deleted successfully", id),
    }))
}

#[derive(Serialize)]
pub struct MessageResponse {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

impl From<Message> for MessageResponse {
    fn from(m: Message) -> Self {
        Self {
            id: m.id,
            conversation_id: m.conversation_id,
            role: m.role.to_string(),
            content: m.content,
            created_at: m.created_at.to_rfc3339(),
        }
    }
}

#[derive(Serialize)]
pub struct MessagesResponse {
    pub messages: Vec<MessageResponse>,
}

async fn get_messages_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<MessagesResponse>, AppError> {
    let messages = state
        .conversation_store
        .get_messages(id, 100, 0)
        .await?;

    Ok(Json(MessagesResponse {
        messages: messages.into_iter().map(Into::into).collect(),
    }))
}

#[derive(Deserialize)]
pub struct AddMessageRequest {
    pub role: String,
    pub content: String,
}

async fn add_message_handler(
    State(state): State<AppState>,
    Path(conversation_id): Path<Uuid>,
    Json(request): Json<AddMessageRequest>,
) -> Result<Json<MessageResponse>, AppError> {
    let role: rag_memory::MessageRole = request
        .role
        .parse()
        .map_err(|_| AppError::InvalidDocumentFormat(format!("Invalid role: {}", request.role)))?;

    let message = Message {
        id: Uuid::new_v4(),
        conversation_id,
        role,
        content: request.content,
        tool_calls: None,
        tool_results: None,
        created_at: chrono::Utc::now(),
    };

    state
        .conversation_store
        .add_message(conversation_id, message.clone())
        .await?;

    Ok(Json(message.into()))
}

// ============= Tool Endpoints =============

#[derive(Serialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Serialize)]
pub struct ToolListResponse {
    pub tools: Vec<ToolInfo>,
}

async fn list_tools_handler(
    State(state): State<AppState>,
) -> Result<Json<ToolListResponse>, AppError> {
    let tools: Vec<ToolInfo> = state
        .tool_registry
        .all_tools()
        .iter()
        .map(|t| ToolInfo {
            name: t.name().to_string(),
            description: t.description().to_string(),
            parameters: t.parameters_schema(),
        })
        .collect();

    Ok(Json(ToolListResponse { tools }))
}

#[derive(Deserialize)]
pub struct ExecuteToolRequest {
    pub arguments: serde_json::Value,
}

#[derive(Serialize)]
pub struct ToolExecutionResponse {
    pub success: bool,
    pub output: String,
    pub data: Option<serde_json::Value>,
}

async fn execute_tool_handler(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(request): Json<ExecuteToolRequest>,
) -> Result<Json<ToolExecutionResponse>, AppError> {
    let tool = state
        .tool_registry
        .get(&name)
        .ok_or_else(|| AppError::DocumentNotFound(format!("Tool '{}' not found", name)))?;

    let result = tool
        .execute(request.arguments)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(ToolExecutionResponse {
        success: result.success,
        output: result.output,
        data: result.data,
    }))
}

// ============= Agent Endpoints =============

#[derive(Serialize)]
pub struct AgentInfo {
    pub name: String,
    pub description: String,
}

#[derive(Serialize)]
pub struct AgentListResponse {
    pub agents: Vec<AgentInfo>,
}

async fn list_agents_handler(
    State(state): State<AppState>,
) -> Result<Json<AgentListResponse>, AppError> {
    let agents: Vec<AgentInfo> = state
        .agent_coordinator
        .get_agent_info()
        .into_iter()
        .map(|info| AgentInfo {
            name: info.name,
            description: info.description,
        })
        .collect();

    Ok(Json(AgentListResponse { agents }))
}

#[derive(Deserialize)]
pub struct ExecuteAgentRequest {
    pub query: String,
    pub agent_name: Option<String>,
    pub conversation_id: Option<Uuid>,
}

#[derive(Serialize)]
pub struct AgentExecutionResponse {
    pub success: bool,
    pub answer: String,
    pub reasoning: Vec<ReasoningStepResponse>,
    pub sources: Vec<SourceInfo>,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct ReasoningStepResponse {
    pub step_type: String,
    pub content: String,
}

async fn execute_agent_handler(
    State(state): State<AppState>,
    Json(request): Json<ExecuteAgentRequest>,
) -> Result<Json<AgentExecutionResponse>, AppError> {
    use rag_agents::AgentContext;

    // Validate query
    let validated = state.security.validate_query(&request.query)
        .map_err(|e| AppError::InvalidDocumentFormat(e.error))?;

    if let Some(warning) = &validated.injection_warning {
        tracing::warn!(warning = %warning, "Processing agent request despite injection warning");
    }

    // Build context
    let mut context = if let Some(conv_id) = request.conversation_id {
        AgentContext::with_conversation(conv_id)
    } else {
        AgentContext::new()
    };

    // Load recent messages if we have a conversation
    if let Some(conv_id) = request.conversation_id {
        let messages = state
            .conversation_store
            .get_recent_messages(conv_id, 10)
            .await?;
        context = context.with_messages(messages);
    }

    // Execute the agent with validated query
    let result = if let Some(agent_name) = request.agent_name {
        state
            .agent_coordinator
            .execute_with_agent(&agent_name, &context, &validated.content)
            .await?
    } else {
        state
            .agent_coordinator
            .execute(&context, &validated.content)
            .await?
    };

    Ok(Json(AgentExecutionResponse {
        success: result.success,
        answer: result.answer,
        reasoning: result
            .reasoning
            .into_iter()
            .map(|step| ReasoningStepResponse {
                step_type: format!("{:?}", step.step_type).to_lowercase(),
                content: step.content,
            })
            .collect(),
        sources: result.sources.into_iter().map(|s| SourceInfo {
            document_id: s.document_id,
            source_file: s.source_file,
            excerpt: s.excerpt,
            relevance_score: s.relevance_score,
        }).collect(),
        error: result.error,
    }))
}
