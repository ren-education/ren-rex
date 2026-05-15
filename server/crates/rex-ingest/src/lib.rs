//! rex-ingest: streams JSONL, resolves PDF anchors, embeds, and writes to the
//! storage layer via the domain ports.

pub mod config;
pub mod error;
pub mod jsonl;
pub mod path_map;
pub mod pipeline;
pub mod stats;
pub mod text;
pub mod validate;

pub use config::IngestConfig;
pub use error::IngestError;
pub use pipeline::{run, IngestServices};
pub use stats::IngestStats;
pub use validate::{
    validate_file, validate_subject, ErrorBucket, ValidateFileReport, ValidateSubjectReport,
};
