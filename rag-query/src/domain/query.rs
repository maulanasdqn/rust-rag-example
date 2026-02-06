use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct QueryResult {
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
