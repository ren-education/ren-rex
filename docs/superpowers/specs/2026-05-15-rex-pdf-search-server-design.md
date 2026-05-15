# rex — PDF Search & Navigator Server: Design Spec

**Status:** Draft
**Date:** 2026-05-15
**Author:** brainstormed with the user

---

## 1. Context & motivation

The `ren-subjects` repo holds, per subject (h2physics, hcchem, h2history, etc.):

- **Raw PDFs** under `ren-subjects/docs/<subject>/{prelims,promos,holy-grail-site}/...`
- **Markdown extractions** of those PDFs under `ren-subjects/workspace/<subject>/content/...` produced by `rob-the-crawler`
- **Compiled question/note items** under `ren-subjects/workspace/<subject>/reference/{questions,notes}.jsonl` — produced by passing the markdown to an LLM. Each row is a structured item with `id`, `parent_id`, `context`, `question`, `answer`, `keywords`, `tags`, and a `source` pointer back to the markdown file. As of this writing, h2physics alone has ~7,800 question items.

Students and teachers need an interface to **find relevant questions, notes, and PDF pages** by keyword + filters + semantic similarity. This spec defines the **server-side architecture** of that interface. The client (web UI, possibly built later in Next.js or similar) is out of scope.

The server is named **`rex`** and lives in the `ren-rex` git repository at `/Users/jcjustin/Projects/tippytop/ren-rex/`. It is a Rust workspace.

## 2. Goals & non-goals

### Goals (v1)

1. Ingest `questions.jsonl` and `notes.jsonl` for any subject into a queryable index.
2. Support **hybrid search** (BM25 + vector + cross-encoder rerank) with per-mode selection.
3. Support **filter-only browsing** over structured tags (topics, schools, paper types, etc.).
4. Resolve each item to a **PDF location** (`pdf_path` + `page_number`) at ingest time.
5. Expose **both an HTTP API and a CLI** with full parity (every API capability has a CLI subcommand).
6. Be **modular** by construction — pluggable storage, embedder, reranker, blob storage — so future swaps (SQLite → Postgres, local FS → S3, GGUF model → remote API) are isolated to a single crate.
7. Be **multi-subject from day 1**: subject is a first-class entity in the data model and API.
8. Surface **observability hooks** sufficient to monitor RAM and per-component memory in production.

### Non-goals (v1)

- A web/mobile UI. Clients are out of scope; the API + CLI are the deliverables.
- Authentication / authorization. Single-tenant, internal-network deployment for v1.
- Multi-tenant isolation, rate limiting, request quotas.
- Incremental / streaming ingestion. `rex ingest` does a full re-index for the named subject.
- HTTP-triggered ingestion. Only the CLI can write to the index in v1.
- Indexing raw PDF page text as a separate search corpus. (The per-question PDF *anchor* is in scope; full-text PDF search is a possible v1.5 feature; the architecture is set up to add it cheaply.)
- Filesystem watcher / auto-reindex.
- Distributed indexing or sharding.
- Live model reload, A/B testing of embedders, query expansion, generative answering.
- `pprof` profiling endpoint, `tokio-console` integration. (Deferred to v1.1+.)
- Shared JSON Schema across rob and rex. (Deferred; contract is defended via serde + drift threshold.)

## 3. Decisions summary

| Topic | Decision |
|---|---|
| Code structure | Cargo workspace with 9 crates (hexagonal, compiler-enforced boundaries) |
| Search unit | Flat question/note items |
| Subject scope | Multi-subject from day 1 (first-class entity) |
| Embedder | `llama-cpp-2` + `embeddinggemma-300M-Q8_0` GGUF, behind `Embedder` trait |
| Reranker | `llama-cpp-2` + `Qwen3-Reranker-0.6B-Q8_0` GGUF, behind `Reranker` trait |
| Storage | SQLite + `sqlite-vec` + FTS5, single file `rex.db`, WAL mode |
| Filesystem | Local FS via `BlobStore` trait |
| Pipeline modes | `Hybrid` (default), `Bm25Only`, `VectorOnly`, `Filter` |
| Fusion | Reciprocal Rank Fusion (RRF), constant `k = 60` |
| Highlighting | Term-based span extraction |
| PDF anchors | Re-extract PDF text via `pdfium-render`, fuzzy-match question → page; file-level fallback |
| Ingest model | `rex ingest` / `rex serve` subcommands, full reindex per subject |
| Frontend parity | Service-layer + CI parity test; CLI runs in-process with lazy-loaded embedder |
| Ingest channel | CLI only in v1 (no HTTP ingest endpoint) |
| API | REST + JSON over `axum`, `/v1` versioned, stable `error.code` map |
| Contract testing | `#[serde(deny_unknown_fields)]` + >5% drift-threshold abort |
| Response metadata | Per-stage timings, per-hit score breakdown, mode, `fts5_query` |
| Profiling hooks | Prometheus metrics for process RSS/threads/FDs + per-component memory |

## 4. Architecture

### 4.1 Workspace and crate graph

`ren-rex/Cargo.toml` is a `[workspace]`. Members live under `crates/`.

```
crates/
├── rex-domain        ← zero external deps. Pure data + trait definitions (ports).
├── rex-search        ← deps: rex-domain. The query pipeline (BM25 + vec + rerank).
├── rex-ingest        ← deps: rex-domain. JSONL parser + indexing orchestrator.
├── rex-pdf           ← deps: rex-domain. PDF text extraction + page anchoring.
├── rex-sqlite        ← deps: rex-domain. Impls ItemStore, VectorStore, FtsIndex.
├── rex-llamacpp      ← deps: rex-domain. Impls Embedder + Reranker (GGUF).
├── rex-fs-local      ← deps: rex-domain. Impls BlobStore (PDF bytes).
├── rex-api           ← deps: rex-domain, rex-search. axum HTTP layer.
└── rex-cli           ← deps: ALL above. Binary `rex`; wires concrete adapters.
```

Dependency graph (arrows = "depends on"):

```
            ┌──────────────────────────────────────────┐
            │              rex-cli (binary)            │
            └──────────────────────────────────────────┘
              │      │      │      │      │      │
              ▼      ▼      ▼      ▼      ▼      ▼
         rex-api  rex-ingest  rex-sqlite  rex-llamacpp  rex-fs-local  rex-pdf
              │      │      │      │           │           │
              └─rex-search   │      │           │           │
                  │          │      │           │           │
                  └──────────┴──────┴───────────┴───────────┘
                                     │
                                     ▼
                                rex-domain  (no deps)
```

**Invariants enforced by the workspace:**

