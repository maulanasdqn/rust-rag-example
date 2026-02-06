use axum::{routing::post, Router};

use super::handlers;

pub fn query_routes() -> Router {
    Router::new().route("/query", post(handlers::query_handler))
}
