use rag_errors::AppError;
use rag_types::{ChunkMetadata, DocumentChunk, DocumentType};
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Document {
    pub content: String,
    pub source_file: String,
    pub document_type: DocumentType,
    pub page_count: Option<usize>,
}

impl Document {
    pub fn chunk(&self, document_id: Uuid, chunk_size: usize, chunk_overlap: usize) -> Vec<DocumentChunk> {
        let text = &self.content;
        let mut chunks = Vec::new();
        let mut start = 0;
        let mut chunk_index = 0;

        while start < text.len() {
            let end = (start + chunk_size).min(text.len());

            let actual_end = if end < text.len() {
                find_break_point(text, start, end)
            } else {
                end
            };

            let content = text[start..actual_end].trim().to_string();

            if !content.is_empty() {
                chunks.push(DocumentChunk {
                    id: Uuid::new_v4(),
                    document_id,
                    content,
                    chunk_index,
                    metadata: ChunkMetadata {
                        source_file: self.source_file.clone(),
                        file_type: self.document_type,
                        page_number: None,
                    },
                });
                chunk_index += 1;
            }

            start = if actual_end >= text.len() {
                text.len()
            } else {
                (actual_end - chunk_overlap).max(start + 1)
            };
        }

        chunks
    }
}

fn find_break_point(text: &str, start: usize, end: usize) -> usize {
    if let Some(pos) = text[start..end].rfind("\n\n") {
        return start + pos + 2;
    }

    for pattern in &[". ", "! ", "? ", ".\n", "!\n", "?\n"] {
        if let Some(pos) = text[start..end].rfind(pattern) {
            return start + pos + pattern.len();
        }
    }

    if let Some(pos) = text[start..end].rfind(' ') {
        return start + pos + 1;
    }

    end
}

pub trait DocumentLoader: Send + Sync {
    fn supports(&self, extension: &str) -> bool;
    fn load(&self, path: &Path) -> Result<Document, AppError>;
    fn load_bytes(&self, bytes: &[u8], filename: &str) -> Result<Document, AppError>;
}
