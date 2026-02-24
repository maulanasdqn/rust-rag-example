use crate::domain::{QueryResult, Source};
use futures::stream::BoxStream;
use rag_database::{DocumentInfo, VectorStore};
use rag_errors::AppError;
use rag_inference::{ChatMessage, EmbeddingProvider, LlmProvider};
use std::sync::Arc;
use tracing::instrument;
use uuid::Uuid;

/// Out-of-context response message
const OUT_OF_CONTEXT_MSG: &str = "I can only answer questions about the uploaded documents. Your question doesn't seem related to the available content.";

pub struct QueryDocuments<V: VectorStore, E: EmbeddingProvider, L: LlmProvider> {
    vector_store: Arc<V>,
    embedding_provider: Arc<E>,
    llm_provider: Arc<L>,
    top_k: usize,
    min_relevance_score: f32,
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
            min_relevance_score: 0.35, // Default threshold
        }
    }

    pub fn with_min_relevance(
        vector_store: Arc<V>,
        embedding_provider: Arc<E>,
        llm_provider: Arc<L>,
        top_k: usize,
        min_relevance_score: f32,
    ) -> Self {
        Self {
            vector_store,
            embedding_provider,
            llm_provider,
            top_k,
            min_relevance_score,
        }
    }

    /// Check if the results are relevant enough to answer the question
    fn is_relevant(&self, results: &[rag_database::SearchResult]) -> bool {
        if results.is_empty() {
            return false;
        }
        // Check if the best result meets the minimum relevance threshold
        let max_score = results.iter().map(|r| r.score).fold(0.0_f32, f32::max);
        tracing::debug!(max_score = max_score, threshold = self.min_relevance_score, "Relevance check");
        max_score >= self.min_relevance_score
    }

    #[instrument(skip(self))]
    pub async fn execute(&self, question: &str) -> Result<QueryResult, AppError> {
        let query_embedding = self.embedding_provider.embed_query(question).await?;
        let results = self.vector_store.search(query_embedding, self.top_k).await?;

        // Check if results are relevant enough
        if !self.is_relevant(&results) {
            tracing::info!(question = question, "Question rejected as out-of-context");
            return Ok(QueryResult {
                answer: OUT_OF_CONTEXT_MSG.to_string(),
                sources: results.iter().map(|r| Source {
                    document_id: r.chunk.document_id,
                    source_file: r.chunk.metadata.source_file.clone(),
                    chunk_content: r.chunk.content.clone(),
                    score: r.score,
                }).collect(),
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

        // Check if results are relevant enough
        if !self.is_relevant(&results) {
            tracing::info!(question = question, "Question rejected as out-of-context (stream)");
            let out_of_context_stream: BoxStream<'static, Result<String, AppError>> =
                Box::pin(futures::stream::once(async {
                    Ok(OUT_OF_CONTEXT_MSG.to_string())
                }));
            return Ok((out_of_context_stream, sources));
        }

        let stream = self.llm_provider.generate_stream(question, &results).await?;
        Ok((stream, sources))
    }

    /// Execute a query with conversation history (non-streaming)
    #[instrument(skip(self, history))]
    pub async fn execute_with_history(
        &self,
        question: &str,
        history: Vec<ChatMessage>,
    ) -> Result<QueryResult, AppError> {
        use futures::StreamExt;

        let (mut stream, sources) = self
            .execute_stream_with_history(question, history)
            .await?;

        // Collect all chunks into a single response
        let mut answer = String::new();
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => answer.push_str(&chunk),
                Err(e) => return Err(e),
            }
        }

        Ok(QueryResult { answer, sources })
    }

    #[instrument(skip(self, history))]
    pub async fn execute_stream_with_history(
        &self,
        question: &str,
        history: Vec<ChatMessage>,
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

        // Check if results are relevant enough
        if !self.is_relevant(&results) {
            tracing::info!(question = question, "Question rejected as out-of-context (stream with history)");
            let out_of_context_stream: BoxStream<'static, Result<String, AppError>> =
                Box::pin(futures::stream::once(async {
                    Ok(OUT_OF_CONTEXT_MSG.to_string())
                }));
            return Ok((out_of_context_stream, sources));
        }

        let stream = self
            .llm_provider
            .generate_stream_with_history(question, &results, history)
            .await?;
        Ok((stream, sources))
    }
}
