//! Port traits — the contracts every adapter must satisfy.
//!
//! Implementations live in `rex-sqlite`, `rex-llamacpp`, `rex-fs-local`, etc.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use bytes::Bytes;

use crate::document::{Document, TagField};
use crate::embedding::Embedding;
use crate::error::Result;
use crate::ids::{DocumentId, SubjectId, TagValue};
use crate::search::{FacetCount, Filters, PdfSummary, SubjectStats};

#[async_trait]
pub trait ItemStore: Send + Sync {
    async fn put(&self, docs: &[Document]) -> Result<()>;
    async fn get(&self, id: &DocumentId) -> Result<Option<Document>>;
    async fn get_many(&self, ids: &[DocumentId]) -> Result<Vec<Document>>;
    async fn query(
        &self,
        filters: &Filters,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Document>>;
    async fn count(&self, filters: &Filters) -> Result<u64>;
    async fn list_subjects(&self) -> Result<Vec<SubjectStats>>;
    async fn list_topics(&self, subject: &SubjectId) -> Result<Vec<TagValue>>;
    async fn facet_counts(
        &self,
        subject: &SubjectId,
        field: TagField,
        filters: &Filters,
    ) -> Result<Vec<FacetCount>>;
    /// List the distinct PDFs that any document in `subject` was anchored to,
    /// with per-PDF item counts and page-anchored counts.
    async fn list_pdfs(&self, subject: &SubjectId) -> Result<Vec<PdfSummary>>;
    async fn clear(&self, subject: &SubjectId) -> Result<()>;
    /// Return any of `ids` that already exist under a subject *other than*
    /// `subject`. `documents` keys on a single global `id`, so ingesting a
    /// colliding id would silently overwrite another subject's document via
    /// INSERT OR REPLACE. Ingest calls this to fail loudly instead.
    async fn find_foreign_ids(
        &self,
        subject: &SubjectId,
        ids: &[DocumentId],
    ) -> Result<Vec<(DocumentId, SubjectId)>>;
}

#[async_trait]
pub trait VectorStore: Send + Sync {
    async fn upsert(&self, items: &[(DocumentId, Embedding)]) -> Result<()>;
    async fn search(
        &self,
        query: &Embedding,
        filters: &Filters,
        k: usize,
    ) -> Result<Vec<(DocumentId, f32)>>;
    async fn clear(&self, subject: &SubjectId) -> Result<()>;
    fn dimension(&self) -> usize;
}

#[async_trait]
pub trait FtsIndex: Send + Sync {
    async fn upsert(&self, items: &[(DocumentId, String)]) -> Result<()>;
    async fn search(
        &self,
        query: &str,
        filters: &Filters,
        k: usize,
    ) -> Result<Vec<(DocumentId, f32)>>;
    async fn clear(&self, subject: &SubjectId) -> Result<()>;
}

#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed_query(&self, text: &str) -> Result<Embedding>;
    async fn embed_documents(&self, texts: &[String]) -> Result<Vec<Embedding>>;
    fn dimension(&self) -> usize;
}

#[async_trait]
pub trait Reranker: Send + Sync {
    async fn rerank(
        &self,
        query: &str,
        candidates: &[(DocumentId, String)],
    ) -> Result<Vec<(DocumentId, f32)>>;
}

#[async_trait]
pub trait BlobStore: Send + Sync {
    async fn get(&self, path: &Path) -> Result<Bytes>;
    async fn exists(&self, path: &Path) -> Result<bool>;
    async fn list(&self, prefix: &Path) -> Result<Vec<PathBuf>>;
}
