use rag_errors::AppError;
use std::sync::Arc;
use surrealdb::engine::local::{Db, RocksDb};
use surrealdb::Surreal;

pub type DbPool = Arc<Surreal<Db>>;

pub async fn create_pool(database_path: &str) -> Result<DbPool, AppError> {
    let db = Surreal::new::<RocksDb>(database_path)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    db.use_ns("rag")
        .use_db("rag_db")
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(Arc::new(db))
}

pub async fn run_migrations(pool: &DbPool) -> Result<(), AppError> {
    // Define document table schema (id is automatic record id)
    pool.query(
        r#"
        DEFINE TABLE IF NOT EXISTS document SCHEMAFULL;
        DEFINE FIELD IF NOT EXISTS source_file ON document TYPE string;
        DEFINE FIELD IF NOT EXISTS created_at ON document TYPE datetime DEFAULT time::now();
        "#,
    )
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // Define document_chunk table with embedding (id is automatic record id)
    pool.query(
        r#"
        DEFINE TABLE IF NOT EXISTS document_chunk SCHEMAFULL;
        DEFINE FIELD IF NOT EXISTS document_id ON document_chunk TYPE string;
        DEFINE FIELD IF NOT EXISTS content ON document_chunk TYPE string;
        DEFINE FIELD IF NOT EXISTS chunk_index ON document_chunk TYPE int;
        DEFINE FIELD IF NOT EXISTS metadata ON document_chunk FLEXIBLE TYPE object;
        DEFINE FIELD IF NOT EXISTS embedding ON document_chunk TYPE array<float>;
        DEFINE FIELD IF NOT EXISTS created_at ON document_chunk TYPE datetime DEFAULT time::now();

        DEFINE INDEX IF NOT EXISTS idx_chunk_document ON document_chunk FIELDS document_id;
        "#,
    )
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // Rebuild HNSW index to ensure all embeddings are indexed
    pool.query(
        r#"
        REMOVE INDEX IF EXISTS idx_chunk_embedding ON document_chunk;
        DEFINE INDEX idx_chunk_embedding ON document_chunk
            FIELDS embedding HNSW DIMENSION 1536 DIST COSINE TYPE F32;
        "#,
    )
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(())
}
