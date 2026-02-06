use crate::loader::LoadedDocument;
use common::types::{ChunkMetadata, DocumentChunk};
use uuid::Uuid;

pub struct DocumentChunker {
    chunk_size: usize,
    chunk_overlap: usize,
}

impl DocumentChunker {
    pub fn new(chunk_size: usize, chunk_overlap: usize) -> Self {
        Self {
            chunk_size,
            chunk_overlap,
        }
    }

    pub fn chunk(&self, document: &LoadedDocument, document_id: Uuid) -> Vec<DocumentChunk> {
        let text = &document.content;
        let mut chunks = Vec::new();
        let mut start = 0;
        let mut chunk_index = 0;

        while start < text.len() {
            let end = (start + self.chunk_size).min(text.len());

            let actual_end = if end < text.len() {
                self.find_break_point(text, start, end)
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
                        source_file: document.source_file.clone(),
                        file_type: document.document_type,
                        page_number: None,
                    },
                });
                chunk_index += 1;
            }

            start = if actual_end >= text.len() {
                text.len()
            } else {
                (actual_end - self.chunk_overlap).max(start + 1)
            };
        }

        chunks
    }

    fn find_break_point(&self, text: &str, start: usize, end: usize) -> usize {
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
}
