use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use rex_domain::{Filters, SearchResponse};
use serde::Deserialize;

use crate::error::ApiError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct FilterBody {
    #[serde(default)]
    pub filters: Filters,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

fn default_limit() -> usize {
    50
}

pub async fn filter(
    State(state): State<Arc<AppState>>,
    Json(body): Json<FilterBody>,
) -> Result<Json<SearchResponse>, ApiError> {
    let resp = state
        .service
        .filter(body.filters, body.limit, body.offset)
        .await?;
    Ok(Json(resp))
}
