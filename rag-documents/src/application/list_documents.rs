use crate::domain::DocumentRepository;
use rag_errors::AppError;
use std::sync::Arc;
use tracing::instrument;
use uuid::Uuid;

pub struct ListDocuments<R: DocumentRepository> {
    repository: Arc<R>,
}

impl<R: DocumentRepository> ListDocuments<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    #[instrument(skip(self))]
    pub async fn execute(&self) -> Result<Vec<Uuid>, AppError> {
        self.repository.list().await
    }
}
