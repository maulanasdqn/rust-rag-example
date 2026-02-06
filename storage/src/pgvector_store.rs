use async_trait::async_trait;
use common::types::{ChunkMetadata, DocumentChunk};
use common::AppError;
use pgvector::Vector;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use tracing::instrument;
use uuid::Uuid;

use crate::models::DocumentInfo;
use crate::vector_store::{SearchResult, VectorStore};

pub struct PgVectorStore {
    pool: PgPool,
}

impl PgVectorStore {
    pub async fn new(database_url: &str, max_connections: u32) -> Result<Self, AppError> {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .connect(database_url)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(Self { pool })
    }

    pub async fn run_migrations(&self) -> Result<(), AppError> {
        sqlx::query(
            r#"
            CREATE EXTENSION IF NOT EXISTS vector;

            CREATE TABLE IF NOT EXISTS documents (
                id UUID PRIMARY KEY,
                source_file TEXT NOT NULL,
                created_at TIMESTAMPTZ DEFAULT NOW()
            );

            CREATE TABLE IF NOT EXISTS document_chunks (
                id UUID PRIMARY KEY,
                document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
                content TEXT NOT NULL,
                chunk_index INTEGER NOT NULL,
                metadata JSONB NOT NULL,
                embedding vector(1536),
                created_at TIMESTAMPTZ DEFAULT NOW()
            );

            CREATE INDEX IF NOT EXISTS idx_chunks_document_id ON document_chunks(document_id);
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM document_chunks")
            .fetch_one(&self.pool)
            .await
            .unwrap_or((0,));

        if count.0 >= 100 {
            sqlx::query(
                r#"
                CREATE INDEX IF NOT EXISTS idx_chunks_embedding ON document_chunks
                    USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);
                "#,
            )
            .execute(&self.pool)
            .await
            .ok();
        }

        Ok(())
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
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
                1 - (embedding <=> $1) as score
            FROM document_chunks
            WHERE embedding IS NOT NULL
            ORDER BY embedding <=> $1
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
