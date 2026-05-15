//! `SearchService` is the single entry point for both the HTTP API and the CLI.
//! Every public capability of rex lives as a method on this struct.

use std::sync::Arc;
use std::time::Instant;

use rex_domain::{
    BlobStore, Document, DocumentId, Embedder, Error, FacetCount, Filters, FtsIndex, ItemStore,
    PdfAnchor, PdfSummary, Reranker, Result, SearchHit, SearchMeta, SearchMode, SearchQuery,
    SearchResponse, SubjectId, SubjectStats, TagField, TagValue, TimingBreakdown, VectorStore,
};

use crate::config::SearchConfig;
use crate::pipeline;

pub struct SearchService {
    items: Arc<dyn ItemStore>,
    vectors: Arc<dyn VectorStore>,
    fts: Arc<dyn FtsIndex>,
    embedder: Arc<dyn Embedder>,
    reranker: Option<Arc<dyn Reranker>>,
    blobs: Option<Arc<dyn BlobStore>>,
    config: SearchConfig,
}

impl SearchService {
    pub fn builder() -> SearchServiceBuilder {
        SearchServiceBuilder::default()
    }

    pub fn config(&self) -> SearchConfig {
        self.config
    }

    pub fn item_store(&self) -> Arc<dyn ItemStore> {
        Arc::clone(&self.items)
    }

    pub fn blob_store(&self) -> Option<Arc<dyn BlobStore>> {
        self.blobs.clone()
    }

    pub async fn search(&self, q: SearchQuery) -> Result<SearchResponse> {
        validate_query(&q)?;

        let limit = q.limit.clamp(1, 100);

        // Filter-only path
        if q.text.is_none() && matches!(q.mode, SearchMode::Filter) {
            return self.filter(q.filters, limit, 0).await;
        }

        let text_raw = q.text.expect("validated above");
        let text = truncate(&text_raw, self.config.max_query_text);

        let t = Instant::now();
        let out = pipeline::run(
            pipeline::PipelineContext {
                items: self.items.as_ref(),
                vectors: self.vectors.as_ref(),
                fts: self.fts.as_ref(),
                embedder: self.embedder.as_ref(),
                reranker: self.reranker.as_deref(),
                config: &self.config,
            },
            text,
            &q.filters,
            limit,
            q.mode,
            q.rerank,
        )
        .await?;

        let took_ms = t.elapsed().as_millis() as u64;

        Ok(SearchResponse {
            hits: out.hits,
            meta: SearchMeta {
                mode: q.mode,
                used_embedder: out.used_embedder,
                used_bm25: out.used_bm25,
                used_vector: out.used_vector,
                used_reranker: out.used_reranker,
                fts5_query: if out.used_bm25 {
                    Some(if q.exact {
                        format!("\"{}\"", text)
                    } else {
                        text.to_string()
                    })
                } else {
                    None
                },
                total_matches: None,
                took_ms,
                took_breakdown: out.timing,
            },
        })
    }

    pub async fn filter(
        &self,
        filters: Filters,
        limit: usize,
        offset: usize,
    ) -> Result<SearchResponse> {
        let t = Instant::now();
        let limit = limit.clamp(1, 100);
        let docs = self.items.query(&filters, limit, offset).await?;
        let total = self.items.count(&filters).await?;
        let hits: Vec<SearchHit> = docs
            .into_iter()
            .map(|d| SearchHit {
                document: d,
                score: 0.0,
                scores: rex_domain::ScoreBreakdown::default(),
                highlights: Vec::new(),
            })
            .collect();
        let took_ms = t.elapsed().as_millis() as u64;
        Ok(SearchResponse {
            hits,
            meta: SearchMeta {
                mode: SearchMode::Filter,
                used_embedder: false,
                used_bm25: false,
                used_vector: false,
                used_reranker: false,
                fts5_query: None,
                total_matches: Some(total),
                took_ms,
                took_breakdown: TimingBreakdown {
                    hydrate_ms: Some(took_ms),
                    ..Default::default()
                },
            },
        })
    }

    pub async fn get(&self, id: &DocumentId) -> Result<Document> {
        self.items
            .get(id)
            .await?
            .ok_or_else(|| Error::not_found(format!("document {id}")))
    }

    pub async fn get_many(&self, ids: &[DocumentId]) -> Result<Vec<Document>> {
        self.items.get_many(ids).await
    }

    pub async fn list_subjects(&self) -> Result<Vec<SubjectStats>> {
        self.items.list_subjects().await
    }

