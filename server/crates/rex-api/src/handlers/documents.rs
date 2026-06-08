use std::path::{Path as StdPath, PathBuf};
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use rex_domain::{Document, DocumentId, Error, PdfAnchor};
use serde::Serialize;

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

pub async fn get_related_files(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<RelatedFilesResponse>, ApiError> {
    let id = DocumentId::parse(&id)
        .map_err(|e| Error::bad_input_field(format!("invalid id: {e}"), "id"))?;
    let doc = state.service.get(&id).await?;

    // No blob store, or the document has no PDF anchor -> empty list (not an error).
    let (blobs, pdf_path) = match (state.blobs.clone(), doc.pdf_anchor) {
        (Some(b), Some(a)) => (b, a.pdf_path),
        _ => {
            return Ok(Json(RelatedFilesResponse {
                dir: String::new(),
                files: vec![],
            }))
        }
    };

    let dir = pdf_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_default();
    // list() swallows missing-dir errors and returns []. Fall back to [] on any error.
    let all = blobs.list(&dir).await.unwrap_or_default();
    let files = related_files_in_dir(&all, &dir, &pdf_path);

    Ok(Json(RelatedFilesResponse {
        dir: dir.to_string_lossy().into_owned(),
        files,
    }))
}

pub async fn get_file(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> Result<Response, ApiError> {
    let blobs = state.blobs.clone().ok_or_else(|| {
        Error::not_found("PDF blob store not configured on this server")
    })?;

    let rel = StdPath::new(&path);
    // Only PDFs are served from this arbitrary-path surface.
    if !ext_is_pdf(rel) {
        return Err(Error::not_found("only PDF files are served").into());
    }

    // safe_join inside the blob store rejects `..` traversal (-> BadInput -> 400).
    // Distinguish a missing file (404) from other storage errors (500).
    if !blobs.exists(rel).await? {
        return Err(Error::not_found(format!("file not found: {path}")).into());
    }
    let bytes = blobs.get(rel).await?;
    let filename = rel
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

#[derive(Serialize)]
pub struct RelatedFile {
    /// Root-relative path, for the `/v1/files/*path` serve endpoint.
    pub path: String,
    /// Basename, for display.
    pub filename: String,
}

#[derive(Serialize)]
pub struct RelatedFilesResponse {
    pub dir: String,
    pub files: Vec<RelatedFile>,
}

fn ext_is_pdf(p: &StdPath) -> bool {
    p.extension()
        .and_then(|s| s.to_str())
        .map(|s| s.eq_ignore_ascii_case("pdf"))
        .unwrap_or(false)
}

/// Filter a recursive blob listing down to the immediate-child PDFs of `dir`,
/// excluding the question's own PDF. Pure, so it is unit-tested without HTTP.
fn related_files_in_dir(all: &[PathBuf], dir: &StdPath, exclude: &StdPath) -> Vec<RelatedFile> {
    let mut out: Vec<RelatedFile> = all
        .iter()
        .filter(|p| p.parent() == Some(dir))
        .filter(|p| p.as_path() != exclude)
        .filter(|p| ext_is_pdf(p))
        .filter_map(|p| {
            let filename = p.file_name()?.to_str()?.to_string();
            Some(RelatedFile {
                path: p.to_string_lossy().into_owned(),
                filename,
            })
        })
        .collect();
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn related_files_in_dir_filters_to_immediate_pdf_siblings() {
        let dir = StdPath::new("h2physics/prelims/2022/TMJC");
        let own = StdPath::new("h2physics/prelims/2022/TMJC/TMJC_2022_H2_Physics_P1_QP.pdf");
        let all = vec![
            PathBuf::from("h2physics/prelims/2022/TMJC/TMJC_2022_H2_Physics_P1_QP.pdf"),
            PathBuf::from("h2physics/prelims/2022/TMJC/10._P1_Solutions.pdf"),
            PathBuf::from("h2physics/prelims/2022/TMJC/notes.txt"),
            PathBuf::from("h2physics/prelims/2022/TMJC/nested/extra.pdf"),
            PathBuf::from("h2physics/prelims/2022/RI/RI_P1_QP.pdf"),
        ];
        let out = related_files_in_dir(&all, dir, own);
        assert_eq!(out.len(), 1, "only the immediate-child solutions PDF survives");
        assert_eq!(out[0].filename, "10._P1_Solutions.pdf");
        assert_eq!(out[0].path, "h2physics/prelims/2022/TMJC/10._P1_Solutions.pdf");
    }

    #[test]
    fn ext_is_pdf_is_case_insensitive_and_rejects_others() {
        assert!(ext_is_pdf(StdPath::new("a/b/c.pdf")));
        assert!(ext_is_pdf(StdPath::new("a/b/c.PDF")));
        assert!(!ext_is_pdf(StdPath::new("a/b/c.txt")));
        assert!(!ext_is_pdf(StdPath::new("a/b/c")));
    }
}
