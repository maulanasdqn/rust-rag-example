use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rag_errors::AppError;
use rag_types::{ChunkMetadata, DocumentChunk};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::RwLock;
use tracing::instrument;
use uuid::Uuid;

use crate::DbPool;

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

// SurrealDB query result types

#[derive(Debug, Deserialize)]
struct ChunkQueryResult {
    id: String,
    document_id: String,
    content: String,
    chunk_index: i32,
    metadata: serde_json::Value,
    #[serde(default)]
    score: f32,
}

#[derive(Debug, Deserialize)]
struct DocumentQueryResult {
    id: String,
    source_file: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct IdResult {
    id: String,
}

#[derive(Debug, Deserialize)]
struct CountResult {
    count: i64,
}

// In-memory vector cache for fast similarity search
#[derive(Debug, Clone)]
struct CachedChunk {
    chunk: DocumentChunk,
    embedding: Vec<f32>,
}

struct VectorCache {
    chunks: HashMap<String, CachedChunk>,
    initialized: bool,
}

impl VectorCache {
    fn new() -> Self {
        Self {
            chunks: HashMap::new(),
            initialized: false,
        }
    }
}

// Fast cosine similarity using SIMD-friendly operations
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

// SurrealVectorStore implementation

pub struct SurrealVectorStore {
    pool: DbPool,
    cache: RwLock<VectorCache>,
}

impl SurrealVectorStore {
    pub fn new(pool: DbPool) -> Self {
        Self {
            pool,
            cache: RwLock::new(VectorCache::new()),
        }
    }

    async fn load_cache(&self) -> Result<(), AppError> {
        let start = std::time::Instant::now();

        // Load all chunks with embeddings
        let mut response = self
            .pool
            .query(
                r#"
                SELECT
                    meta::id(id) as id,
                    document_id,
                    content,
                    chunk_index,
                    metadata,
                    embedding
                FROM document_chunk
                WHERE embedding IS NOT NONE
                "#,
            )
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        #[derive(Debug, Deserialize)]
        struct CacheQueryResult {
            id: String,
            document_id: String,
            content: String,
            chunk_index: i32,
            metadata: serde_json::Value,
            embedding: Vec<f32>,
        }

        let rows: Vec<CacheQueryResult> = response
            .take(0)
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let mut cache = self.cache.write().unwrap();
        cache.chunks.clear();

        for row in rows {
            let metadata: ChunkMetadata = serde_json::from_value(row.metadata)
                .map_err(|e| AppError::Internal(e.to_string()))?;

            let id = Uuid::parse_str(&row.id).map_err(|e| AppError::Internal(e.to_string()))?;
            let document_id =
                Uuid::parse_str(&row.document_id).map_err(|e| AppError::Internal(e.to_string()))?;

            cache.chunks.insert(
                row.id.clone(),
                CachedChunk {
                    chunk: DocumentChunk {
                        id,
                        document_id,
                        content: row.content,
                        chunk_index: row.chunk_index as usize,
                        metadata,
                    },
                    embedding: row.embedding,
                },
            );
        }

        cache.initialized = true;
        tracing::info!("Loaded {} chunks into cache in {:?}", cache.chunks.len(), start.elapsed());

        Ok(())
    }

    async fn ensure_cache_loaded(&self) -> Result<(), AppError> {
        {
            let cache = self.cache.read().unwrap();
            if cache.initialized {
                return Ok(());
            }
        }
        self.load_cache().await
    }

    fn invalidate_cache(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.initialized = false;
    }
}

#[async_trait]
impl VectorStore for SurrealVectorStore {
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
        let source_file = chunks[0].metadata.source_file.clone();

        // Upsert document - use UPSERT with record id
        let doc_id_str = document_id.to_string();
        self.pool
            .query("UPSERT type::thing('document', $id) SET source_file = $source_file")
            .bind(("id", doc_id_str))
            .bind(("source_file", source_file))
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // Insert chunks with embeddings
        for (chunk, embedding) in chunks.into_iter().zip(embeddings.into_iter()) {
            let metadata_json = serde_json::to_value(&chunk.metadata)
                .map_err(|e| AppError::Internal(e.to_string()))?;

            self.pool
                .query(
                    r#"
                    CREATE type::thing('document_chunk', $id) SET
                        document_id = $document_id,
                        content = $content,
                        chunk_index = $chunk_index,
                        metadata = $metadata,
                        embedding = $embedding
                    "#,
                )
                .bind(("id", chunk.id.to_string()))
                .bind(("document_id", chunk.document_id.to_string()))
                .bind(("content", chunk.content))
                .bind(("chunk_index", chunk.chunk_index as i32))
                .bind(("metadata", metadata_json))
                .bind(("embedding", embedding))
                .await
                .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        }

