use crate::domain::DocumentRepository;
use rag_errors::AppError;
use std::sync::Arc;
use tracing::instrument;
use uuid::Uuid;

pub struct DeleteDocument<R: DocumentRepository> {
    repository: Arc<R>,
}

impl<R: DocumentRepository> DeleteDocument<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    #[instrument(skip(self))]
    pub async fn execute(&self, document_id: Uuid) -> Result<(), AppError> {
        self.repository.delete(document_id).await
    }
}
