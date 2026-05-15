use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use rex_domain::{PdfSummary, SubjectId};
use serde::Serialize;

use crate::error::ApiError;
use crate::state::AppState;

#[derive(Serialize)]
pub struct PdfListResponse {
    pub subject: String,
    pub pdfs: Vec<PdfSummary>,
}

pub async fn list_pdfs(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<PdfListResponse>, ApiError> {
    let pdfs = state.service.list_pdfs(&SubjectId::new(id.clone())).await?;
    Ok(Json(PdfListResponse { subject: id, pdfs }))
}
