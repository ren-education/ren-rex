//! Search request and response types.

use serde::{Deserialize, Serialize};

use crate::document::{Document, DocumentKind};
use crate::ids::{SubjectId, TagValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchMode {
    Hybrid,
    Bm25Only,
    VectorOnly,
    Filter,
}

impl Default for SearchMode {
    fn default() -> Self {
        SearchMode::Hybrid
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Filters {
    pub subject: Option<SubjectId>,
    #[serde(default)]
    pub topics: Vec<TagValue>,
    #[serde(default)]
    pub question_types: Vec<TagValue>,
    #[serde(default)]
    pub paper_types: Vec<TagValue>,
    #[serde(default)]
    pub schools: Vec<TagValue>,
    #[serde(default)]
    pub source_types: Vec<TagValue>,
    #[serde(default)]
    pub exam_systems: Vec<TagValue>,
    pub marks_range: Option<(u32, u32)>,
    pub kind: Option<DocumentKind>,
}

impl Filters {
    /// Does this filter match every document? (subject = None and all lists empty)
    pub fn is_match_all(&self) -> bool {
        self.subject.is_none()
            && self.topics.is_empty()
            && self.question_types.is_empty()
            && self.paper_types.is_empty()
            && self.schools.is_empty()
            && self.source_types.is_empty()
            && self.exam_systems.is_empty()
            && self.marks_range.is_none()
            && self.kind.is_none()
    }

    /// Apply the filter to a Document in pure-Rust (used by FakeItemStore).
    pub fn matches(&self, doc: &Document) -> bool {
        if let Some(s) = &self.subject {
            if &doc.subject != s {
                return false;
            }
        }
        if let Some(k) = self.kind {
            if doc.kind != k {
                return false;
            }
        }
        if let Some((lo, hi)) = self.marks_range {
            match doc.mark {
                Some(m) if m >= lo && m <= hi => {}
                _ => return false,
            }
        }

        fn or_match(needles: &[TagValue], haystack: &[TagValue]) -> bool {
            needles.is_empty() || needles.iter().any(|n| haystack.contains(n))
        }

        or_match(&self.topics, &doc.tags.topics)
            && or_match(&self.question_types, &doc.tags.question_types)
            && or_match(&self.paper_types, &doc.tags.paper_types)
            && or_match(&self.schools, &doc.tags.schools)
            && or_match(&self.source_types, &doc.tags.source_types)
            && or_match(&self.exam_systems, &doc.tags.exam_systems)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub text: Option<String>,
    #[serde(default)]
    pub filters: Filters,
    pub limit: usize,
    #[serde(default)]
    pub mode: SearchMode,
    #[serde(default)]
    pub exact: bool,
    /// Defaults to true. Honored only when mode=Hybrid.
    #[serde(default = "default_true")]
    pub rerank: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub document: Document,
    pub score: f32,
    pub scores: ScoreBreakdown,
    pub highlights: Vec<Highlight>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub bm25: Option<f32>,
    pub vector: Option<f32>,
    pub rerank: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Highlight {
    pub field: HighlightField,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HighlightField {
    Context,
    Question,
    Answer,
    Notes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub hits: Vec<SearchHit>,
    pub meta: SearchMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMeta {
    pub mode: SearchMode,
    pub used_embedder: bool,
    pub used_bm25: bool,
    pub used_vector: bool,
    pub used_reranker: bool,
    pub fts5_query: Option<String>,
    /// Total matches (set only on Filter mode).
    pub total_matches: Option<u64>,
    pub took_ms: u64,
    pub took_breakdown: TimingBreakdown,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TimingBreakdown {
    pub embed_ms: Option<u64>,
    pub bm25_ms: Option<u64>,
    pub vector_ms: Option<u64>,
    pub fuse_ms: Option<u64>,
    pub rerank_ms: Option<u64>,
    pub hydrate_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubjectStats {
    pub id: SubjectId,
    pub item_count: u64,
    pub question_count: u64,
    pub note_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacetCount {
    pub value: TagValue,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacetCounts {
    pub subject: SubjectId,
    pub field: String,
    pub values: Vec<FacetCount>,
}

