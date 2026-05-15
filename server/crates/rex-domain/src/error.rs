//! The single error type returned across all port boundaries.

use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("not found: {what}")]
    NotFound { what: String },

    #[error("bad input: {message}")]
    BadInput {
        message: String,
        field: Option<String>,
    },

    #[error("conflict: {message}")]
    Conflict { message: String },

    #[error("storage error: {source}")]
    Storage {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("embedding model unavailable: {message}")]
    Embedding { message: String },

    #[error("rerank model unavailable: {message}")]
    Reranking { message: String },

    #[error("pdf failure: {message}")]
    Pdf {
        message: String,
        path: Option<PathBuf>,
    },

    #[error("schema mismatch: {message}")]
    SchemaMismatch { message: String },

    #[error("ingest aborted due to drift: {skipped}/{total} rows skipped")]
    SchemaDrift {
        skipped: u64,
        total: u64,
        sample_errors: Vec<String>,
    },

    #[error("internal: {message}")]
    Internal { message: String },
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn not_found(what: impl Into<String>) -> Self {
        Error::NotFound { what: what.into() }
    }

    pub fn bad_input(message: impl Into<String>) -> Self {
        Error::BadInput {
            message: message.into(),
            field: None,
        }
    }

    pub fn bad_input_field(message: impl Into<String>, field: impl Into<String>) -> Self {
        Error::BadInput {
            message: message.into(),
            field: Some(field.into()),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Error::Internal {
            message: message.into(),
        }
    }
}