- `rex-domain` has zero external dependencies (no I/O crates, no async runtime). Its `Cargo.toml` is asserted by a workspace lint to depend only on `serde`, `uuid`, `thiserror`, and similar utility crates.
- Adapter crates (`rex-sqlite`, `rex-llamacpp`, `rex-fs-local`, `rex-pdf`) never depend on each other. The compiler refuses to compile a violation.
- `rex-search` and `rex-ingest` depend on `rex-domain` only — they have no knowledge of which concrete adapter is in use.
- `rex-cli` is the *only* crate that wires concrete adapter types into the service layer.

### 4.2 Runtime topology

```
                       ┌─────────────────────┐
   ren-subjects/  ───▶ │ rex ingest          │ ──▶ rex.db (SQLite + sqlite-vec + FTS5)
   workspace/         │  (one-shot CLI)     │
                       └─────────────────────┘
                                                       reads
                                                          │
                       ┌─────────────────────┐            ▼
   HTTP clients  ◀──── │ rex serve           │ ◀── rex.db
                       │  (axum, async)      │
                       │  + GGUF models      │
                       │    in process mem   │
                       └─────────────────────┘
                                ▲
                                │ reads PDF bytes by path
                                ▼
                       ren-subjects/docs/<subject>/
```

Two processes, one shared file (`rex.db`). Ingest writes; serve reads. SQLite WAL mode permits multiple concurrent readers + one writer with no special server. There is no shared in-memory state between the two processes.

**The DB is portable.** Ingest can be run on a beefier dev machine; the resulting `rex.db` (typically ~140 MB per subject; ~1.1 GB for 8 subjects) is `scp`'d to the EC2 deploy target. EC2 only needs the embedder loaded at serve time (and only the embedder if `--no-reranker`).

## 5. Domain model & ports (`rex-domain`)

`rex-domain` contains **plain Rust types** (no async, no I/O) and **trait definitions** (the ports). No implementation lives here.

### 5.1 Core types

```rust
pub struct SubjectId(String);                     // "h2physics", "hcchem"
pub struct DocumentId(Uuid);                       // from JSONL `id` field
pub struct TagValue(String);                       // "dynamics", "paper-2", "hci"
pub struct SourcePath(PathBuf);                    // markdown path, relative to workspace

pub enum DocumentKind { Question, Note }

pub struct Document {
    pub id: DocumentId,
    pub subject: SubjectId,
    pub kind: DocumentKind,
    pub parent_id: Option<DocumentId>,             // for child question parts
    pub number: Option<String>,                     // "1(a)(i)"
    pub source: SourcePath,
    pub context: Option<String>,
    pub question: Option<String>,
    pub answer: Option<String>,
    pub notes: Option<String>,
    pub mark: Option<u32>,
    pub options: Option<Vec<String>>,
    pub keywords: Vec<String>,
    pub tags: Tags,
    pub pdf_anchor: Option<PdfAnchor>,             // set during ingest
}

pub struct Tags {
    pub topics:         Vec<TagValue>,
    pub question_types: Vec<TagValue>,
    pub exam_systems:   Vec<TagValue>,
    pub paper_types:    Vec<TagValue>,
    pub schools:        Vec<TagValue>,
    pub source_types:   Vec<TagValue>,
}

pub struct PdfAnchor {
    pub pdf_path: PathBuf,                          // relative to BlobStore root
    pub page_number: Option<u32>,                   // None = file-level fallback
    pub bbox: Option<BoundingBox>,                  // optional, usually None in v1
    pub confidence: f32,                            // fuzzy-match score, 0.0-1.0
}

pub struct BoundingBox { pub x: f32, pub y: f32, pub w: f32, pub h: f32 }

pub struct Embedding(Vec<f32>);                    // newtype with dimension checking

pub enum SearchMode { Hybrid, Bm25Only, VectorOnly, Filter }

pub struct Filters {
    pub subject:        Option<SubjectId>,
    pub topics:         Vec<TagValue>,              // OR within a field, AND across fields
    pub question_types: Vec<TagValue>,
    pub paper_types:    Vec<TagValue>,
    pub schools:        Vec<TagValue>,
    pub source_types:   Vec<TagValue>,
    pub exam_systems:   Vec<TagValue>,
    pub marks_range:    Option<(u32, u32)>,
    pub kind:           Option<DocumentKind>,
}

pub struct SearchQuery {
    pub text:    Option<String>,
    pub filters: Filters,
    pub limit:   usize,                             // default 20, capped at 100
    pub mode:    SearchMode,                        // default Hybrid
    pub exact:   bool,                              // only honored when mode=Bm25Only
    pub rerank:  bool,                              // default true when mode=Hybrid
}

pub struct SearchHit {
    pub document:   Document,
    pub score:      f32,                            // fused final score
    pub scores:     ScoreBreakdown,                 // per-stage contributions
    pub highlights: Vec<Highlight>,
}

pub struct ScoreBreakdown {
    pub bm25:   Option<f32>,
    pub vector: Option<f32>,
    pub rerank: Option<f32>,
}

pub struct Highlight {
    pub field: HighlightField,                      // Question | Answer | Context | Notes
    pub text:  String,                              // snippet ~120 chars with <em>...</em>
}

pub struct SearchResponse {
    pub hits: Vec<SearchHit>,
    pub meta: SearchMeta,
}

pub struct SearchMeta {
    pub mode:           SearchMode,
    pub used_embedder:  bool,
    pub used_bm25:      bool,
    pub used_vector:    bool,
    pub used_reranker:  bool,
    pub fts5_query:     Option<String>,             // what was sent to FTS5
    pub total_matches:  Option<u64>,                // set only on Filter mode
    pub took_ms:        u64,
    pub took_breakdown: TimingBreakdown,
}

pub struct TimingBreakdown {
    pub embed_ms:   Option<u64>,
    pub bm25_ms:    Option<u64>,
    pub vector_ms:  Option<u64>,
    pub fuse_ms:    Option<u64>,
    pub rerank_ms:  Option<u64>,
    pub hydrate_ms: Option<u64>,
}
```

### 5.2 Ports (traits)

All trait methods are `async`. We use `#[async_trait]` because trait objects (`Arc<dyn Embedder>`) are required.

