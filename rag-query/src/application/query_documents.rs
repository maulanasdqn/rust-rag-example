use crate::domain::{QueryResult, Source};
use futures::stream::BoxStream;
use rag_database::{DocumentInfo, VectorStore};
use rag_errors::AppError;
use rag_inference::{EmbeddingProvider, LlmProvider};
use std::sync::Arc;
use tracing::instrument;
use uuid::Uuid;

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

    pub async fn get_document_info(&self, document_id: Uuid) -> Result<Option<DocumentInfo>, AppError> {
        self.vector_store.get_document_info(document_id).await
    }

    #[instrument(skip(self))]
    pub async fn execute_stream(
        &self,
        question: &str,
    ) -> Result<(BoxStream<'static, Result<String, AppError>>, Vec<Source>), AppError> {
        let query_embedding = self.embedding_provider.embed_query(question).await?;
        let results = self.vector_store.search(query_embedding, self.top_k).await?;

        let sources: Vec<Source> = results
            .iter()
            .map(|r| Source {
                document_id: r.chunk.document_id,
                source_file: r.chunk.metadata.source_file.clone(),
                chunk_content: r.chunk.content.clone(),
                score: r.score,
            })
            .collect();

        if results.is_empty() {
            let empty_stream: BoxStream<'static, Result<String, AppError>> =
                Box::pin(futures::stream::once(async {
                    Ok("I don't have enough context to answer this question.".to_string())
                }));
            return Ok((empty_stream, sources));
        }

        let stream = self.llm_provider.generate_stream(question, &results).await?;
        Ok((stream, sources))
    }
}
