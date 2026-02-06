use axum::{
    extract::{Multipart, Path, State},
    Json,
};
use common::AppError;
use rag::RagEngine;
use std::sync::Arc;
use uuid::Uuid;

use crate::dto::*;

pub type AppState = Arc<RagEngine>;

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

pub async fn upload_document(
    State(engine): State<AppState>,
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

        let document_id = engine.ingest_document(&content, &filename).await?;

        return Ok(Json(UploadResponse {
            document_id,
            message: format!("Document '{}' uploaded and processed successfully", filename),
        }));
    }

    Err(AppError::InvalidDocumentFormat(
        "No file provided".to_string(),
    ))
}

pub async fn query(
    State(engine): State<AppState>,
    Json(request): Json<QueryRequest>,
) -> Result<Json<QueryResponse>, AppError> {
    let response = engine.query(&request.question).await?;

    Ok(Json(QueryResponse {
        answer: response.answer,
        sources: response
            .sources
            .into_iter()
            .map(|s| SourceInfo {
                document_id: s.document_id,
                source_file: s.source_file,
                excerpt: truncate(&s.chunk_content, 200),
                relevance_score: s.score,
            })
            .collect(),
    }))
}

pub async fn list_documents(
    State(engine): State<AppState>,
) -> Result<Json<DocumentListResponse>, AppError> {
    let documents = engine.list_documents().await?;
    Ok(Json(DocumentListResponse { documents }))
}

pub async fn delete_document(
    State(engine): State<AppState>,
    Path(document_id): Path<Uuid>,
) -> Result<Json<DeleteResponse>, AppError> {
    engine.delete_document(document_id).await?;
    Ok(Json(DeleteResponse {
        message: format!("Document {} deleted successfully", document_id),
    }))
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}
