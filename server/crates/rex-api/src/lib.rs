//! rex-api: axum HTTP layer over a SearchService.
//!
//! Mounts the rex v1 surface under `/v1`. Every endpoint described in
//! spec §8 plus `GET /v1/subjects/:id/pdfs` (the per-subject PDF list).

mod error;
mod handlers;
mod state;

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;

pub use error::ApiError;
pub use state::{AppState, AppStateBuilder};

/// Bind and serve the rex HTTP API. Blocks until the listener fails or the
/// process is terminated. Logs the bound address to stderr on startup.
pub async fn run(state: Arc<AppState>, addr: std::net::SocketAddr) -> std::io::Result<()> {
    let router = build_router(state);
    eprintln!("rex serve · listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await
}

const BODY_LIMIT_BYTES: usize = 1_048_576; // 1 MB
const REQUEST_TIMEOUT_SECS: u64 = 30;

pub fn build_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .merge(handlers::routes())
        .with_state(state)
        .layer(cors)
        .layer(TimeoutLayer::new(Duration::from_secs(REQUEST_TIMEOUT_SECS)))
        .layer(RequestBodyLimitLayer::new(BODY_LIMIT_BYTES))
}

/// Convenience: declared routes (for CLI/API parity testing).
pub fn declared_routes() -> Vec<&'static str> {
    handlers::declared_routes()
}
