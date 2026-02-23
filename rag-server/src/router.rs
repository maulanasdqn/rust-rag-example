use axum::{
    extract::{DefaultBodyLimit, Multipart, Path, State},
    response::sse::{Event, KeepAlive, Sse},
    routing::{delete, get, post},
    Json, Router,
};
use futures::stream::StreamExt;
use rag_errors::AppError;
use rag_types::{DeleteResponse, DocumentInfo, DocumentListResponse, QueryResponse, SourceInfo, UploadResponse};
use std::convert::Infallible;
use tokio_stream::wrappers::ReceiverStream;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use uuid::Uuid;

use crate::use_cases::AppState;

#[derive(serde::Deserialize)]
pub struct QueryRequest {
    pub question: String,
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

pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/api/documents/documents", post(upload_document_handler))
        .route("/api/documents/documents", get(list_documents_handler))
        .route("/api/documents/documents/:id", delete(delete_document_handler))
        .route("/api/query", post(query_handler))
        .route("/api/chat/stream", post(chat_stream_handler))
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024)) // 100MB limit
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
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
    let result = state.query_documents.execute(&request.question).await?;

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
    Json(request): Json<QueryRequest>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(100);

    tokio::spawn(async move {
        // Send sources first
        match state.query_documents.execute_stream(&request.question).await {
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
