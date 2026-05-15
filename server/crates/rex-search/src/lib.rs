//! rex-search: the hybrid search pipeline.
//!
//! This crate depends only on `rex-domain`. It contains no I/O — all storage,
//! embedding, and reranking is invoked through the trait ports.

pub mod config;
pub mod fusion;
pub mod highlights;
pub mod pipeline;
pub mod service;

#[cfg(any(test, feature = "fakes"))]
pub mod fakes;

pub use config::SearchConfig;
pub use service::SearchService;
