use async_trait::async_trait;
use rag_errors::AppError;
use rag_types::DocumentChunk;
use uuid::Uuid;

#[async_trait]
pub trait DocumentRepository: Send + Sync {
    async fn store(&self, chunks: Vec<DocumentChunk>, embeddings: Vec<Vec<f32>>) -> Result<(), AppError>;
    async fn delete(&self, document_id: Uuid) -> Result<(), AppError>;
    async fn list(&self) -> Result<Vec<Uuid>, AppError>;
}
