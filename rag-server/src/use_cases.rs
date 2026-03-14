use rag_config::Settings;
use rag_database::{create_pool, run_migrations, SurrealVectorStore};
use rag_documents::{
    application::{DeleteDocument, ListDocuments, UploadDocument},
    infrastructure::persistence::{CompositeLoader, SurrealDocumentRepository},
};
use rag_inference::{OpenAIEmbedding, OpenAILlm};
use rag_memory::{SurrealConversationStore, traits::ConversationStore};
use rag_query::application::QueryDocuments;
use rag_tools::{CalculatorTool, DateTimeTool, ToolRegistry, WebSearchTool};
use std::sync::Arc;

use crate::security::SecurityState;

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
    pub conversation_store: Arc<dyn ConversationStore>,
    pub tool_registry: Arc<ToolRegistry>,
    pub security: SecurityState,
}

impl AppState {
    pub async fn new(settings: &Settings) -> Result<Self, Box<dyn std::error::Error>> {
        let pool = create_pool(&settings.database.path).await?;
        run_migrations(&pool).await?;
        let vector_store = Arc::new(SurrealVectorStore::new(pool.clone()));

        let document_repository = Arc::new(SurrealDocumentRepository::new(vector_store.clone()));
        let document_loader = Arc::new(CompositeLoader::default_loaders());

        let embedding_provider = Arc::new(OpenAIEmbedding::new(
            &settings.openai.api_key,
            &settings.openai.api_base,
            &settings.openai.embedding_model,
        ));

        // Create LLM provider with optional max_tokens for cost control
        let llm_provider = if settings.security.max_tokens > 0 {
            Arc::new(OpenAILlm::with_max_tokens(
                &settings.openai.api_key,
                &settings.openai.api_base,
                &settings.openai.chat_model,
                &settings.rag.system_prompt,
                settings.security.max_tokens,
            ))
        } else {
            Arc::new(OpenAILlm::new(
                &settings.openai.api_key,
                &settings.openai.api_base,
                &settings.openai.chat_model,
                &settings.rag.system_prompt,
            ))
        };

        let upload_document = Arc::new(UploadDocument::new(
            document_loader,
            document_repository.clone(),
            embedding_provider.clone(),
            settings.rag.chunk_size,
            settings.rag.chunk_overlap,
        ));

        let list_documents = Arc::new(ListDocuments::new(document_repository.clone()));
        let delete_document = Arc::new(DeleteDocument::new(document_repository));

        let query_documents = Arc::new(QueryDocuments::with_min_relevance(
            vector_store,
            embedding_provider,
            llm_provider,
            settings.rag.top_k,
            settings.rag.min_relevance_score,
        ));

        // Initialize conversation store
        let conversation_store = SurrealConversationStore::new(pool);
        conversation_store.init_schema().await?;
        let conversation_store: Arc<dyn ConversationStore> = Arc::new(conversation_store);

        // Initialize tool registry with built-in tools
        let mut tool_registry = ToolRegistry::new();
        tool_registry.register(CalculatorTool::new());
        tool_registry.register(DateTimeTool::new());
        tool_registry.register(WebSearchTool::new());
        let tool_registry = Arc::new(tool_registry);

        // Initialize security state
        let security = SecurityState::new(settings.security.clone());

        Ok(Self {
            upload_document,
            list_documents,
            delete_document,
            query_documents,
            conversation_store,
            tool_registry,
            security,
        })
    }
}
