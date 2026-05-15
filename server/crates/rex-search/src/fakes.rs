//! In-memory fake adapters used by tests in this crate and downstream crates.
//!
//! Gated behind `#[cfg(any(test, feature = "fakes"))]` so adapters never
//! accidentally depend on them in production.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use async_trait::async_trait;
use bytes::Bytes;
use rex_domain::{
    BlobStore, Document, DocumentId, Embedder, Embedding, FacetCount, Filters, FtsIndex,
    ItemStore, PdfSummary, Reranker, Result, SubjectId, SubjectStats, TagField, TagValue,
    VectorStore,
};

// ─── ItemStore ─────────────────────────────────────────────────────────

pub struct FakeItemStore {
    inner: Mutex<HashMap<DocumentId, Document>>,
}

impl FakeItemStore {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    pub fn with(docs: impl IntoIterator<Item = Document>) -> Self {
        let mut map = HashMap::new();
        for d in docs {
            map.insert(d.id, d);
        }
        Self {
            inner: Mutex::new(map),
        }
    }
}

impl Default for FakeItemStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ItemStore for FakeItemStore {
    async fn put(&self, docs: &[Document]) -> Result<()> {
        let mut g = self.inner.lock().unwrap();
        for d in docs {
            g.insert(d.id, d.clone());
        }
        Ok(())
    }

    async fn get(&self, id: &DocumentId) -> Result<Option<Document>> {
        let g = self.inner.lock().unwrap();
        Ok(g.get(id).cloned())
    }

    async fn get_many(&self, ids: &[DocumentId]) -> Result<Vec<Document>> {
        let g = self.inner.lock().unwrap();
        Ok(ids.iter().filter_map(|id| g.get(id).cloned()).collect())
    }

    async fn query(
        &self,
        filters: &Filters,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Document>> {
        let g = self.inner.lock().unwrap();
        let mut all: Vec<Document> = g
            .values()
            .filter(|d| filters.matches(d))
            .cloned()
            .collect();
        // Deterministic order: by id.
        all.sort_by_key(|d| d.id.as_uuid());
        Ok(all.into_iter().skip(offset).take(limit).collect())
    }

    async fn count(&self, filters: &Filters) -> Result<u64> {
        let g = self.inner.lock().unwrap();
        Ok(g.values().filter(|d| filters.matches(d)).count() as u64)
    }

    async fn list_subjects(&self) -> Result<Vec<SubjectStats>> {
        let g = self.inner.lock().unwrap();
        let mut buckets: HashMap<SubjectId, (u64, u64)> = HashMap::new();
        for d in g.values() {
            let e = buckets.entry(d.subject.clone()).or_insert((0, 0));
            match d.kind {
                rex_domain::DocumentKind::Question => e.0 += 1,
                rex_domain::DocumentKind::Note => e.1 += 1,
            }
        }
        let mut out: Vec<SubjectStats> = buckets
            .into_iter()
            .map(|(id, (q, n))| SubjectStats {
                id,
                item_count: q + n,
                question_count: q,
                note_count: n,
            })
            .collect();
        out.sort_by(|a, b| a.id.0.cmp(&b.id.0));
        Ok(out)
    }

    async fn list_topics(&self, subject: &SubjectId) -> Result<Vec<TagValue>> {
        let g = self.inner.lock().unwrap();
        let mut topics: std::collections::HashSet<TagValue> = std::collections::HashSet::new();
        for d in g.values() {
            if &d.subject == subject {
                for t in &d.tags.topics {
                    topics.insert(t.clone());
                }
            }
        }
        let mut v: Vec<TagValue> = topics.into_iter().collect();
        v.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(v)
    }

    async fn facet_counts(
        &self,
        subject: &SubjectId,
        field: TagField,
        filters: &Filters,
    ) -> Result<Vec<FacetCount>> {
        let g = self.inner.lock().unwrap();
        let mut counts: HashMap<TagValue, u64> = HashMap::new();
        for d in g.values() {
            if &d.subject != subject {
                continue;
            }
            if !filters.matches(d) {
                continue;
            }
            for v in d.tags.values_for(field) {
                *counts.entry(v.clone()).or_insert(0) += 1;
            }
        }
        let mut out: Vec<FacetCount> = counts
            .into_iter()
            .map(|(value, count)| FacetCount { value, count })
            .collect();
        out.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.value.0.cmp(&b.value.0)));
        Ok(out)
    }

    async fn list_pdfs(&self, subject: &SubjectId) -> Result<Vec<PdfSummary>> {
        let g = self.inner.lock().unwrap();
        let mut by_path: HashMap<std::path::PathBuf, (u64, u64)> = HashMap::new();
        for d in g.values() {
            if &d.subject != subject {
                continue;
            }
            if let Some(anchor) = &d.pdf_anchor {
                let entry = by_path.entry(anchor.pdf_path.clone()).or_insert((0, 0));
                entry.0 += 1;
                if anchor.page_number.is_some() {
                    entry.1 += 1;
                }
            }
        }
        let mut out: Vec<PdfSummary> = by_path
            .into_iter()
            .map(|(pdf_path, (item_count, page_anchored_count))| PdfSummary {
                pdf_path,
                item_count,
                page_anchored_count,
            })
            .collect();
        out.sort_by(|a, b| a.pdf_path.cmp(&b.pdf_path));
        Ok(out)
    }

    async fn clear(&self, subject: &SubjectId) -> Result<()> {
        let mut g = self.inner.lock().unwrap();
        g.retain(|_, d| &d.subject != subject);
        Ok(())
    }
}