```rust
#[async_trait]
pub trait ItemStore: Send + Sync {
    async fn put(&self, docs: &[Document]) -> Result<()>;
    async fn get(&self, id: &DocumentId) -> Result<Option<Document>>;
    async fn get_many(&self, ids: &[DocumentId]) -> Result<Vec<Document>>;
    async fn query(&self, f: &Filters, limit: usize, offset: usize)
                   -> Result<Vec<Document>>;
    async fn count(&self, f: &Filters) -> Result<u64>;
    async fn list_subjects(&self) -> Result<Vec<SubjectId>>;
    async fn list_topics(&self, subject: &SubjectId) -> Result<Vec<TagValue>>;
    async fn facet_counts(&self, subject: &SubjectId, field: TagField,
                          filters: &Filters) -> Result<Vec<(TagValue, u64)>>;
    async fn clear(&self, subject: &SubjectId) -> Result<()>;
}

#[async_trait]
pub trait VectorStore: Send + Sync {
    async fn upsert(&self, items: &[(DocumentId, Embedding)]) -> Result<()>;
    async fn search(&self, q: &Embedding, f: &Filters, k: usize)
                    -> Result<Vec<(DocumentId, f32)>>;
    async fn clear(&self, subject: &SubjectId) -> Result<()>;
    fn dimension(&self) -> usize;                  // schema-asserted at startup
}

#[async_trait]
pub trait FtsIndex: Send + Sync {
    async fn upsert(&self, items: &[(DocumentId, String)]) -> Result<()>;
    async fn search(&self, query: &str, f: &Filters, k: usize)
                    -> Result<Vec<(DocumentId, f32)>>;
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
    async fn rerank(&self, query: &str, candidates: &[(DocumentId, String)])
                    -> Result<Vec<(DocumentId, f32)>>;
}

#[async_trait]
pub trait BlobStore: Send + Sync {
    async fn get(&self, path: &Path) -> Result<Bytes>;
    async fn exists(&self, path: &Path) -> Result<bool>;
    async fn list(&self, prefix: &Path) -> Result<Vec<PathBuf>>;
}
```

### 5.3 Error model

```rust
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("not found: {what}")]
    NotFound { what: String },

    #[error("bad input: {message}")]
    BadInput { message: String, field: Option<String> },

    #[error("conflict: {message}")]
    Conflict { message: String },

    #[error("storage error: {source}")]
    Storage { #[source] source: Box<dyn std::error::Error + Send + Sync> },

    #[error("embedding model unavailable: {message}")]
    Embedding { message: String },

    #[error("rerank model unavailable: {message}")]
    Reranking { message: String },

    #[error("pdf failure: {message}")]
    Pdf { message: String, path: Option<PathBuf> },

    #[error("schema mismatch: {message}")]
    SchemaMismatch { message: String },

    #[error("ingest aborted due to drift: {skipped}/{total} rows skipped")]
    SchemaDrift { skipped: u64, total: u64, sample_errors: Vec<String> },

    #[error("internal: {message}")]
    Internal { message: String },
}

pub type Result<T> = std::result::Result<T, Error>;
```

Adapters convert their library-specific errors into `Error` at the trait boundary. The API layer maps `Error` variants to HTTP status codes (see §8.5); the CLI maps them to exit codes + stderr.

## 6. Search pipeline (`rex-search`)

### 6.1 Service layer

`rex-search` exposes a `SearchService` that both `rex-api` and `rex-cli` call. This is the single source of truth for "what rex can do". No retrieval logic lives in the API or CLI layers.

```rust
pub struct SearchService {
    items:     Arc<dyn ItemStore>,
    vectors:   Arc<dyn VectorStore>,
    fts:       Arc<dyn FtsIndex>,
    embedder:  Arc<dyn Embedder>,
    reranker:  Option<Arc<dyn Reranker>>,
    config:    SearchConfig,
}

pub struct SearchConfig {
    pub rrf_k:           u32,        // default 60
    pub retrieve_k:      usize,      // top-K from BM25 & vector, default 50
    pub rerank_top_n:    usize,      // top-N for cross-encoder, default 20
    pub max_query_text:  usize,      // default 1024 chars
}

impl SearchService {
    pub async fn search(&self, q: SearchQuery)            -> Result<SearchResponse>;
    pub async fn filter(&self, f: Filters, lim: usize,
                        offset: usize)                    -> Result<SearchResponse>;
    pub async fn get(&self, id: &DocumentId)              -> Result<Document>;
    pub async fn get_many(&self, ids: &[DocumentId])      -> Result<Vec<Document>>;
    pub async fn list_subjects(&self)                     -> Result<Vec<SubjectStats>>;
    pub async fn list_subject(&self, id: &SubjectId)      -> Result<SubjectStats>;
    pub async fn facet_counts(&self, subject: &SubjectId,
                              field: TagField,
                              filters: Filters)           -> Result<FacetCounts>;
    pub async fn pdf_anchor(&self, id: &DocumentId)       -> Result<PdfAnchor>;
}
```

### 6.2 The pipeline

```
        ┌──────────────────────────────────────────────────────┐
        │  SearchService::search(query: SearchQuery)           │
        └──────────────────────────────────────────────────────┘
                                  │
   ┌──────────────────────────────┼──────────────────────────────┐
   │                              │                              │
   ▼ (only if query.text some)    ▼ (always)                     ▼
┌──────────┐                  ┌──────────┐                  filters pushed
│ Embedder │                  │ Filters  │                  ↓ to each store
│ embed_   │                  │ validate │
│ query()  │                  │          │
└────┬─────┘                  └────┬─────┘
     │                             │
     ▼                             ▼
┌────────────┐                ┌────────────┐
│VectorStore │                │ FtsIndex   │   (parallel via tokio::join!)
│ .search()  │                │ .search()  │
│ top-K=50   │                │ top-K=50   │
└─────┬──────┘                └─────┬──────┘
      │                             │
      └──────────────┬──────────────┘
                     ▼
              ┌────────────┐
              │   Fuse     │   Reciprocal Rank Fusion:
              │   (RRF)    │   score(d) = Σ 1 / (k + rank_i(d))
              │ top-N=20   │   k = 60
              └─────┬──────┘
                    ▼
              ┌──────────────┐
              │ ItemStore    │   Hydrate doc bodies for rerank
              │ .get_many()  │
              └──────┬───────┘
                     ▼
              ┌────────────┐    Only if reranker enabled,
              │  Reranker  │    mode=Hybrid, query.text is Some
              │ .rerank()  │
              └─────┬──────┘
                    ▼
              ┌──────────────┐
              │ Highlights   │   Term-highlight from query
              │ + ScoreBreak │
              └─────┬────────┘
                    ▼
                  Top-limit
              SearchResponse
              (hits + meta)
```

### 6.3 Search modes

The pipeline branches at entry based on `query.mode` and whether `query.text` is `Some`. Each mode skips deterministic stages:

