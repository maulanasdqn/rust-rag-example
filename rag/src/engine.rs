use common::AppError;
use document::{CompositeLoader, DocumentChunker};
use rag_config::Settings;
use storage::{PgVectorStore, VectorStore};
use tracing::instrument;
use uuid::Uuid;

use crate::{EmbeddingService, ResponseGenerator, Retriever};

pub struct RagEngine {
    retriever: Retriever<PgVectorStore>,
    generator: ResponseGenerator,
    document_loader: CompositeLoader,
    chunker: DocumentChunker,
}

impl RagEngine {
    pub async fn new(settings: &Settings) -> Result<Self, AppError> {
        let vector_store =
            PgVectorStore::new(&settings.database.url, settings.database.max_connections).await?;

        vector_store.run_migrations().await?;

        let embedding_service =
            EmbeddingService::new(&settings.openai.api_key, &settings.openai.embedding_model);

        let retriever = Retriever::new(vector_store, embedding_service, settings.rag.top_k);

        let generator = ResponseGenerator::new(
            &settings.openai.api_key,
            &settings.openai.chat_model,
            &settings.rag.system_prompt,
        );

        let document_loader = CompositeLoader::default_loaders();
        let chunker = DocumentChunker::new(settings.rag.chunk_size, settings.rag.chunk_overlap);

        Ok(Self {
            retriever,
            generator,
            document_loader,
            chunker,
        })
    }

    #[instrument(skip(self, content), fields(filename = %filename))]
    pub async fn ingest_document(&self, content: &[u8], filename: &str) -> Result<Uuid, AppError> {
        let document = self.document_loader.load_bytes(content, filename)?;
        let document_id = Uuid::new_v4();
        let chunks = self.chunker.chunk(&document, document_id);

        if chunks.is_empty() {
            return Err(AppError::InvalidDocumentFormat(
                "Document produced no chunks".to_string(),
            ));
        }

        let texts: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
        let embeddings = self.retriever.embedding_service().embed_texts(texts).await?;

        self.retriever
            .vector_store()
            .store_chunks(chunks, embeddings)
            .await?;

        Ok(document_id)
    }

    #[instrument(skip(self))]
    pub async fn query(&self, question: &str) -> Result<QueryResponse, AppError> {
        let results = self.retriever.retrieve(question).await?;

        if results.is_empty() {
            return Ok(QueryResponse {
                answer: "I don't have enough context to answer this question.".to_string(),
                sources: Vec::new(),
            });
        }

        let answer = self.generator.generate(question, &results).await?;

        let sources = results
            .iter()
            .map(|r| Source {
                document_id: r.chunk.document_id,
                source_file: r.chunk.metadata.source_file.clone(),
                chunk_content: r.chunk.content.clone(),
                score: r.score,
            })
            .collect();

        Ok(QueryResponse { answer, sources })
    }

    pub async fn delete_document(&self, document_id: Uuid) -> Result<(), AppError> {
        self.retriever
            .vector_store()
            .delete_document(document_id)
            .await
    }

    pub async fn list_documents(&self) -> Result<Vec<Uuid>, AppError> {
        self.retriever.vector_store().list_documents().await
    }
}

#[derive(Debug, Clone)]
pub struct QueryResponse {
    pub answer: String,
    pub sources: Vec<Source>,
}

#[derive(Debug, Clone)]
pub struct Source {
    pub document_id: Uuid,
    pub source_file: String,
    pub chunk_content: String,
    pub score: f32,
}