// ─── VectorStore ───────────────────────────────────────────────────────

pub struct FakeVectorStore {
    items: Mutex<Vec<(DocumentId, Vec<f32>, SubjectId)>>,
    item_lookup: std::sync::Arc<FakeItemStore>,
    dim: usize,
}

impl FakeVectorStore {
    pub fn new(dim: usize, item_lookup: std::sync::Arc<FakeItemStore>) -> Self {
        Self {
            items: Mutex::new(Vec::new()),
            item_lookup,
            dim,
        }
    }
}

#[async_trait]
impl VectorStore for FakeVectorStore {
    async fn upsert(&self, items: &[(DocumentId, Embedding)]) -> Result<()> {
        let lookup = self.item_lookup.inner.lock().unwrap();
        let mut g = self.items.lock().unwrap();
        for (id, emb) in items {
            let subject = lookup
                .get(id)
                .map(|d| d.subject.clone())
                .unwrap_or_else(|| SubjectId::new(""));
            // Remove any existing entry for this id, then insert.
            g.retain(|(eid, _, _)| eid != id);
            g.push((*id, emb.as_slice().to_vec(), subject));
        }
        Ok(())
    }

    async fn search(
        &self,
        query: &Embedding,
        filters: &Filters,
        k: usize,
    ) -> Result<Vec<(DocumentId, f32)>> {
        let g = self.items.lock().unwrap();
        let lookup = self.item_lookup.inner.lock().unwrap();
        let mut scored: Vec<(DocumentId, f32)> = g
            .iter()
            .filter_map(|(id, vec, _)| {
                let doc = lookup.get(id)?;
                if !filters.matches(&doc) {
                    return None;
                }
                let sim = cosine(query.as_slice(), vec);
                Some((*id, sim))
            })
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(k);
        Ok(scored)
    }

    async fn clear(&self, subject: &SubjectId) -> Result<()> {
        let mut g = self.items.lock().unwrap();
        g.retain(|(_, _, s)| s != subject);
        Ok(())
    }

    fn dimension(&self) -> usize {
        self.dim
    }
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    let denom = (na.sqrt() * nb.sqrt()).max(f32::EPSILON);
    dot / denom
}

// ─── FtsIndex ──────────────────────────────────────────────────────────

pub struct FakeFtsIndex {
    items: Mutex<Vec<(DocumentId, String, SubjectId)>>,
    item_lookup: std::sync::Arc<FakeItemStore>,
}

impl FakeFtsIndex {
    pub fn new(item_lookup: std::sync::Arc<FakeItemStore>) -> Self {
        Self {
            items: Mutex::new(Vec::new()),
            item_lookup,
        }
    }
}

#[async_trait]
impl FtsIndex for FakeFtsIndex {
    async fn upsert(&self, items: &[(DocumentId, String)]) -> Result<()> {
        let lookup = self.item_lookup.inner.lock().unwrap();
        let mut g = self.items.lock().unwrap();
        for (id, text) in items {
            let subject = lookup
                .get(id)
                .map(|d| d.subject.clone())
                .unwrap_or_else(|| SubjectId::new(""));
            g.retain(|(eid, _, _)| eid != id);
            g.push((*id, text.clone(), subject));
        }
        Ok(())
    }

    async fn search(
        &self,
        query: &str,
        filters: &Filters,
        k: usize,
    ) -> Result<Vec<(DocumentId, f32)>> {
        let q_terms: Vec<String> = query
            .split(|c: char| !c.is_alphanumeric())
            .filter(|t| t.len() >= 2)
            .map(|t| t.to_lowercase())
            .collect();

        let g = self.items.lock().unwrap();
        let lookup = self.item_lookup.inner.lock().unwrap();

        let mut scored: Vec<(DocumentId, f32)> = g
            .iter()
            .filter_map(|(id, text, _)| {
                let doc = lookup.get(id)?;
                if !filters.matches(&doc) {
                    return None;
                }
                let lower = text.to_lowercase();
                let score: f32 = q_terms
                    .iter()
                    .map(|t| lower.matches(t.as_str()).count() as f32)
                    .sum();
                if score > 0.0 {
                    Some((*id, score))
                } else {
                    None
                }
            })
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(k);
        Ok(scored)
    }

