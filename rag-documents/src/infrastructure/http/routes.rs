use axum::{
    routing::{delete, get, post},
    Router,
};

use super::handlers;

pub fn document_routes() -> Router {
    Router::new()
        .route("/documents", post(handlers::upload_document_handler))
        .route("/documents", get(handlers::list_documents_handler))
        .route("/documents/{id}", delete(handlers::delete_document_handler))
}
