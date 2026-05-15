use std::path::Path as StdPath;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use rex_domain::{Document, DocumentId, Error, PdfAnchor};

use crate::error::ApiError;
use crate::state::AppState;

pub async fn get_document(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Document>, ApiError> {
    let id = DocumentId::parse(&id)
        .map_err(|e| Error::bad_input_field(format!("invalid id: {e}"), "id"))?;
    let doc = state.service.get(&id).await?;
    Ok(Json(doc))
}

pub async fn get_pdf_anchor(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<PdfAnchor>, ApiError> {
    let id = DocumentId::parse(&id)
        .map_err(|e| Error::bad_input_field(format!("invalid id: {e}"), "id"))?;
    let anchor = state.service.pdf_anchor(&id).await?;
    Ok(Json(anchor))
}

pub async fn get_pdf(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let blobs = state.blobs.clone().ok_or_else(|| {
        Error::not_found("PDF blob store not configured on this server")
    })?;

    let id = DocumentId::parse(&id)
        .map_err(|e| Error::bad_input_field(format!("invalid id: {e}"), "id"))?;
    let anchor = state.service.pdf_anchor(&id).await?;

    let bytes = blobs.get(&anchor.pdf_path).await?;
    let filename = anchor
        .pdf_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("document.pdf")
        .to_string();
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/pdf"),
            (
                header::CONTENT_DISPOSITION,
                Box::leak(format!("inline; filename=\"{}\"", filename).into_boxed_str()),
            ),
        ],
        Body::from(bytes),
    )
        .into_response())
}

#[allow(dead_code)]
fn ext_is_pdf(p: &StdPath) -> bool {
    p.extension().and_then(|s| s.to_str()) == Some("pdf")
}
