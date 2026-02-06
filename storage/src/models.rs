use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct DocumentInfo {
    pub id: Uuid,
    pub source_file: String,
    pub chunk_count: usize,
    pub created_at: DateTime<Utc>,
}