    async fn clear(&self, subject: &SubjectId) -> Result<()> {
        let mut g = self.items.lock().unwrap();
        g.retain(|(_, _, s)| s != subject);
        Ok(())
    }
}

// ─── Spy Embedder (records every call for assertions) ──────────────────

pub struct SpyEmbedder {
    inner: Mutex<Vec<String>>,
    dim: usize,
}

impl SpyEmbedder {
    pub fn new(dim: usize) -> Self {
        Self {
            inner: Mutex::new(Vec::new()),
            dim,
        }
    }

    pub fn calls(&self) -> Vec<String> {
        self.inner.lock().unwrap().clone()
    }

    pub fn reset(&self) {
        self.inner.lock().unwrap().clear();
    }
}

#[async_trait]
impl Embedder for SpyEmbedder {
    async fn embed_query(&self, text: &str) -> Result<Embedding> {
        self.inner.lock().unwrap().push(format!("query:{text}"));
        // Deterministic: byte-position seeded vector.
        let v = stub_vector(text, self.dim);
        Embedding::new(self.dim, v)
    }

    async fn embed_documents(&self, texts: &[String]) -> Result<Vec<Embedding>> {
        let mut out = Vec::with_capacity(texts.len());
        for t in texts {
            self.inner.lock().unwrap().push(format!("doc:{t}"));
            let v = stub_vector(t, self.dim);
            out.push(Embedding::new(self.dim, v)?);
        }
        Ok(out)
    }

    fn dimension(&self) -> usize {
        self.dim
    }
}

fn stub_vector(text: &str, dim: usize) -> Vec<f32> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut v = vec![0.0f32; dim];
    for tok in text.split(|c: char| !c.is_alphanumeric()).filter(|t| t.len() >= 2) {
        let mut h = DefaultHasher::new();
        tok.to_lowercase().hash(&mut h);
        let seed = h.finish();
        for i in 0..4u64 {
            let idx = (seed.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(i) as usize) % dim;
            let sign = if (seed >> (i + 1)) & 1 == 0 { 1.0 } else { -1.0 };
            v[idx] += sign;
        }
    }
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in &mut v {
            *x /= norm;
        }
    }
    v
}

// ─── Spy Reranker ──────────────────────────────────────────────────────

pub struct SpyReranker {
    calls: Mutex<Vec<(String, usize)>>,
}

impl SpyReranker {
    pub fn new() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
        }
    }

    pub fn calls(&self) -> Vec<(String, usize)> {
        self.calls.lock().unwrap().clone()
    }
}

impl Default for SpyReranker {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Reranker for SpyReranker {
    async fn rerank(
        &self,
        query: &str,
        candidates: &[(DocumentId, String)],
    ) -> Result<Vec<(DocumentId, f32)>> {
        self.calls
            .lock()
            .unwrap()
            .push((query.to_string(), candidates.len()));
        // Score by token overlap (simple but deterministic).
        let q: std::collections::HashSet<String> = query
            .split(|c: char| !c.is_alphanumeric())
            .filter(|t| t.len() >= 2)
            .map(|t| t.to_lowercase())
            .collect();
        let mut scored: Vec<_> = candidates
            .iter()
            .map(|(id, text)| {
                let ts: std::collections::HashSet<String> = text
                    .split(|c: char| !c.is_alphanumeric())
                    .filter(|t| t.len() >= 2)
                    .map(|t| t.to_lowercase())
                    .collect();
                let inter = q.intersection(&ts).count() as f32;
                let union = q.union(&ts).count().max(1) as f32;
                (*id, inter / union)
            })
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(scored)
    }
}

// ─── BlobStore (in-memory) ─────────────────────────────────────────────

pub struct FakeBlobStore {
    files: Mutex<HashMap<PathBuf, Bytes>>,
}

impl FakeBlobStore {
    pub fn new() -> Self {
        Self {
            files: Mutex::new(HashMap::new()),
        }
    }

    pub fn put(&self, path: impl Into<PathBuf>, bytes: impl Into<Bytes>) {
        self.files.lock().unwrap().insert(path.into(), bytes.into());
    }
}

impl Default for FakeBlobStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BlobStore for FakeBlobStore {
    async fn get(&self, path: &Path) -> Result<Bytes> {
        self.files
            .lock()
            .unwrap()
            .get(path)
            .cloned()
            .ok_or_else(|| rex_domain::Error::not_found(path.display().to_string()))
    }

    async fn exists(&self, path: &Path) -> Result<bool> {
        Ok(self.files.lock().unwrap().contains_key(path))
    }

    async fn list(&self, prefix: &Path) -> Result<Vec<PathBuf>> {
        let g = self.files.lock().unwrap();
        Ok(g.keys()
            .filter(|p| p.starts_with(prefix))
            .cloned()
            .collect())
    }
}
