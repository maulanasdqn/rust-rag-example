use async_openai::{
    config::OpenAIConfig,
    types::{CreateEmbeddingRequestArgs, EmbeddingInput},
    Client,
};
use common::AppError;
use tracing::instrument;

pub struct EmbeddingService {
    client: Client<OpenAIConfig>,
    model: String,
}

impl EmbeddingService {
    pub fn new(api_key: &str, model: &str) -> Self {
        let config = OpenAIConfig::new().with_api_key(api_key);
        let client = Client::with_config(config);
        Self {
            client,
            model: model.to_string(),
        }
    }

    #[instrument(skip(self, texts), fields(text_count = texts.len()))]
    pub async fn embed_texts(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let request = CreateEmbeddingRequestArgs::default()
            .model(&self.model)
            .input(EmbeddingInput::StringArray(texts))
            .build()
            .map_err(|e| AppError::EmbeddingError(e.to_string()))?;

        let response = self
            .client
            .embeddings()
            .create(request)
            .await
            .map_err(|e| AppError::OpenAIError(e.to_string()))?;

        let embeddings: Vec<Vec<f32>> = response.data.into_iter().map(|e| e.embedding).collect();

        Ok(embeddings)
    }

    #[instrument(skip(self, text))]
    pub async fn embed_query(&self, text: &str) -> Result<Vec<f32>, AppError> {
        let embeddings = self.embed_texts(vec![text.to_string()]).await?;
        embeddings
            .into_iter()
            .next()
            .ok_or_else(|| AppError::EmbeddingError("No embedding returned".to_string()))
    }
}