    pub async fn list_subject(&self, id: &SubjectId) -> Result<SubjectStats> {
        self.list_subjects()
            .await?
            .into_iter()
            .find(|s| &s.id == id)
            .ok_or_else(|| Error::not_found(format!("subject {id}")))
    }

    pub async fn facet_counts(
        &self,
        subject: &SubjectId,
        field: TagField,
        filters: Filters,
    ) -> Result<Vec<FacetCount>> {
        // Ensure the filter scopes to this subject regardless of caller intent.
        let mut f = filters;
        f.subject = Some(subject.clone());
        self.items.facet_counts(subject, field, &f).await
    }

    pub async fn list_topics(&self, subject: &SubjectId) -> Result<Vec<TagValue>> {
        self.items.list_topics(subject).await
    }

    pub async fn pdf_anchor(&self, id: &DocumentId) -> Result<PdfAnchor> {
        let doc = self.get(id).await?;
        doc.pdf_anchor
            .ok_or_else(|| Error::not_found(format!("pdf_anchor for {id}")))
    }

    pub async fn list_pdfs(&self, subject: &SubjectId) -> Result<Vec<PdfSummary>> {
        self.items.list_pdfs(subject).await
    }
}

fn validate_query(q: &SearchQuery) -> Result<()> {
    match (&q.text, q.mode) {
        (Some(t), m) if matches!(m, SearchMode::Filter) => {
            return Err(Error::bad_input_field(
                format!(
                    "text {:?} provided with mode=Filter; use /v1/filter or `rex filter`",
                    t.chars().take(30).collect::<String>()
                ),
                "mode",
            ));
        }
        (Some(t), _) if t.trim().is_empty() => {
            return Err(Error::bad_input_field(
                "text is empty",
                "text",
            ));
        }
        (None, m) if !matches!(m, SearchMode::Filter) => {
            return Err(Error::bad_input_field(
                "text is required for non-Filter modes",
                "text",
            ));
        }
        _ => {}
    }
    Ok(())
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        let mut end = max;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        &s[..end]
    }
}

#[derive(Default)]
pub struct SearchServiceBuilder {
    items: Option<Arc<dyn ItemStore>>,
    vectors: Option<Arc<dyn VectorStore>>,
    fts: Option<Arc<dyn FtsIndex>>,
    embedder: Option<Arc<dyn Embedder>>,
    reranker: Option<Arc<dyn Reranker>>,
    blobs: Option<Arc<dyn BlobStore>>,
    config: Option<SearchConfig>,
}

impl SearchServiceBuilder {
    pub fn items(mut self, v: Arc<dyn ItemStore>) -> Self {
        self.items = Some(v);
        self
    }
    pub fn vectors(mut self, v: Arc<dyn VectorStore>) -> Self {
        self.vectors = Some(v);
        self
    }
    pub fn fts(mut self, v: Arc<dyn FtsIndex>) -> Self {
        self.fts = Some(v);
        self
    }
    pub fn embedder(mut self, v: Arc<dyn Embedder>) -> Self {
        self.embedder = Some(v);
        self
    }
    pub fn reranker(mut self, v: Arc<dyn Reranker>) -> Self {
        self.reranker = Some(v);
        self
    }
    pub fn blobs(mut self, v: Arc<dyn BlobStore>) -> Self {
        self.blobs = Some(v);
        self
    }
    pub fn config(mut self, c: SearchConfig) -> Self {
        self.config = Some(c);
        self
    }

    pub fn build(self) -> Result<SearchService> {
        Ok(SearchService {
            items: self
                .items
                .ok_or_else(|| Error::internal("SearchService: items not set"))?,
            vectors: self
                .vectors
                .ok_or_else(|| Error::internal("SearchService: vectors not set"))?,
            fts: self
                .fts
                .ok_or_else(|| Error::internal("SearchService: fts not set"))?,
            embedder: self
                .embedder
                .ok_or_else(|| Error::internal("SearchService: embedder not set"))?,
            reranker: self.reranker,
            blobs: self.blobs,
            config: self.config.unwrap_or_default(),
        })
    }
}

// Exposed so `pipeline` can build candidate text without duplicating the logic.
pub(crate) fn highlights_candidate_text(d: &Document) -> String {
    let mut s = String::new();
    if let Some(c) = &d.context {
        s.push_str(c);
        s.push(' ');
    }
    if let Some(q) = &d.question {
        s.push_str(q);
        s.push(' ');
    }
    if let Some(a) = &d.answer {
        s.push_str(a);
    }
    s
}
