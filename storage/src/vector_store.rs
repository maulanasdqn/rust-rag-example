use async_trait::async_trait;
use common::types::DocumentChunk;
use common::AppError;
use uuid::Uuid;

use crate::models::DocumentInfo;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub chunk: DocumentChunk,
    pub score: f32,
}

#[async_trait]
pub trait VectorStore: Send + Sync {
    async fn store_chunks(
        &self,
        chunks: Vec<DocumentChunk>,
        embeddings: Vec<Vec<f32>>,
    ) -> Result<(), AppError>;

    async fn search(
        &self,
        query_embedding: Vec<f32>,
        top_k: usize,
    ) -> Result<Vec<SearchResult>, AppError>;

    async fn delete_document(&self, document_id: Uuid) -> Result<(), AppError>;

    async fn list_documents(&self) -> Result<Vec<Uuid>, AppError>;

    async fn get_document_info(&self, document_id: Uuid) -> Result<Option<DocumentInfo>, AppError>;
}
