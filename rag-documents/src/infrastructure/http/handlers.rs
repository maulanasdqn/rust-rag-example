use axum::{
    extract::{Multipart, Path},
    Json,
};
use rag_errors::AppError;
use rag_types::{DeleteResponse, DocumentListResponse, UploadResponse};
use uuid::Uuid;

pub async fn upload_document_handler(
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

        let _content = field
            .bytes()
            .await
            .map_err(|e| AppError::InvalidDocumentFormat(e.to_string()))?;

        return Ok(Json(UploadResponse {
            document_id: Uuid::new_v4(),
            message: format!("Document '{}' uploaded and processed successfully", filename),
        }));
    }

    Err(AppError::InvalidDocumentFormat(
        "No file provided".to_string(),
    ))
}

pub async fn list_documents_handler() -> Result<Json<DocumentListResponse>, AppError> {
    Ok(Json(DocumentListResponse {
        documents: Vec::new(),
    }))
}

pub async fn delete_document_handler(
    Path(document_id): Path<Uuid>,
) -> Result<Json<DeleteResponse>, AppError> {
    Ok(Json(DeleteResponse {
        message: format!("Document {} deleted successfully", document_id),
    }))
}
