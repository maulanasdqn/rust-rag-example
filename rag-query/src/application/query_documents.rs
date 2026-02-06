use crate::domain::{QueryResult, Source};
use async_trait::async_trait;
use rag_database::{SearchResult, VectorStore};
use rag_errors::AppError;
use std::sync::Arc;
use tracing::instrument;

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed_query(&self, text: &str) -> Result<Vec<f32>, AppError>;
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn generate(&self, query: &str, context: &[SearchResult]) -> Result<String, AppError>;
}

pub struct QueryDocuments<V: VectorStore, E: EmbeddingProvider, L: LlmProvider> {
    vector_store: Arc<V>,
    embedding_provider: Arc<E>,
    llm_provider: Arc<L>,
    top_k: usize,
}

impl<V: VectorStore, E: EmbeddingProvider, L: LlmProvider> QueryDocuments<V, E, L> {
    pub fn new(
        vector_store: Arc<V>,
        embedding_provider: Arc<E>,
        llm_provider: Arc<L>,
        top_k: usize,
    ) -> Self {
        Self {
            vector_store,
            embedding_provider,
            llm_provider,
            top_k,
        }
    }

    #[instrument(skip(self))]
    pub async fn execute(&self, question: &str) -> Result<QueryResult, AppError> {
        let query_embedding = self.embedding_provider.embed_query(question).await?;
        let results = self.vector_store.search(query_embedding, self.top_k).await?;

        if results.is_empty() {
            return Ok(QueryResult {
                answer: "I don't have enough context to answer this question.".to_string(),
                sources: Vec::new(),
            });
        }

        let answer = self.llm_provider.generate(question, &results).await?;

        let sources = results
            .iter()
            .map(|r| Source {
                document_id: r.chunk.document_id,
                source_file: r.chunk.metadata.source_file.clone(),
                chunk_content: r.chunk.content.clone(),
                score: r.score,
            })
            .collect();

        Ok(QueryResult { answer, sources })
    }
}
