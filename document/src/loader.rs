use common::types::DocumentType;
use common::AppError;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct LoadedDocument {
    pub content: String,
    pub source_file: String,
    pub document_type: DocumentType,
    pub page_count: Option<usize>,
}

pub trait DocumentLoader: Send + Sync {
    fn supports(&self, extension: &str) -> bool;
    fn load(&self, path: &Path) -> Result<LoadedDocument, AppError>;
    fn load_bytes(&self, bytes: &[u8], filename: &str) -> Result<LoadedDocument, AppError>;
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
        use crate::{PdfLoader, TextLoader};
        Self::new().with_loader(PdfLoader).with_loader(TextLoader)
    }

    pub fn load(&self, path: &Path) -> Result<LoadedDocument, AppError> {
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

    pub fn load_bytes(&self, bytes: &[u8], filename: &str) -> Result<LoadedDocument, AppError> {
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

impl Default for CompositeLoader {
    fn default() -> Self {
        Self::default_loaders()
    }
}
