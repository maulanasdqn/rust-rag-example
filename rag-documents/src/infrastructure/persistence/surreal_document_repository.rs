use crate::domain::DocumentRepository;
use async_trait::async_trait;
use rag_database::{SurrealVectorStore, VectorStore};
use rag_errors::AppError;
use rag_types::DocumentChunk;
use std::sync::Arc;
use uuid::Uuid;

pub struct SurrealDocumentRepository {
    vector_store: Arc<SurrealVectorStore>,
}

impl SurrealDocumentRepository {
    pub fn new(vector_store: Arc<SurrealVectorStore>) -> Self {
        Self { vector_store }
    }
}

#[async_trait]
impl DocumentRepository for SurrealDocumentRepository {
    async fn store(&self, chunks: Vec<DocumentChunk>, embeddings: Vec<Vec<f32>>) -> Result<(), AppError> {
        self.vector_store.store_chunks(chunks, embeddings).await
    }

    async fn delete(&self, document_id: Uuid) -> Result<(), AppError> {
        self.vector_store.delete_document(document_id).await
    }

    async fn list(&self) -> Result<Vec<Uuid>, AppError> {
        self.vector_store.list_documents().await
    }
}
