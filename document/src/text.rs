use crate::loader::{DocumentLoader, LoadedDocument};
use common::types::DocumentType;
use common::AppError;
use std::fs;
use std::path::Path;
use tracing::instrument;

pub struct TextLoader;

impl DocumentLoader for TextLoader {
    fn supports(&self, extension: &str) -> bool {
        matches!(
            extension.to_lowercase().as_str(),
            "txt" | "md" | "markdown"
        )
    }

    #[instrument(skip(self), fields(path = %path.display()))]
    fn load(&self, path: &Path) -> Result<LoadedDocument, AppError> {
        let content = fs::read_to_string(path)?;
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("txt");

        let document_type = if extension.eq_ignore_ascii_case("md")
            || extension.eq_ignore_ascii_case("markdown")
        {
            DocumentType::Markdown
        } else {
            DocumentType::Text
        };

        Ok(LoadedDocument {
            content,
            source_file: path.to_string_lossy().to_string(),
            document_type,
            page_count: None,
        })
    }

    #[instrument(skip(self, bytes), fields(filename = %filename))]
    fn load_bytes(&self, bytes: &[u8], filename: &str) -> Result<LoadedDocument, AppError> {
        let content = String::from_utf8(bytes.to_vec()).map_err(|e| {
            AppError::InvalidDocumentFormat(format!("Invalid UTF-8 in text file: {}", e))
        })?;

        let extension = Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("txt");

        let document_type = if extension.eq_ignore_ascii_case("md")
            || extension.eq_ignore_ascii_case("markdown")
        {
            DocumentType::Markdown
        } else {
            DocumentType::Text
        };

        Ok(LoadedDocument {
            content,
            source_file: filename.to_string(),
            document_type,
            page_count: None,
        })
    }
}
