//! Route table.

use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

mod documents;
mod filter;
mod health;
mod pdfs;
mod search;
mod subjects;

pub fn routes() -> Router<std::sync::Arc<AppState>> {
    Router::new()
        .route("/v1/health", get(health::health))
        .route("/v1/subjects", get(subjects::list_subjects))
        .route("/v1/subjects/:id", get(subjects::get_subject))
        .route(
            "/v1/subjects/:id/tag-values/:field",
            post(subjects::tag_values),
        )
        .route("/v1/subjects/:id/pdfs", get(pdfs::list_pdfs))
        .route("/v1/search", post(search::search))
        .route("/v1/filter", post(filter::filter))
        .route("/v1/documents/:id", get(documents::get_document))
        .route(
            "/v1/documents/:id/pdf-anchor",
            get(documents::get_pdf_anchor),
        )
        .route("/v1/documents/:id/pdf", get(documents::get_pdf))
}

pub fn declared_routes() -> Vec<&'static str> {
    vec![
        "GET /v1/health",
        "GET /v1/subjects",
        "GET /v1/subjects/:id",
        "POST /v1/subjects/:id/tag-values/:field",
        "GET /v1/subjects/:id/pdfs",
        "POST /v1/search",
        "POST /v1/filter",
        "GET /v1/documents/:id",
        "GET /v1/documents/:id/pdf-anchor",
        "GET /v1/documents/:id/pdf",
    ]
}
