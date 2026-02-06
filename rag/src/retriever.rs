use common::AppError;
use storage::{SearchResult, VectorStore};
use tracing::instrument;

use crate::EmbeddingService;

pub struct Retriever<V: VectorStore> {
    vector_store: V,
    embedding_service: EmbeddingService,
    top_k: usize,
}

impl<V: VectorStore> Retriever<V> {
    pub fn new(vector_store: V, embedding_service: EmbeddingService, top_k: usize) -> Self {
        Self {
            vector_store,
            embedding_service,
            top_k,
        }
    }

    #[instrument(skip(self))]
    pub async fn retrieve(&self, query: &str) -> Result<Vec<SearchResult>, AppError> {
        let query_embedding = self.embedding_service.embed_query(query).await?;
        self.vector_store.search(query_embedding, self.top_k).await
    }

    pub fn vector_store(&self) -> &V {
        &self.vector_store
    }

    pub fn embedding_service(&self) -> &EmbeddingService {
        &self.embedding_service
    }
}
