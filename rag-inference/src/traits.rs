use async_trait::async_trait;
use futures::stream::BoxStream;
use rag_database::SearchResult;
use rag_errors::AppError;

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed_query(&self, text: &str) -> Result<Vec<f32>, AppError>;
    async fn embed_texts(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError>;
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn generate(&self, query: &str, context: &[SearchResult]) -> Result<String, AppError>;
    async fn generate_stream(
        &self,
        query: &str,
        context: &[SearchResult],
    ) -> Result<BoxStream<'static, Result<String, AppError>>, AppError>;
}
