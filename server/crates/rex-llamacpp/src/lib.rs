//! rex-llamacpp: GGUF-backed Embedder + Reranker (default: deterministic stub).

#[cfg(feature = "stub")]
pub mod stub;

#[cfg(feature = "stub")]
pub use stub::{StubEmbedder, StubReranker};
