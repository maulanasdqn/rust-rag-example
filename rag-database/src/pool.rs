use rag_errors::AppError;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub type DbPool = PgPool;

pub async fn create_pool(database_url: &str, max_connections: u32) -> Result<DbPool, AppError> {
    PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(database_url)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))
}

pub async fn run_migrations(pool: &DbPool) -> Result<(), AppError> {
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
    .execute(pool)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM document_chunks")
        .fetch_one(pool)
        .await
        .unwrap_or((0,));

    if count.0 >= 100 {
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_chunks_embedding ON document_chunks
                USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);
            "#,
        )
        .execute(pool)
        .await
        .ok();
    }

    Ok(())
}
