use crate::loader::{DocumentLoader, LoadedDocument};
use common::types::DocumentType;
use common::AppError;
use pdf_extract::extract_text;
use std::io::Write;
use std::path::Path;
use tracing::instrument;

pub struct PdfLoader;

impl DocumentLoader for PdfLoader {
    fn supports(&self, extension: &str) -> bool {
        extension.eq_ignore_ascii_case("pdf")
    }

    #[instrument(skip(self), fields(path = %path.display()))]
    fn load(&self, path: &Path) -> Result<LoadedDocument, AppError> {
        let content = extract_text(path).map_err(|e| {
            AppError::InvalidDocumentFormat(format!("Failed to extract PDF text: {}", e))
        })?;

        Ok(LoadedDocument {
            content,
            source_file: path.to_string_lossy().to_string(),
            document_type: DocumentType::Pdf,
            page_count: None,
        })
    }

    #[instrument(skip(self, bytes), fields(filename = %filename))]
    fn load_bytes(&self, bytes: &[u8], filename: &str) -> Result<LoadedDocument, AppError> {
        let mut temp_file = tempfile::NamedTempFile::new()?;
        temp_file.write_all(bytes)?;

        let content = extract_text(temp_file.path()).map_err(|e| {
            AppError::InvalidDocumentFormat(format!("Failed to extract PDF text: {}", e))
        })?;

        Ok(LoadedDocument {
            content,
            source_file: filename.to_string(),
            document_type: DocumentType::Pdf,
            page_count: None,
        })
    }
}
