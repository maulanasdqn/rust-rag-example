pub mod application;
pub mod domain;
pub mod infrastructure;

pub use application::{DeleteDocument, EmbeddingProvider, ListDocuments, UploadDocument};
pub use domain::{Document, DocumentLoader, DocumentRepository};
pub use infrastructure::{
    document_routes, CompositeLoader, PdfLoader, PostgresDocumentRepository, TextLoader,
};