| Mode | Embedder | BM25 | Vector | Rerank | When used |
|---|---|---|---|---|---|
| `Hybrid` (default) | ✅ | ✅ | ✅ | ✅ | General "smart" search |
| `Bm25Only` | ❌ | ✅ | ❌ | ❌ | Pure keyword, exact-term search |
| `VectorOnly` | ✅ | ❌ | ✅ | ❌ | Pure semantic; keywords don't help |
| `Filter` | ❌ | ❌ | ❌ | ❌ | Triggered automatically when `text=None` |

A request with `text=None` short-circuits to the filter-only path (`ItemStore::query`) regardless of `mode`. A request with `text=Some` and `mode=Filter` is treated as `BadInput`. A request with `text=None` and `mode != Filter` is also rejected (clients must pick deliberately).

### 6.4 Fusion: Reciprocal Rank Fusion (RRF)

`score(d) = Σ_i  1 / (k + rank_i(d))` summed over retrievers (BM25 + vector). `k = 60` is the default from Cormack et al. (configurable but rarely needs tuning). RRF operates on **ranks**, not scores, so BM25's TF-IDF scores and cosine similarities do not need normalization. Documents that appear in both retrievers' top-K dominate the fused list.

When a document appears in only one retriever's top-K, its missing rank is treated as `retrieve_k + 1` (i.e., just outside the cutoff), giving it a small but nonzero score from the missing side. This avoids the "two retrievers disagree entirely → empty fused list" edge case.

### 6.5 Reranking

Cross-encoder reranking runs only when:

- `query.mode == Hybrid`
- `query.text.is_some()`
- `self.reranker.is_some()`
- `query.rerank` is `true` (default)

The reranker is given the top-20 (`rerank_top_n`) candidates after fusion, paired with the candidate's `search_text` (constructed identically to ingest, see §7.3). The reranker returns scores in `[0, 1]` per pair; the final ordering is by reranker score descending.

A no-op fake reranker (returns input order unchanged) is used in tests and when `--no-reranker` is set at serve time. This means `SearchService` always has *some* reranker, but the no-op reranker yields the post-fusion ordering.

### 6.6 Highlights

After ranking, for each hit, term-based span extraction:

1. Tokenize the original query: lowercase, strip stopwords, drop tokens < 3 chars.
2. For each field in `[context, question, answer, notes]`, find spans containing any query token.
3. Pick up to 3 snippets per hit, each ~120 chars, centered on matched terms, with `<em>...</em>` wrapping matched tokens.

This is pure CPU, deterministic, ~1 ms per hit. Span-level neural highlighting is a future-mode upgrade.

### 6.7 Filter push-down

All three retrieval ports (`ItemStore::query`, `FtsIndex::search`, `VectorStore::search`) accept the same `Filters` value and apply it at the storage layer. This is non-negotiable: post-retrieval filtering produces silently wrong results when filters are selective. The shared `document_tags` join table (see §10) is the mechanism that lets all three apply filters identically.

## 7. Ingestion pipeline (`rex-ingest` + `rex-pdf`)

### 7.1 CLI entry

```bash
rex ingest \
  --subject h2physics \
  --workspace /path/to/ren-subjects/workspace \
  --docs-root /path/to/ren-subjects/docs \
  --db ./rex.db \
  [--rebuild]                # default: replace this subject's data only
  [--max-skip-pct 5]         # default: 5%
  [--batch-size 256]         # default: 256
```

### 7.2 Phased pipeline

```
Phase 1: Parse JSONL (streaming)
  - Open <workspace>/<subject>/reference/{questions,notes}.jsonl
  - Stream rows via BufRead + serde_json::from_str::<JsonlRow>
  - serde JsonlRow has #[serde(deny_unknown_fields)] → drift surfaces immediately
  - Malformed rows: log WARN with error + row excerpt; increment skipped counter
  - After file consumed: if (skipped / total) > max_skip_pct → abort with SchemaDrift
  - Map JsonlRow → IngestRecord { doc: Document (no anchor), search_text: String }

Phase 2: Resolve PDF anchors + Embed (batched, 256 at a time)
  - For each record: derive PDF path from doc.source (markdown → PDF mapping rule)
  - Read PDF bytes via BlobStore
  - Extract per-page text via rex-pdf::extract_pages (pdfium-render)
  - Fuzzy-match doc.context + doc.question against each page; pick best with confidence
  - If confidence >= 0.6: pdf_anchor = Some(PdfAnchor { page_number: Some, .. })
  - If confidence <  0.6: pdf_anchor = Some(PdfAnchor { page_number: None,  .. })
  - If PDF read/extract fails: log WARN, increment pdfs_failed; pdf_anchor = Some(file-level)
  - Embedder::embed_documents(&search_texts) batches all 256 search_texts in one GGUF call

Phase 3: Commit (single transaction per batch)
  - Open SQLite transaction
  - On first batch for a subject (or --rebuild): clear subject from items/fts/vec
  - ItemStore::put(&docs)
  - FtsIndex::upsert(&[(id, search_text)])
  - VectorStore::upsert(&[(id, embedding)])
  - INSERT into ingest_log
  - COMMIT
```

### 7.3 Search text construction

```rust
fn build_search_text(doc: &Document) -> String {
    let mut s = String::new();
    if let Some(c) = &doc.context  { s.push_str("Context: ");  s.push_str(c); s.push('\n'); }
    if let Some(q) = &doc.question { s.push_str("Question: "); s.push_str(q); s.push('\n'); }
    if let Some(a) = &doc.answer   { s.push_str("Answer: ");   s.push_str(a); s.push('\n'); }
    if let Some(n) = &doc.notes    { s.push_str("Notes: ");    s.push_str(n); s.push('\n'); }
    if !doc.keywords.is_empty() {
        s.push_str("Keywords: ");
        s.push_str(&doc.keywords.join(", "));
    }
    s
}
```

The "Context: / Question: / Answer:" prefixes are soft signals to the embedder about field semantics. Used identically at query time when reranking candidates (the reranker is given the same `search_text` representation).

### 7.4 PDF anchor strategy (Strategy C with fallback)

1. **Path mapping.** `doc.source` is a markdown path like `content/prelims/2019/HCI/2019 HCI Prelim H2 Physics 9749 P2.md`. The corresponding PDF path is derived by replacing the `.md` extension with `.pdf` and rebasing from `<workspace>/<subject>/content/...` to `<docs-root>/<subject>/...`. This mapping rule is hardcoded in `rex-ingest` and tested.

2. **PDF page extraction.** `rex-pdf::extract_pages(bytes: Bytes) -> Vec<(u32, String)>` returns `(page_number, page_text)` pairs using `pdfium-render`. Pages with extraction failures are skipped (logged); the rest proceed.

