use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use rex_domain::{SearchQuery, SearchResponse};

use crate::error::ApiError;
use crate::state::AppState;

pub async fn search(
    State(state): State<Arc<AppState>>,
    Json(query): Json<SearchQuery>,
) -> Result<Json<SearchResponse>, ApiError> {
    let resp = state.service.search(query).await?;
    Ok(Json(resp))
}
