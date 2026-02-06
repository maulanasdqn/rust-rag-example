use axum::Json;
use rag_errors::AppError;
use rag_types::{QueryResponse, SourceInfo};
use uuid::Uuid;

use super::dto::QueryRequest;

pub async fn query_handler(
    Json(_request): Json<QueryRequest>,
) -> Result<Json<QueryResponse>, AppError> {
    Ok(Json(QueryResponse {
        answer: "This is a placeholder response.".to_string(),
        sources: vec![SourceInfo {
            document_id: Uuid::new_v4(),
            source_file: "example.pdf".to_string(),
            excerpt: "Example excerpt...".to_string(),
            relevance_score: 0.95,
        }],
    }))
}
