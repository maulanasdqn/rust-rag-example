use crate::domain::{DocumentLoader, DocumentRepository};
use rag_errors::AppError;
use rag_inference::EmbeddingProvider;
use std::sync::Arc;
use tracing::instrument;
use uuid::Uuid;

pub struct UploadDocument<L: DocumentLoader, R: DocumentRepository, E: EmbeddingProvider> {
    loader: Arc<L>,
    repository: Arc<R>,
    embedding_provider: Arc<E>,
    chunk_size: usize,
    chunk_overlap: usize,
}

impl<L: DocumentLoader, R: DocumentRepository, E: EmbeddingProvider> UploadDocument<L, R, E> {
    pub fn new(
        loader: Arc<L>,
        repository: Arc<R>,
        embedding_provider: Arc<E>,
        chunk_size: usize,
        chunk_overlap: usize,
    ) -> Self {
        Self {
            loader,
            repository,
            embedding_provider,
            chunk_size,
            chunk_overlap,
        }
    }

    #[instrument(skip(self, content), fields(filename = %filename))]
    pub async fn execute(&self, content: &[u8], filename: &str) -> Result<Uuid, AppError> {
        let document = self.loader.load_bytes(content, filename)?;
        let document_id = Uuid::new_v4();
        let chunks = document.chunk(document_id, self.chunk_size, self.chunk_overlap);

        if chunks.is_empty() {
            return Err(AppError::InvalidDocumentFormat(
                "Document produced no chunks".to_string(),
            ));
        }

        let texts: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
        let embeddings = self.embedding_provider.embed_texts(texts).await?;

        self.repository.store(chunks, embeddings).await?;

        Ok(document_id)
    }
}
