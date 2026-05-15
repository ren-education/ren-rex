//! rex-domain — the core types and trait ports for the rex search server.
//!
//! This crate has **zero external runtime dependencies** beyond serde/uuid/thiserror.
//! All concrete adapters (storage, embedder, blob store, etc.) implement the traits
//! defined in [`ports`] and live in their own crates.

pub mod document;
pub mod embedding;
pub mod error;
pub mod ids;
pub mod pdf;
pub mod ports;
pub mod search;

pub use document::{Document, DocumentKind, TagField, Tags};
pub use embedding::Embedding;
pub use error::{Error, Result};
pub use ids::{DocumentId, SourcePath, SubjectId, TagValue};
pub use pdf::{BoundingBox, FallbackReason, PdfAnchor};
pub use ports::{BlobStore, Embedder, FtsIndex, ItemStore, Reranker, VectorStore};
pub use search::{
    FacetCount, FacetCounts, Filters, Highlight, HighlightField, PdfSummary, ScoreBreakdown,
    SearchHit, SearchMeta, SearchMode, SearchQuery, SearchResponse, SubjectStats,
    TimingBreakdown,
};
