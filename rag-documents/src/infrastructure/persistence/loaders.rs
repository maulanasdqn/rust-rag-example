use crate::domain::{Document, DocumentLoader};
use pdf_extract::extract_text;
use rag_errors::AppError;
use rag_types::DocumentType;
use std::fs;
use std::io::Write;
use std::path::Path;
use tracing::instrument;

pub struct PdfLoader;

impl DocumentLoader for PdfLoader {
    fn supports(&self, extension: &str) -> bool {
        extension.eq_ignore_ascii_case("pdf")
    }

    #[instrument(skip(self), fields(path = %path.display()))]
    fn load(&self, path: &Path) -> Result<Document, AppError> {
        let content = extract_text(path).map_err(|e| {
            AppError::InvalidDocumentFormat(format!("Failed to extract PDF text: {}", e))
        })?;

        Ok(Document {
            content,
            source_file: path.to_string_lossy().to_string(),
            document_type: DocumentType::Pdf,
            page_count: None,
        })
    }

    #[instrument(skip(self, bytes), fields(filename = %filename))]
    fn load_bytes(&self, bytes: &[u8], filename: &str) -> Result<Document, AppError> {
        let mut temp_file = tempfile::NamedTempFile::new()?;
        temp_file.write_all(bytes)?;

        let content = extract_text(temp_file.path()).map_err(|e| {
            AppError::InvalidDocumentFormat(format!("Failed to extract PDF text: {}", e))
        })?;

        Ok(Document {
            content,
            source_file: filename.to_string(),
            document_type: DocumentType::Pdf,
            page_count: None,
        })
    }
}

pub struct TextLoader;

impl DocumentLoader for TextLoader {
    fn supports(&self, extension: &str) -> bool {
        matches!(
            extension.to_lowercase().as_str(),
            "txt" | "md" | "markdown"
        )
    }

    #[instrument(skip(self), fields(path = %path.display()))]
    fn load(&self, path: &Path) -> Result<Document, AppError> {
        let content = fs::read_to_string(path)?;
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("txt");

        let document_type = if extension.eq_ignore_ascii_case("md")
            || extension.eq_ignore_ascii_case("markdown")
        {
            DocumentType::Markdown
        } else {
            DocumentType::Text
        };

        Ok(Document {
            content,
            source_file: path.to_string_lossy().to_string(),
            document_type,
            page_count: None,
        })
    }

    #[instrument(skip(self, bytes), fields(filename = %filename))]
    fn load_bytes(&self, bytes: &[u8], filename: &str) -> Result<Document, AppError> {
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

        Ok(Document {
            content,
            source_file: filename.to_string(),
            document_type,
            page_count: None,
        })
    }
}

pub struct CompositeLoader {
    loaders: Vec<Box<dyn DocumentLoader>>,
}

impl CompositeLoader {
    pub fn new() -> Self {
        Self {
            loaders: Vec::new(),
        }
    }

    pub fn with_loader(mut self, loader: impl DocumentLoader + 'static) -> Self {
        self.loaders.push(Box::new(loader));
        self
    }

    pub fn default_loaders() -> Self {
        Self::new().with_loader(PdfLoader).with_loader(TextLoader)
    }
}

impl Default for CompositeLoader {
    fn default() -> Self {
        Self::default_loaders()
    }
}

impl DocumentLoader for CompositeLoader {
    fn supports(&self, extension: &str) -> bool {
        self.loaders.iter().any(|l| l.supports(extension))
    }

    fn load(&self, path: &Path) -> Result<Document, AppError> {
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        for loader in &self.loaders {
            if loader.supports(extension) {
                return loader.load(path);
            }
        }

        Err(AppError::InvalidDocumentFormat(format!(
            "Unsupported file type: {}",
            extension
        )))
    }

    fn load_bytes(&self, bytes: &[u8], filename: &str) -> Result<Document, AppError> {
        let extension = Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        for loader in &self.loaders {
            if loader.supports(extension) {
                return loader.load_bytes(bytes, filename);
            }
        }

        Err(AppError::InvalidDocumentFormat(format!(
            "Unsupported file type: {}",
            extension
        )))
    }
}
