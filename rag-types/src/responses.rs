use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub document_id: Uuid,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct QueryResponse {
    pub answer: String,
    pub sources: Vec<SourceInfo>,
}

#[derive(Debug, Serialize)]
pub struct SourceInfo {
    pub document_id: Uuid,
    pub source_file: String,
    pub excerpt: String,
    pub relevance_score: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct DocumentInfo {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct DocumentListResponse {
    pub documents: Vec<DocumentInfo>,
}

#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub message: String,
}
