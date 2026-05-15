//! Wires concrete adapters into the SearchService + IngestServices.
//!
//! This is the only file in the workspace that mentions every adapter crate
//! by name. Every other crate sees only `rex-domain` traits.

use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use rex_domain::{BlobStore, Embedder, FtsIndex, ItemStore, Reranker, VectorStore};
use rex_search::SearchService;
use rex_sqlite::{open_db, SqliteStore};

pub const EMBEDDING_DIM: usize = 64;

pub struct Adapters {
    pub items: Arc<dyn ItemStore>,
    pub vectors: Arc<dyn VectorStore>,
    pub fts: Arc<dyn FtsIndex>,
    pub blobs: Arc<dyn BlobStore>,
    pub embedder: Arc<dyn Embedder>,
    pub reranker: Option<Arc<dyn Reranker>>,
}

pub fn open_adapters(db_path: &Path, docs_root: Option<&Path>) -> Result<Adapters> {
    let conn = open_db(db_path).with_context(|| {
        format!("failed to open sqlite db at {}", db_path.display())
    })?;
    let store: Arc<SqliteStore> = Arc::new(SqliteStore::new(conn, EMBEDDING_DIM));
    let blobs: Arc<dyn BlobStore> = match docs_root {
        Some(root) => Arc::new(rex_fs_local::LocalFsBlobStore::new(root)),
        None => {
            // A dummy blob store that always reports "not found". Ingest will
            // mark PDFs as not-found which is harmless. Search/get don't need
            // PDF bytes unless the user invokes `rex pdf`.
            Arc::new(NullBlobStore)
        }
    };
    let embedder: Arc<dyn Embedder> = Arc::new(rex_llamacpp::StubEmbedder::new(EMBEDDING_DIM));
    let reranker: Option<Arc<dyn Reranker>> = Some(Arc::new(rex_llamacpp::StubReranker));

    Ok(Adapters {
        items: store.clone() as Arc<dyn ItemStore>,
        vectors: store.clone() as Arc<dyn VectorStore>,
        fts: store.clone() as Arc<dyn FtsIndex>,
        blobs,
        embedder,
        reranker,
    })
}

pub fn build_search_service(adapters: &Adapters) -> Result<SearchService> {
    let mut builder = SearchService::builder()
        .items(adapters.items.clone())
        .vectors(adapters.vectors.clone())
        .fts(adapters.fts.clone())
        .embedder(adapters.embedder.clone())
        .blobs(adapters.blobs.clone());
    if let Some(rr) = &adapters.reranker {
        builder = builder.reranker(rr.clone());
    }
    builder.build().map_err(anyhow::Error::from)
}

struct NullBlobStore;

#[async_trait::async_trait]
impl BlobStore for NullBlobStore {
    async fn get(&self, path: &std::path::Path) -> rex_domain::Result<bytes::Bytes> {
        Err(rex_domain::Error::not_found(format!(
            "blob store disabled; cannot read {}",
            path.display()
        )))
    }
    async fn exists(&self, _path: &std::path::Path) -> rex_domain::Result<bool> {
        Ok(false)
    }
    async fn list(
        &self,
        _prefix: &std::path::Path,
    ) -> rex_domain::Result<Vec<std::path::PathBuf>> {
        Ok(Vec::new())
    }
}
