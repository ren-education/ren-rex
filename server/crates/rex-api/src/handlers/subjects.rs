use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use rex_domain::{Filters, SubjectId, SubjectStats, TagField};
use serde::Deserialize;

use crate::error::ApiError;
use crate::state::AppState;

pub async fn list_subjects(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<SubjectStats>>, ApiError> {
    let s = state.service.list_subjects().await?;
    Ok(Json(s))
}

pub async fn get_subject(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SubjectStats>, ApiError> {
    let stats = state.service.list_subject(&SubjectId::new(id)).await?;
    Ok(Json(stats))
}

#[derive(Deserialize, Default)]
pub struct TagValuesBody {
    #[serde(default)]
    pub filters: Filters,
}

pub async fn tag_values(
    State(state): State<Arc<AppState>>,
    Path((id, field)): Path<(String, String)>,
    body: Option<Json<TagValuesBody>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let field = TagField::from_db_str(&field).ok_or_else(|| {
        ApiError::from(rex_domain::Error::bad_input_field(
            format!("unknown tag field: {field}"),
            "field",
        ))
    })?;
    let filters = body.map(|b| b.0.filters).unwrap_or_default();
    let counts = state
        .service
        .facet_counts(&SubjectId::new(id.clone()), field, filters)
        .await?;
    Ok(Json(serde_json::json!({
        "subject": id,
        "field": field.as_db_str(),
        "values": counts,
    })))
}
