pub mod http;
pub mod persistence;

pub use http::{document_routes, handlers, dto};
pub use persistence::{CompositeLoader, PdfLoader, SurrealDocumentRepository, TextLoader};