3. **Fuzzy match.** For each `Document`, build a `target = doc.context.unwrap_or("") + " " + doc.question.unwrap_or("")`, truncated to 500 chars. Compute a similarity score against each page's text using:
   - Normalize both to lowercase, strip non-alphanumeric.
   - 3-gram set overlap (Jaccard) — fast, robust to OCR-style minor differences.
4. **Confidence threshold.** Pick the highest-scoring page; if its score >= 0.6, anchor with `page_number = Some(n), confidence = score`. If below, anchor with `page_number = None, confidence = score` (file-level fallback).

5. **PDF read failure.** If `BlobStore::get` or `extract_pages` errors out: anchor with `pdf_path` only, `page_number = None`, `confidence = 0.0`. The `pdfs_failed` counter increments. Ingest does not abort.

### 7.5 Failure modes & error handling

| Failure | Behavior |
|---|---|
| Single malformed row (JSON parse fail) | Log WARN, skip row, increment `rows_skipped` |
| Skip rate exceeds `max_skip_pct` | Abort with `SchemaDrift` error showing first 10 errors |
| PDF read fails for one file | Log WARN, fallback anchor, increment `pdfs_failed`; ingest continues |
| Embedder fails (model not loaded / OOM) | Fatal; ingest aborts before commit; no partial state |
| Transaction commit fails (disk full, locked) | SQLite rolls back; ingest exits non-zero; safe to re-run |
| Schema version mismatch in DB | Abort at startup with `SchemaMismatch` and remediation instructions |

### 7.6 Stdout output

```
$ rex ingest --subject h2physics --workspace ... --docs-root ...
[1/3] Parsing JSONL                  7,446 questions, 312 notes, 8 rows skipped
[2/3] Resolving PDF anchors          47 PDFs, 7,423 anchored, 23 file-level fallback
[2/3] Embedding (batch 29/29)        ████████████████████ 7,758/7,758  18.3s
[3/3] Committing transaction         done

✔ Ingested 7,758 documents for subject 'h2physics' in 28.4s
  Index size: 142 MB (items 12 MB, fts 38 MB, vectors 89 MB, anchors 3 MB)
```

### 7.7 Contract testing

Two layers of defense against rob-side schema drift:

1. **`#[serde(deny_unknown_fields)]` on `JsonlRow`.** Any field rob adds that `rex` does not declare causes immediate parse failure on that row. Catches additive drift (the silent-loss failure mode).

2. **Drift threshold abort.** If `(rows_skipped / rows_total) > max_skip_pct` (default 5%), ingest aborts with `SchemaDrift`, showing up to 10 sample errors. Catches systemic shape changes.

These two together cover: field added (deny_unknown), field removed (parse fail → threshold), field type changed (parse fail → threshold), field renamed (parse fail → threshold).

Fixture-based contract tests and a shared JSON Schema are explicitly **deferred to a future revision**; both add maintenance burden disproportionate to their additional safety on top of the two layers above.

## 8. HTTP API (`rex-api`)

REST + JSON over `axum` on tokio. No auth in v1. Versioned under `/v1`.

### 8.1 Endpoints

```
─── Discovery / metadata ────────────────────────────────────────────────
GET   /v1/health                                       → 200 OK
GET   /v1/subjects                                     → list subjects
GET   /v1/subjects/:id                                 → subject + counts
POST  /v1/subjects/:id/tag-values/:field               → facet values (filter-aware)

─── Search ──────────────────────────────────────────────────────────────
POST  /v1/search                                       → text + filter search
POST  /v1/filter                                       → filter-only browse

─── Document access ─────────────────────────────────────────────────────
GET   /v1/documents/:id                                → single Document
POST  /v1/documents/batch                              → up to 100 by id
GET   /v1/documents/:id/pdf-anchor                     → just the PdfAnchor
GET   /v1/documents/:id/pdf                            → raw PDF bytes
GET   /v1/documents/:id/pdf/page/:n                    → single page as PDF

─── Ops ─────────────────────────────────────────────────────────────────
GET   /v1/metrics                                      → Prometheus text
```

### 8.2 `POST /v1/search`

Request:
```json
{
  "text": "centripetal force tension",
  "mode": "Hybrid",
  "filters": {
    "subject": "h2physics",
    "topics": ["circular-motion"],
    "paper_types": ["paper-2"],
    "schools": ["hci"],
    "marks_range": [1, 5],
    "kind": "Question"
  },
  "limit": 20,
  "exact": false,
  "rerank": true
}
```

Response:
```json
{
  "hits": [
    {
      "document": { "id": "...", "subject": "h2physics", "...": "..." },
      "score": 0.873,
      "scores": { "bm25": 12.4, "vector": 0.81, "rerank": 0.87 },
      "highlights": [
        { "field": "question", "text": "...<em>tension</em> in the <em>cable</em>..." }
      ]
    }
  ],
  "meta": {
    "mode": "Hybrid",
    "used_embedder": true,
    "used_bm25": true,
    "used_vector": true,
    "used_reranker": true,
    "fts5_query": "centripetal force tension",
    "took_ms": 842,
    "took_breakdown": {
      "embed_ms": 28, "bm25_ms": 12, "vector_ms": 18,
      "fuse_ms": 2, "rerank_ms": 760, "hydrate_ms": 22
    }
  }
}
```

Validation:
- `text` required.
- `mode` defaults to `Hybrid` if omitted.
- `mode == Filter` with `text` set → `BadInput`.
- `text` set with `mode == Filter` → `BadInput`.
- `limit` clamped to `[1, 100]`.
- `filters.subject` recommended but not required; absence means cross-subject search.

### 8.3 `POST /v1/filter`

Request:
```json
{
  "filters": { "subject": "h2physics", "schools": ["hci", "njc"], "paper_types": ["paper-2"] },
  "limit": 50,
  "offset": 0,
  "order_by": "default"
}
```

