use rag_config::Settings;
use rag_database::{create_pool, run_migrations, SurrealVectorStore};
use rag_documents::{
    application::{DeleteDocument, ListDocuments, UploadDocument},
    infrastructure::persistence::{CompositeLoader, SurrealDocumentRepository},
};
use rag_inference::{OpenAIEmbedding, OpenAILlm};
use rag_query::application::QueryDocuments;
use std::sync::Arc;

pub type UploadDocumentUseCase =
    UploadDocument<CompositeLoader, SurrealDocumentRepository, OpenAIEmbedding>;
pub type ListDocumentsUseCase = ListDocuments<SurrealDocumentRepository>;
pub type DeleteDocumentUseCase = DeleteDocument<SurrealDocumentRepository>;
pub type QueryDocumentsUseCase = QueryDocuments<SurrealVectorStore, OpenAIEmbedding, OpenAILlm>;

#[derive(Clone)]
pub struct AppState {
    pub upload_document: Arc<UploadDocumentUseCase>,
    pub list_documents: Arc<ListDocumentsUseCase>,
    pub delete_document: Arc<DeleteDocumentUseCase>,
    pub query_documents: Arc<QueryDocumentsUseCase>,
}

impl AppState {
    pub async fn new(settings: &Settings) -> Result<Self, Box<dyn std::error::Error>> {
        let pool = create_pool(&settings.database.path).await?;
        run_migrations(&pool).await?;
        let vector_store = Arc::new(SurrealVectorStore::new(pool));

        let document_repository = Arc::new(SurrealDocumentRepository::new(vector_store.clone()));
        let document_loader = Arc::new(CompositeLoader::default_loaders());

        let embedding_provider = Arc::new(OpenAIEmbedding::new(
            &settings.openai.api_key,
            &settings.openai.api_base,
            &settings.openai.embedding_model,
        ));

        let llm_provider = Arc::new(OpenAILlm::new(
            &settings.openai.api_key,
            &settings.openai.api_base,
            &settings.openai.chat_model,
            &settings.rag.system_prompt,
        ));

        let upload_document = Arc::new(UploadDocument::new(
            document_loader,
            document_repository.clone(),
            embedding_provider.clone(),
            settings.rag.chunk_size,
            settings.rag.chunk_overlap,
        ));

        let list_documents = Arc::new(ListDocuments::new(document_repository.clone()));
        let delete_document = Arc::new(DeleteDocument::new(document_repository));

        let query_documents = Arc::new(QueryDocuments::new(
            vector_store,
            embedding_provider,
            llm_provider,
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