        // Invalidate cache so it reloads on next search
        self.invalidate_cache();

        Ok(())
    }

    #[instrument(skip(self, query_embedding))]
    async fn search(
        &self,
        query_embedding: Vec<f32>,
        top_k: usize,
    ) -> Result<Vec<SearchResult>, AppError> {
        let start = std::time::Instant::now();

        // Ensure embeddings are loaded into memory
        self.ensure_cache_loaded().await?;

        // Fast in-memory similarity search
        let cache = self.cache.read().unwrap();

        // Calculate similarities for all chunks
        let mut scored: Vec<(f32, &CachedChunk)> = cache
            .chunks
            .values()
            .map(|cached| {
                let score = cosine_similarity(&query_embedding, &cached.embedding);
                (score, cached)
            })
            .collect();

        // Sort by score descending and take top_k
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let results: Vec<SearchResult> = scored
            .into_iter()
            .take(top_k)
            .map(|(score, cached)| SearchResult {
                chunk: cached.chunk.clone(),
                score,
            })
            .collect();

        tracing::debug!("In-memory search took {:?} ({} chunks)", start.elapsed(), cache.chunks.len());

        Ok(results)
    }

    #[instrument(skip(self))]
    async fn delete_document(&self, document_id: Uuid) -> Result<(), AppError> {
        let doc_id = document_id.to_string();

        // Delete chunks first (no cascade in SurrealDB by default)
        self.pool
            .query("DELETE document_chunk WHERE document_id = $doc_id")
            .bind(("doc_id", doc_id.clone()))
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // Delete document using record id
        self.pool
            .query("DELETE type::thing('document', $doc_id)")
            .bind(("doc_id", doc_id))
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // Invalidate cache
        self.invalidate_cache();

        Ok(())
    }

    #[instrument(skip(self))]
    async fn list_documents(&self) -> Result<Vec<Uuid>, AppError> {
        let mut response = self
            .pool
            .query("SELECT meta::id(id) as id, created_at FROM document ORDER BY created_at DESC")
            .await
            .map_err(|e| {
                tracing::error!("SurrealDB query error: {}", e);
                AppError::DatabaseError(e.to_string())
            })?;

        let rows: Vec<IdResult> = response
            .take(0)
            .map_err(|e| {
                tracing::error!("SurrealDB take error: {}", e);
                AppError::DatabaseError(e.to_string())
            })?;

        rows.iter()
            .map(|r| Uuid::parse_str(&r.id).map_err(|e| AppError::Internal(e.to_string())))
            .collect()
    }

    #[instrument(skip(self))]
    async fn get_document_info(&self, document_id: Uuid) -> Result<Option<DocumentInfo>, AppError> {
        let doc_id = document_id.to_string();

        // Get document info
        let mut response = self
            .pool
            .query("SELECT meta::id(id) as id, source_file, created_at FROM document WHERE id = type::thing('document', $doc_id)")
            .bind(("doc_id", doc_id.clone()))
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let docs: Vec<DocumentQueryResult> = response
            .take(0)
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        if let Some(doc) = docs.into_iter().next() {
            // Get chunk count
            let mut count_response = self
                .pool
                .query("SELECT count() AS count FROM document_chunk WHERE document_id = $doc_id GROUP ALL")
                .bind(("doc_id", doc_id))
                .await
                .map_err(|e| AppError::DatabaseError(e.to_string()))?;

            let counts: Vec<CountResult> = count_response
                .take(0)
                .map_err(|e| AppError::DatabaseError(e.to_string()))?;

            let chunk_count = counts.first().map(|c| c.count as usize).unwrap_or(0);

            let id =
                Uuid::parse_str(&doc.id).map_err(|e| AppError::Internal(e.to_string()))?;

            Ok(Some(DocumentInfo {
                id,
                source_file: doc.source_file,
                chunk_count,
                created_at: doc.created_at,
            }))
        } else {
            Ok(None)
        }
    }
}
