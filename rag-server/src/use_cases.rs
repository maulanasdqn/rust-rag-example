use async_trait::async_trait;
use rag_config::Settings;
use rag_database::{create_pool, PgVectorStore};
use rag_documents::{
    application::{DeleteDocument, ListDocuments, UploadDocument},
    infrastructure::persistence::{CompositeLoader, PostgresDocumentRepository},
    EmbeddingProvider as DocumentEmbeddingProvider,
};
use rag_errors::AppError;
use rag_query::{
    application::QueryDocuments,
    infrastructure::services::{EmbeddingService, LlmService},
};
use std::sync::Arc;

pub struct DocumentEmbedder {
    service: Arc<EmbeddingService>,
}

impl DocumentEmbedder {
    pub fn new(service: Arc<EmbeddingService>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl DocumentEmbeddingProvider for DocumentEmbedder {
    async fn embed_texts(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError> {
        self.service.embed_texts(texts).await
    }
}

pub type UploadDocumentUseCase =
    UploadDocument<CompositeLoader, PostgresDocumentRepository, DocumentEmbedder>;
pub type ListDocumentsUseCase = ListDocuments<PostgresDocumentRepository>;
pub type DeleteDocumentUseCase = DeleteDocument<PostgresDocumentRepository>;
pub type QueryDocumentsUseCase = QueryDocuments<PgVectorStore, EmbeddingService, LlmService>;

#[derive(Clone)]
pub struct AppState {
    pub upload_document: Arc<UploadDocumentUseCase>,
    pub list_documents: Arc<ListDocumentsUseCase>,
    pub delete_document: Arc<DeleteDocumentUseCase>,
    pub query_documents: Arc<QueryDocumentsUseCase>,
}

impl AppState {
    pub async fn new(settings: &Settings) -> Result<Self, Box<dyn std::error::Error>> {
        let pool = create_pool(&settings.database.url, settings.database.max_connections).await?;
        let vector_store = Arc::new(PgVectorStore::new(pool));

        let document_repository = Arc::new(PostgresDocumentRepository::new(vector_store.clone()));
        let document_loader = Arc::new(CompositeLoader::default_loaders());

        let embedding_service = Arc::new(EmbeddingService::new(
            &settings.openai.api_key,
            &settings.openai.api_base,
            &settings.openai.embedding_model,
        ));

        let document_embedder = Arc::new(DocumentEmbedder::new(embedding_service.clone()));

        let llm_service = Arc::new(LlmService::new(
            &settings.openai.api_key,
            &settings.openai.api_base,
            &settings.openai.chat_model,
            &settings.rag.system_prompt,
        ));

        let upload_document = Arc::new(UploadDocument::new(
            document_loader,
            document_repository.clone(),
            document_embedder,
            settings.rag.chunk_size,
            settings.rag.chunk_overlap,
        ));

        let list_documents = Arc::new(ListDocuments::new(document_repository.clone()));
        let delete_document = Arc::new(DeleteDocument::new(document_repository));

        let query_documents = Arc::new(QueryDocuments::new(
            vector_store,
            embedding_service,
            llm_service,
            settings.rag.top_k,
        ));

        Ok(Self {
            upload_document,
            list_documents,
            delete_document,
            query_documents,
        })
    }
}
