use axum::Router;
use rag_documents::infrastructure::http::routes::document_routes;
use rag_query::infrastructure::http::routes::query_routes;
use tower_http::trace::TraceLayer;

pub fn create_router() -> Router {
    Router::new()
        .nest("/api/documents", document_routes())
        .nest("/api", query_routes())
        .layer(TraceLayer::new_for_http())
}
