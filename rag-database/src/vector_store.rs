use async_trait::async_trait;
use chrono::{DateTime, Utc};
use pgvector::Vector;
use rag_errors::AppError;
use rag_types::{ChunkMetadata, DocumentChunk};
use sqlx::{PgPool, Row};
use tracing::instrument;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct DocumentInfo {
    pub id: Uuid,
    pub source_file: String,
    pub chunk_count: usize,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub chunk: DocumentChunk,
    pub score: f32,
}

#[async_trait]
pub trait VectorStore: Send + Sync {
    async fn store_chunks(
        &self,
        chunks: Vec<DocumentChunk>,
        embeddings: Vec<Vec<f32>>,
    ) -> Result<(), AppError>;

    async fn search(
        &self,
        query_embedding: Vec<f32>,
        top_k: usize,
    ) -> Result<Vec<SearchResult>, AppError>;

    async fn delete_document(&self, document_id: Uuid) -> Result<(), AppError>;

    async fn list_documents(&self) -> Result<Vec<Uuid>, AppError>;

    async fn get_document_info(&self, document_id: Uuid) -> Result<Option<DocumentInfo>, AppError>;
}

pub struct PgVectorStore {
    pool: PgPool,
}

impl PgVectorStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl VectorStore for PgVectorStore {
    #[instrument(skip(self, chunks, embeddings))]
    async fn store_chunks(
        &self,
        chunks: Vec<DocumentChunk>,
        embeddings: Vec<Vec<f32>>,
    ) -> Result<(), AppError> {
        if chunks.is_empty() {
            return Ok(());
        }

        let document_id = chunks[0].document_id;
        let source_file = &chunks[0].metadata.source_file;

        sqlx::query(
            "INSERT INTO documents (id, source_file) VALUES ($1, $2) ON CONFLICT (id) DO NOTHING",
        )
        .bind(document_id)
        .bind(source_file)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        for (chunk, embedding) in chunks.into_iter().zip(embeddings.into_iter()) {
            let metadata_json =
                serde_json::to_value(&chunk.metadata).map_err(|e| AppError::Internal(e.to_string()))?;

            let vector = Vector::from(embedding);

            sqlx::query(
                r#"
                INSERT INTO document_chunks (id, document_id, content, chunk_index, metadata, embedding)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
            )
            .bind(chunk.id)
            .bind(chunk.document_id)
            .bind(&chunk.content)
            .bind(chunk.chunk_index as i32)
            .bind(&metadata_json)
            .bind(&vector)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        }

        Ok(())
    }

    #[instrument(skip(self, query_embedding))]
    async fn search(
        &self,
        query_embedding: Vec<f32>,
        top_k: usize,
    ) -> Result<Vec<SearchResult>, AppError> {
        let vector = Vector::from(query_embedding);

        let rows = sqlx::query(
            r#"
            SELECT
                id, document_id, content, chunk_index, metadata,
                (1 - (embedding <=> $1::vector))::real as score
            FROM document_chunks
            WHERE embedding IS NOT NULL
            ORDER BY embedding <=> $1::vector
            LIMIT $2
            "#,
        )
        .bind(&vector)
        .bind(top_k as i32)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            let metadata_json: serde_json::Value = row.get("metadata");
            let metadata: ChunkMetadata =
                serde_json::from_value(metadata_json).map_err(|e| AppError::Internal(e.to_string()))?;

            results.push(SearchResult {
                chunk: DocumentChunk {
                    id: row.get("id"),
                    document_id: row.get("document_id"),
                    content: row.get("content"),
                    chunk_index: row.get::<i32, _>("chunk_index") as usize,
                    metadata,
                },
                score: row.get("score"),
            });
        }

        Ok(results)
    }

    #[instrument(skip(self))]
    async fn delete_document(&self, document_id: Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM documents WHERE id = $1")
            .bind(document_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn list_documents(&self) -> Result<Vec<Uuid>, AppError> {
        let rows = sqlx::query("SELECT id FROM documents ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(rows.iter().map(|r| r.get("id")).collect())
    }

    #[instrument(skip(self))]
    async fn get_document_info(&self, document_id: Uuid) -> Result<Option<DocumentInfo>, AppError> {
        let row = sqlx::query(
            r#"
            SELECT d.id, d.source_file, d.created_at, COUNT(c.id) as chunk_count
            FROM documents d
            LEFT JOIN document_chunks c ON d.id = c.document_id
            WHERE d.id = $1
            GROUP BY d.id
            "#,
        )
        .bind(document_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(row.map(|r| DocumentInfo {
            id: r.get("id"),
            source_file: r.get("source_file"),
            chunk_count: r.get::<i64, _>("chunk_count") as usize,
            created_at: r.get("created_at"),
        }))
    }
}
