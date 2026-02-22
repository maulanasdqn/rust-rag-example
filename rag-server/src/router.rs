use axum::{
    extract::State,
    routing::{delete, get, post},
    Json, Router,
};
use axum::extract::{Multipart, Path};
use rag_errors::AppError;
use rag_types::{DeleteResponse, DocumentListResponse, QueryResponse, SourceInfo, UploadResponse};
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
    let documents = state.list_documents.execute().await?;
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
