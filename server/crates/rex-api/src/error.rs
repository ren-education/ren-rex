//! Map `rex_domain::Error` to HTTP responses per spec §8.5.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use rex_domain::Error as DomainError;
use serde::Serialize;

pub struct ApiError(pub DomainError);

impl From<DomainError> for ApiError {
    fn from(e: DomainError) -> Self {
        ApiError(e)
    }
}

#[derive(Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Serialize)]
struct ErrorDetail {
    code: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<serde_json::Value>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, details) = match &self.0 {
            DomainError::NotFound { .. } => (StatusCode::NOT_FOUND, "not_found", None),
            DomainError::BadInput { field, .. } => (
                StatusCode::BAD_REQUEST,
                "bad_input",
                field
                    .clone()
                    .map(|f| serde_json::json!({ "field": f })),
            ),
            DomainError::Conflict { .. } => (StatusCode::CONFLICT, "conflict", None),
            DomainError::Embedding { .. } => {
                (StatusCode::SERVICE_UNAVAILABLE, "embedder_unavailable", None)
            }
            DomainError::Reranking { .. } => {
                (StatusCode::SERVICE_UNAVAILABLE, "reranker_unavailable", None)
            }
            DomainError::SchemaMismatch { .. } => {
                (StatusCode::SERVICE_UNAVAILABLE, "schema_mismatch", None)
            }
            DomainError::Pdf { path, .. } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "pdf_failure",
                path.as_ref()
                    .map(|p| serde_json::json!({ "path": p.to_string_lossy() })),
            ),
            DomainError::Storage { .. }
            | DomainError::SchemaDrift { .. }
            | DomainError::Internal { .. } => (StatusCode::INTERNAL_SERVER_ERROR, "internal", None),
        };

        let body = ErrorBody {
            error: ErrorDetail {
                code,
                message: self.0.to_string(),
                details,
            },
        };
        (status, Json(body)).into_response()
    }
}