`order_by` values in v1: `"default"` (insertion order), `"marks_desc"`, `"marks_asc"`. Pagination via `offset` is supported only on this path (search results are score-ordered; offset doesn't make sense there).

Response is a `SearchResponse` with `meta.mode = Filter`, `used_*` all false, `total_matches = Some(n)`, hits each have `scores` all `None`.

### 8.4 `POST /v1/subjects/:id/tag-values/:field`

Returns facet counts within a filtered subset. The most important endpoint for a filter-sidebar UX.

```http
POST /v1/subjects/h2physics/tag-values/topics
{ "filters": { "schools": ["hci"], "paper_types": ["paper-2"] } }
```
```json
{
  "subject": "h2physics",
  "field":   "topics",
  "values":  [
    { "value": "dynamics",        "count": 47 },
    { "value": "circular-motion", "count": 31 },
    { "value": "thermal-physics", "count": 22 }
  ]
}
```

Valid `field` values: `topics`, `question_types`, `exam_systems`, `paper_types`, `schools`, `source_types`. Unknown field → `BadInput`.

### 8.5 Error model

All errors return:
```json
{ "error": { "code": "...", "message": "...", "details": { } } }
```

Mapping:

| `Error` variant | HTTP status | `error.code` |
|---|---|---|
| `NotFound` | 404 | `not_found` |
| `BadInput` | 400 | `bad_input` |
| `Conflict` | 409 | `conflict` |
| `Embedding` | 503 | `embedder_unavailable` |
| `Reranking` | 503 | `reranker_unavailable` |
| `Storage` | 500 | `internal` |
| `Pdf` | 500 | `pdf_failure` |
| `SchemaMismatch` | 503 | `schema_mismatch` |
| `Internal` / other | 500 | `internal` |

`internal` responses include `details.request_id` cross-referenceable to server logs.

### 8.6 Operational concerns

- **CORS.** Allow `*` in dev (env-configurable); explicit allowlist in prod.
- **Request body limit.** 1 MB cap (axum `RequestBodyLimitLayer`).
- **Timeouts.** 30 s per request (tower `TimeoutLayer`).
- **Rate limiting.** Out of scope for v1 (single-tenant).
- **TLS.** Out of scope; nginx in front of `rex serve` handles TLS in prod.

## 9. CLI (`rex-cli`)

### 9.1 Parity invariant

Every API capability has a matching CLI subcommand. A workspace-level integration test enumerates both surfaces and asserts equivalence:

```rust
#[test]
fn cli_api_parity() {
    let api:   HashSet<_> = rex_api::declared_routes()   .into_iter().map(canon).collect();
    let cli:   HashSet<_> = rex_cli::declared_commands().into_iter().map(canon).collect();
    assert_eq!(api, cli, "every API route must have a matching CLI command");
}
```

### 9.2 Subcommands

| HTTP route | CLI subcommand |
|---|---|
| `POST /v1/search` | `rex search <query> [--subject ...] [--topic ...] [--paper-type ...] [--mode hybrid|bm25|vector] [--exact] [--no-rerank] [--limit N]` |
| `POST /v1/filter` | `rex filter [--subject ...] [--topic ...] [--paper-type ...] [--limit N] [--offset N] [--order-by default|marks_desc|marks_asc]` |
| `GET /v1/documents/:id` | `rex get <id>` |
| `POST /v1/documents/batch` | `rex get-many <id> <id> <id> ...` |
| `GET /v1/documents/:id/pdf-anchor` | `rex pdf-anchor <id>` |
| `GET /v1/documents/:id/pdf` | `rex pdf <id> [-o out.pdf]` |
| `GET /v1/documents/:id/pdf/page/:n` | `rex pdf <id> --page N [-o page.pdf]` |
| `GET /v1/subjects` | `rex subjects` |
| `GET /v1/subjects/:id` | `rex subject <id>` |
| `POST /v1/subjects/:id/tag-values/:field` | `rex tag-values --subject <id> --field <field> [--filter ...]` |

Plus the two write paths that are CLI-only:

```
rex ingest --subject ... --workspace ... --docs-root ... [--rebuild] [--max-skip-pct 5]
rex serve  --db ... --docs-root ... --models-dir ... --bind ... [--cors-allow ...] [--no-reranker] [--warm]
```

### 9.3 Output

Each subcommand emits a human-readable pretty default and a `--json` flag that emits the same JSON the API would return. Scripting against the CLI is structurally identical to scripting against the API.

### 9.4 Execution mode

`rex search`, `rex filter`, `rex get*`, etc. run **in-process**: open `rex.db` directly, instantiate adapters, invoke `SearchService`. The embedder is **lazily loaded** — only constructed when the subcommand actually needs to embed (e.g., `rex search` in `Hybrid` or `VectorOnly` mode). Filter-only subcommands never load the model.

A `--remote http://...` flag is stubbed in clap but returns `"not implemented"` in v1, reserving the namespace for a future client-mode where the CLI talks to a deployed `rex serve`.

## 10. Storage schema (`rex-sqlite`)

Single SQLite file (`rex.db`) holds **three logical stores** backed by **one schema**. WAL mode for concurrent reads.

### 10.1 Tables

```sql
CREATE TABLE subjects (
    id         TEXT PRIMARY KEY,
    created_at INTEGER NOT NULL,
    item_count INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE documents (
    id            TEXT PRIMARY KEY,
    subject_id    TEXT NOT NULL REFERENCES subjects(id) ON DELETE CASCADE,
    kind          TEXT NOT NULL CHECK (kind IN ('Question', 'Note')),
    parent_id     TEXT REFERENCES documents(id),
    number        TEXT,
    source_path   TEXT NOT NULL,
    context       TEXT,
    question      TEXT,
    answer        TEXT,
    notes         TEXT,
    mark          INTEGER,
    options_json  TEXT,
    keywords_json TEXT NOT NULL DEFAULT '[]',
    pdf_path      TEXT,
    pdf_page      INTEGER,
    pdf_bbox_json TEXT,
    pdf_confidence REAL,
    created_at    INTEGER NOT NULL
);
CREATE INDEX idx_documents_subject ON documents(subject_id);
CREATE INDEX idx_documents_kind    ON documents(subject_id, kind);
CREATE INDEX idx_documents_parent  ON documents(parent_id);

CREATE TABLE document_tags (
    document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    field       TEXT NOT NULL,
    value       TEXT NOT NULL,
    PRIMARY KEY (document_id, field, value)
);
CREATE INDEX idx_tags_field_value ON document_tags(field, value);
CREATE INDEX idx_tags_doc_field   ON document_tags(document_id, field);

CREATE VIRTUAL TABLE document_fts USING fts5(
    document_id UNINDEXED,
    search_text,
    content='',
    tokenize='porter unicode61 remove_diacritics 1'
);

CREATE VIRTUAL TABLE document_vec USING vec0(
    document_id TEXT PRIMARY KEY,
    embedding   FLOAT[768]
);

CREATE TABLE ingest_log (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    subject_id  TEXT NOT NULL,
    started_at  INTEGER NOT NULL,
    finished_at INTEGER,
    items_added INTEGER NOT NULL DEFAULT 0,
    items_total INTEGER NOT NULL DEFAULT 0,
    pdfs_seen   INTEGER NOT NULL DEFAULT 0,
    pdfs_failed INTEGER NOT NULL DEFAULT 0,
    status      TEXT NOT NULL CHECK (status IN ('running','ok','failed')),
    error       TEXT
);

CREATE TABLE schema_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
INSERT INTO schema_meta (key, value) VALUES ('version', '1');
INSERT INTO schema_meta (key, value) VALUES ('embedder', 'embeddinggemma-300M-Q8_0');
INSERT INTO schema_meta (key, value) VALUES ('vector_dim', '768');
```

### 10.2 PRAGMAs (applied at first open)

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA temp_store = MEMORY;
PRAGMA mmap_size = 268435456;     -- 256 MB
PRAGMA foreign_keys = ON;
```

### 10.3 Query patterns

Hybrid search query path:

```sql
-- Step 1: resolve filtered ID set as a CTE
WITH filtered AS (
  SELECT id FROM documents
  WHERE subject_id = ?
    AND id IN (SELECT document_id FROM document_tags WHERE field='topics' AND value='dynamics')
)
-- Step 2a: BM25 over filtered set
SELECT document_id, bm25(document_fts) AS score
FROM document_fts
WHERE document_fts MATCH ? AND document_id IN filtered
ORDER BY score LIMIT 50;

-- Step 2b: vector over the same filtered set (sqlite-vec supports WHERE id IN constraints)
SELECT document_id, distance
FROM document_vec
WHERE embedding MATCH ? AND k = 50 AND document_id IN filtered;
```

Facet counts:

```sql
SELECT value, COUNT(*) FROM document_tags
WHERE field = 'topics'
  AND document_id IN (... filtered set ...)
GROUP BY value
ORDER BY COUNT(*) DESC;
```

### 10.4 Dimension safety

`document_vec.embedding` is typed `FLOAT[768]` (embeddinggemma-300M dimension). At server startup, `rex-cli` calls `embedder.dimension()` and compares to `schema_meta.vector_dim`. If they disagree, the server refuses to start with:

> *"vector table is dimension 768 but configured embedder produces dimension N — re-ingest required after schema migration"*

A future model swap requires a schema migration (recreate `document_vec` with the new dimension) and a full re-ingest. `schema_meta.version` is bumped accordingly.

### 10.5 Size estimates

For h2physics (~7,800 items):

| Table | Approx size |
|---|---|
| `documents` | 12 MB |
| `document_tags` | 1 MB |
| `document_fts` | 38 MB |
| `document_vec` | 89 MB |
| `ingest_log`, `subjects`, `schema_meta` | < 1 MB |
| **Total** | **~140 MB per subject** |

8 subjects: ~1.1 GB total.

## 11. Configuration & ops

### 11.1 Config sources

`rex` reads configuration in this precedence order:

1. CLI flags (highest)
2. Environment variables: `REX_DB`, `REX_DOCS_ROOT`, `REX_MODELS_DIR`, `REX_BIND`, `REX_CORS_ALLOW`, `REX_LOG_LEVEL`, `REX_LOG_FORMAT`
3. Defaults (lowest)

No `.env` file scanning by default. Production deployments set env vars via systemd.

### 11.2 Logging

`tracing` + `tracing-subscriber`. JSON format in prod (`REX_LOG_FORMAT=json`), pretty in dev. Per-request spans carry `request_id`, `route`, `status`, `latency_ms`, and (for `/search`) `mode`, `subject`, `hits_returned`, `text_chars`.

### 11.3 Metrics

`metrics` crate + `metrics-exporter-prometheus`. Exposed at `GET /v1/metrics` in Prometheus text format. The set is:

```
# Request-level
rex_requests_total{route, status, method}              counter
rex_request_duration_ms{route}                          histogram

# Search-stage
rex_search_stage_duration_ms{stage}                     histogram   # stage = embed|bm25|vector|fuse|rerank|hydrate
rex_search_hits{mode}                                   histogram

# Ingest
rex_ingest_runs_total{subject, status}                  counter
rex_rows_skipped_total{subject, reason}                 counter
rex_pdfs_failed_total{subject}                          counter

# Process memory (sampled every 10s by background task)
rex_process_rss_bytes                                   gauge
rex_process_vsize_bytes                                 gauge
rex_process_threads                                     gauge
rex_process_open_fds                                    gauge

# Component memory (sampled every 30s)
rex_component_memory_bytes{component}                   gauge       # sqlite_cache|embedder_weights|reranker_weights|lru_pdf_pages
```

Process metrics are gathered via the `sysinfo` crate. Component metrics:

- `sqlite_cache`: derived from `PRAGMA cache_size` × current page utilization.
- `embedder_weights`: constant after model load (model file size).
- `reranker_weights`: constant after model load.
- `lru_pdf_pages`: `len * average_entry_size` from the page-slicing LRU.

### 11.4 Deployment shape

```
EC2 instance (single host, t3.medium or t3.large)
├── systemd: rex.service
│   ExecStart=/opt/rex/bin/rex serve
│     --db /var/lib/rex/rex.db
│     --docs-root /var/lib/rex/docs
│     --models-dir /var/lib/rex/models
│     --bind 0.0.0.0:8080
│     --warm
│   User=rex
│   Restart=on-failure
├── /var/lib/rex/
│   ├── rex.db              (scp'd from dev box after ingest)
│   ├── docs/<subject>/...  (scp'd PDFs)
│   └── models/...          (scp'd GGUF weights, one-time)
└── nginx (TLS termination, HTTP/2 — out of scope for v1 but planned)
```

Ingest runs on the dev box (faster CPU, easier to iterate). The resulting `rex.db` is `scp`'d to EC2. EC2 needs the embedding model loaded at serve time; the reranker model can be skipped with `--no-reranker` if RAM is tight.

### 11.5 Backups

`rex.db` is a single file. `sqlite3 .backup` produces a consistent snapshot suitable for cron upload to S3. Restore = `scp` back into place.

## 12. Testing strategy

### 12.1 Testing pyramid

```
                                ┌───────────────┐
                                │  E2E (axum    │   ~3-5 tests
                                │  test client) │   Real SQLite, fake embedder
                                └───────────────┘
                              ┌─────────────────────┐
                              │  Integration tests  │   ~15-25 tests
                              │  (per-crate, real   │   Real adapters where fast
                              │   adapters)         │   (SQLite); fakes elsewhere
                              └─────────────────────┘
                          ┌───────────────────────────────┐
                          │     Unit tests (per crate)    │   ~80-120 tests
                          │     Pure logic, all fakes     │   Microsecond runtimes
                          └───────────────────────────────┘
```

### 12.2 Per-crate test plans

**`rex-domain`** — type-level unit tests. `Filters` validation, `Embedding` dimension checks, error variant mapping. ~10 tests.

**`rex-search`** — the crown jewel; runs entirely on fake adapters in microseconds. Covers:
- RRF fusion produces expected ranking from canned BM25 + vector results
- Filter push-down: a given `Filters` constrains all three retrievers identically
- Mode dispatch: `Bm25Only` skips embedder/vector/rerank, etc.
- Reranker permutes scores; missing reranker preserves fused order
- Highlight extraction over canned text
- `ScoreBreakdown` and `SearchMeta` populated correctly per mode
- Degenerate inputs: zero hits, all-tied scores, very short queries

**`rex-ingest`** — covers:
- `deny_unknown_fields` rejects rows with unexpected keys (negative test)
- Drift threshold: 6% skip → abort; 4% skip → proceed
- Search text construction has correct prefixes
- Parent/child preserved
- Tag normalization (case, whitespace)
- Mapping rule: markdown source → PDF path

**`rex-sqlite`** — integration tests with `:memory:` SQLite (~10ms each):
- Schema creates idempotently
- FK cascade on subject delete
- Filter SQL produces correct ID sets across compound filters
- FTS5 BM25 rankings on canned text
- sqlite-vec roundtrip; KNN respects `WHERE id IN (...)`
- `Embedder::dimension()` mismatch refuses startup

**`rex-llamacpp`** — behind `#[cfg(feature = "llama-tests")]`, requires model weights, runs in a nightly CI lane. Tests: dimension matches expected, query vs document prefixes produce distinct vectors, reranker scores correlate with relevance on canned pairs.

**`rex-pdf`** — real small PDFs in `tests/fixtures/`. Tests: page text extraction, fuzzy anchor returns correct page for known question, returns `None` below threshold.

**`rex-fs-local`** — tempdir; write+read+exists+list trivial.

**`rex-api`** — integration via `axum::Router::oneshot()`. Asserts request/response shapes for every endpoint with fake adapters.

**`rex-cli`** — `insta` snapshot tests for canned argv → stdout. Catches output regressions.

**Workspace E2E** — `tests/e2e.rs`. Boots real `rex serve` on a temp DB, ingests a 2-question / 1-PDF fixture, exercises each endpoint. ~5 s. Runs in default CI.

### 12.3 Property tests (proptest)

- **Filter equivalence:** any random `Filters` produces the same ID set across `ItemStore`, `FtsIndex` pre-filter, and `VectorStore` pre-filter.
- **RRF stability:** same inputs → same ranking; small perturbations → small ranking deltas.
- **Document round-trip:** any `Document` → SQLite → read-back == original.

### 12.4 CI parity gate

A single test (`cli_api_parity`) enumerates the API route declarations and CLI command declarations and asserts they cover the same logical capabilities. Adding an API route without a matching CLI subcommand (or vice versa) fails CI.

## 13. Repository layout

```
ren-rex/
├── Cargo.toml                 ← [workspace]
├── README.md
├── crates/
│   ├── rex-domain/
│   ├── rex-search/
│   ├── rex-ingest/
│   ├── rex-pdf/
│   ├── rex-sqlite/
│   ├── rex-llamacpp/
│   ├── rex-fs-local/
│   ├── rex-api/
│   └── rex-cli/
├── tests/
│   └── e2e.rs                 ← workspace-level
├── docs/
│   └── superpowers/specs/
│       └── 2026-05-15-rex-pdf-search-server-design.md
└── scripts/
    ├── dev-ingest.sh
    └── fetch-models.sh
```

## 14. Future work (explicitly out of scope for v1)

- **Incremental ingest.** Track per-file mtime/hash in `ingest_log`; only re-process changed files.
- **PDF text indexing as a second corpus.** Strategy C already extracts per-page text; storing it in a sibling `pdf_pages` table + FTS5 + vector index is purely additive. Search modes would accept a `target: questions|pdfs|both` parameter.
- **Authentication.** API key in header → middleware in `rex-api`.
- **Admin HTTP endpoints.** `POST /v1/admin/ingest`, behind auth, runs `rex-ingest` library functions in a background task.
- **Alternative storage backends.** `rex-postgres` (single crate; impls `ItemStore + VectorStore + FtsIndex`), `rex-qdrant` (vector store only).
- **Alternative blob storage.** `rex-fs-s3` (impls `BlobStore`).
- **Alternative embedders.** `rex-openai`, `rex-voyage` impls of `Embedder` and `Reranker`.
- **Profiling endpoint.** `/debug/pprof/{profile,heap}` behind `--enable-profiling`.
- **Tokio runtime introspection.** `tokio-console` behind a Cargo feature.
- **Shared JSON Schema with rob-the-crawler.** When a second engineer maintains rob, promote contract testing from `deny_unknown_fields` + drift threshold to a shared schema document.
- **Client SDKs.** TypeScript + Python clients generated from an OpenAPI spec.
- **gRPC alongside REST.** `rex-grpc` crate alongside `rex-api`, both consuming `rex-search`.

## 15. Glossary

- **bi-encoder.** Embedding model architecture that produces a single vector per text input independently. Fast at query time (one forward pass + index lookup) but less accurate than cross-encoder reranking. `Embedder` impls are bi-encoders.
- **cross-encoder.** Reranking model architecture that reads (query, document) jointly and outputs a relevance score per pair. Slow per pair (~30-50ms) but very accurate. `Reranker` impls are cross-encoders.
- **RRF.** Reciprocal Rank Fusion. A score-free way of combining ranked lists from multiple retrievers. `score(d) = Σ 1/(k + rank_i(d))`.
- **FTS5.** SQLite's full-text-search extension, providing BM25 ranking.
- **sqlite-vec.** SQLite extension providing a `vec0` virtual table type for KNN search over fixed-dimension float vectors. Supports `WHERE rowid IN (...)` constraints.
- **GGUF.** Quantized model file format used by `llama.cpp`. We use Q8_0 (8-bit) quantizations for both embeddinggemma and Qwen3-Reranker.
- **WAL mode.** SQLite Write-Ahead Logging journal mode. Supports concurrent readers + one writer without locking the entire DB.
- **port / adapter.** Hexagonal-architecture terminology. A port is a trait defined in the domain crate; an adapter is a concrete implementation in an adapter crate (e.g., `rex-sqlite` is a SQLite-backed adapter for the `ItemStore`/`VectorStore`/`FtsIndex` ports).
