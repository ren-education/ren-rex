# rex Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the rex PDF search & navigator server end-to-end and validate it against `ren-subjects/workspace/h2history` as the live test bed.

**Architecture:** Rust Cargo workspace, 9 crates, hexagonal (ports & adapters). Two binary modes: `rex ingest` (writes SQLite + sqlite-vec + FTS5) and `rex serve` (HTTP API via axum). CLI subcommands at full parity with API. Hybrid search via Reciprocal Rank Fusion of BM25 + vector retrieval, with optional cross-encoder reranking.

**Tech Stack:** Rust 1.78+, `tokio`, `axum`, `rusqlite` + `sqlite-vec` + FTS5, `pdfium-render`, `llama-cpp-2` (best-effort for v1; fallback to a deterministic stub embedder if build is blocked), `clap`, `tracing`, `metrics` + `metrics-exporter-prometheus`, `serde` + `serde_json`, `async-trait`, `thiserror`, `anyhow`, `insta`, `proptest`.

**Spec:** `docs/superpowers/specs/2026-05-15-rex-pdf-search-server-design.md`. Read this first; the plan assumes familiarity with §1-§15 of the spec.

**Test bed:** `/Users/jcjustin/Projects/tippytop/ren-subjects/workspace/h2history/` and `/Users/jcjustin/Projects/tippytop/ren-subjects/docs/h2history/`.

---

## Pragmatic note on granularity

The skill template prefers single-action steps (write test, run test, implement, run test, commit). For boilerplate plumbing (Cargo.toml entries, mechanical adapter wiring) we use higher-level steps to avoid plan bloat. For core logic (RRF fusion, mode dispatch, ingest contract, schema, path mapping) we follow strict TDD with one-action steps. The judgment call per task is: "does this code have non-trivial behavior worth pinning with a test?" If yes, TDD. If no, write and commit.

## Risk register (resolve as we go)

| Risk | Likelihood | Mitigation |
|---|---|---|
| `llama-cpp-2` build requires C++ toolchain not present | Medium | Plan separates embedder integration (Chunk 8). Stub embedder in Chunk 2 keeps the rest of the system testable end-to-end. |
| `pdfium-render` requires pdfium dynamic lib at runtime | Medium | Use bundled-static variant of the crate if available. Fall back to a simpler pdf extractor (`lopdf` or `pdf-extract`) if pdfium proves too painful. |
| `sqlite-vec` integration with `rusqlite` (extension loading) is finicky | Medium | Use `rusqlite`'s `load_extension` API. Bundle the extension via `vec0.dylib`/`.so` for the target arch. Plan has a Chunk 3 verification step. |
| JSONL schema gaps (e.g., `depends_on`, source extension variance across subjects) | High | First ingest run in Chunk 9 is *expected* to fail; the failure tells us what to add to `JsonlRow`. Plan includes an explicit "diagnose + extend" task. |
| `rex.db` write contention if ingest and serve run together | Low | WAL mode enables concurrent reads, but serve must reload on changes. v1 deploys ingest offline; doc this in README. |

---

## Chunk 1: Workspace bootstrap + `rex-domain`

### Task 1: Workspace `Cargo.toml` + scaffolding

**Files:**
- Create: `Cargo.toml` (workspace manifest)
- Create: `rust-toolchain.toml`
- Create: `.gitignore`
- Create: `README.md` (minimal)
- Create: `crates/rex-domain/Cargo.toml`
- Create: `crates/rex-search/Cargo.toml`
- Create: `crates/rex-ingest/Cargo.toml`
- Create: `crates/rex-pdf/Cargo.toml`
- Create: `crates/rex-sqlite/Cargo.toml`
- Create: `crates/rex-llamacpp/Cargo.toml`
- Create: `crates/rex-fs-local/Cargo.toml`
- Create: `crates/rex-api/Cargo.toml`
- Create: `crates/rex-cli/Cargo.toml`
- Create: `crates/*/src/lib.rs` (empty placeholders) for all 9 crates
- Create: `crates/rex-cli/src/main.rs` with `fn main() {}`

- [ ] **Step 1:** Create workspace `Cargo.toml` with all 9 members, shared `[workspace.package]`, shared `[workspace.dependencies]` for common crates (`serde`, `serde_json`, `tokio`, `tracing`, `thiserror`, `async-trait`, `anyhow`, `uuid`, `bytes`).
- [ ] **Step 2:** Create per-crate `Cargo.toml` files. Each crate uses `workspace = true` for shared deps. Adapter crates depend only on `rex-domain` (verified by the file contents).
- [ ] **Step 3:** Create `rust-toolchain.toml` pinning `channel = "1.78"`.
- [ ] **Step 4:** Create `.gitignore`: `/target`, `*.db`, `*.db-shm`, `*.db-wal`, `/models`, `*.gguf`.
- [ ] **Step 5:** Create empty `lib.rs` placeholders for all 9 crates and `main.rs` for `rex-cli`.
- [ ] **Step 6:** Run `cargo build --workspace`. Expected: all 9 crates compile, no warnings.
- [ ] **Step 7:** Commit: `chore: bootstrap workspace with 9 empty crates`.

### Task 2: `rex-domain` — core types

**Files:**
- Modify: `crates/rex-domain/Cargo.toml` (add: `serde`, `serde_json`, `uuid`, `thiserror`, `bytes`, `async-trait`)
- Create: `crates/rex-domain/src/lib.rs`
- Create: `crates/rex-domain/src/ids.rs` (SubjectId, DocumentId, TagValue, SourcePath)
- Create: `crates/rex-domain/src/document.rs` (Document, DocumentKind, Tags, TagField)
- Create: `crates/rex-domain/src/pdf.rs` (PdfAnchor, BoundingBox, FallbackReason)
- Create: `crates/rex-domain/src/search.rs` (SearchMode, SearchQuery, SearchHit, ScoreBreakdown, Highlight, HighlightField, Filters, SearchResponse, SearchMeta, TimingBreakdown)
- Create: `crates/rex-domain/src/embedding.rs` (Embedding newtype with dimension method)
- Create: `crates/rex-domain/src/error.rs` (Error enum, Result alias)

- [ ] **Step 1:** Implement every type per spec §5.1. Use `#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]` where appropriate. Newtype IDs wrap `String` / `Uuid`. Document gains `depends_on: Vec<DocumentId>` (discovered in pre-flight check). `Filters` includes Default impl that yields the "match-everything" filter.
- [ ] **Step 2:** Implement `Embedding(Vec<f32>)` with `new(dim_expected: usize, v: Vec<f32>) -> Result<Self>` that errors on dim mismatch. `dimension() -> usize`. `as_slice() -> &[f32]`.
- [ ] **Step 3:** Implement `Error` enum per spec §5.3 with `#[derive(thiserror::Error, Debug)]`.
- [ ] **Step 4:** Re-export the full public surface from `lib.rs`.
- [ ] **Step 5:** Run `cargo build -p rex-domain`. Expected: clean compile.
- [ ] **Step 6:** Commit: `feat(domain): add core types and error model`.

### Task 3: `rex-domain` — ports (traits)

**Files:**
- Create: `crates/rex-domain/src/ports.rs` (ItemStore, VectorStore, FtsIndex, Embedder, Reranker, BlobStore)

- [ ] **Step 1:** Define all six traits per spec §5.2 using `#[async_trait::async_trait]`. Every method returns `crate::Result<T>`. All traits are `Send + Sync`.
- [ ] **Step 2:** Re-export from `lib.rs`.
- [ ] **Step 3:** `cargo build -p rex-domain`. Expected: clean compile.
- [ ] **Step 4:** Commit: `feat(domain): add port traits`.

### Task 4: `rex-domain` — unit tests

**Files:**
- Create: `crates/rex-domain/tests/types.rs`

- [ ] **Step 1:** Write tests:
  - `embedding_dimension_mismatch_errors`: `Embedding::new(768, vec![0.0; 512])` returns `Err`.
  - `filters_default_is_match_everything`: `Filters::default().is_match_all() == true`.
  - `document_kind_serialization`: serde roundtrip of `Question` and `Note`.
  - `tag_field_enumeration`: assert the closed list of `TagField` values matches the 6 fields in spec §5.1.
  - `error_variants_match_spec`: pattern-match every `Error` variant to confirm exhaustiveness.
- [ ] **Step 2:** Run `cargo test -p rex-domain`. Expected: all pass.
- [ ] **Step 3:** Commit: `test(domain): add type-level tests`.

---

## Chunk 2: `rex-search` — pipeline with fakes

### Task 5: Fake adapters (testing utilities)

**Files:**
- Modify: `crates/rex-search/Cargo.toml` (deps: `rex-domain`, `tokio`, `tracing`, `async-trait`, `futures`)
- Create: `crates/rex-search/src/lib.rs`
- Create: `crates/rex-search/src/fakes.rs` (FakeItemStore, FakeVectorStore, FakeFtsIndex, FakeEmbedder, NoOpReranker)

- [ ] **Step 1:** Implement `FakeItemStore` backed by `Mutex<HashMap<DocumentId, Document>>`. Implements `ItemStore` trait. Filters applied via `Document.matches(filters)`.
- [ ] **Step 2:** Implement `FakeVectorStore`: in-memory `Vec<(DocumentId, Vec<f32>)>`. `search` computes cosine similarity, returns top-K filtered by the same filter logic.
- [ ] **Step 3:** Implement `FakeFtsIndex`: in-memory `Vec<(DocumentId, String)>`. `search` does substring matching with a simple TF score.
- [ ] **Step 4:** Implement `FakeEmbedder`: deterministic, hashes input to seed a 768-dim vector. Same input → same vector. Different inputs → different vectors with weak similarity for shared tokens.
- [ ] **Step 5:** Implement `NoOpReranker`: returns input order unchanged.
- [ ] **Step 6:** Gate fakes behind `#[cfg(any(test, feature = "fakes"))]` plus a `fakes` Cargo feature so other crates can import them in their own tests.
- [ ] **Step 7:** `cargo build -p rex-search --features fakes`. Expected: clean compile.
- [ ] **Step 8:** Commit: `feat(search): add fake adapters for testing`.

### Task 6: RRF fusion

**Files:**
- Create: `crates/rex-search/src/fusion.rs`

- [ ] **Step 1: Write failing tests:**
  - `rrf_doc_in_both_lists_dominates`: doc appearing rank 1 in BM25 and rank 1 in vector beats docs in only one list.
  - `rrf_doc_in_only_one_list_still_scored`: doc appearing only in BM25 receives nonzero score.
  - `rrf_constant_k_default_60`: explicit assertion that default k = 60.
  - `rrf_empty_inputs_returns_empty`: both lists empty → empty output.
  - `rrf_ranking_deterministic_for_same_input`: same input → same output two runs.
- [ ] **Step 2:** Run `cargo test -p rex-search fusion`. Expected: fail (functions undefined).
- [ ] **Step 3:** Implement `rrf_fuse(bm25: &[(DocumentId, f32)], vec: &[(DocumentId, f32)], top_n: usize, k: u32) -> Vec<(DocumentId, f32)>` per spec §6.4.
- [ ] **Step 4:** Run tests. Expected: pass.
- [ ] **Step 5:** Commit: `feat(search): implement RRF fusion`.

### Task 7: Highlights

**Files:**
- Create: `crates/rex-search/src/highlights.rs`

- [ ] **Step 1: Write failing tests:**
  - `highlight_finds_query_term_in_question`: query "tension" highlights `<em>tension</em>` in question text.
  - `highlight_skips_stopwords`: query "the cable" does not highlight "the".
  - `highlight_capped_at_three_per_hit`: doc with 10 matches yields ≤3 highlight snippets.
  - `highlight_snippet_length_around_120_chars`: each snippet is ~120 chars centered on the match.
  - `highlight_no_match_returns_empty`: query terms absent in doc → empty highlights.
- [ ] **Step 2:** Run tests. Expected: fail.
- [ ] **Step 3:** Implement `extract_highlights(query: &str, doc: &Document) -> Vec<Highlight>` per spec §6.6. Use a small stopword list (`the`, `a`, `an`, `of`, `to`, `for`, `in`, `is`, `it`).
- [ ] **Step 4:** Run tests. Expected: pass.
- [ ] **Step 5:** Commit: `feat(search): implement term-based highlighting`.

### Task 8: SearchService skeleton + mode dispatch

**Files:**
- Create: `crates/rex-search/src/service.rs` (`SearchService` struct, methods, mode dispatch)
- Create: `crates/rex-search/src/config.rs` (`SearchConfig` with defaults)

- [ ] **Step 1:** Define `SearchService` with `Arc<dyn ItemStore>`, `Arc<dyn VectorStore>`, `Arc<dyn FtsIndex>`, `Arc<dyn Embedder>`, `Option<Arc<dyn Reranker>>`, `SearchConfig`.
- [ ] **Step 2: Write failing tests for validation:**
  - `validate_text_none_with_hybrid_mode_rejects`: `SearchQuery { text: None, mode: Hybrid, .. }` → `Err(BadInput)`.
  - `validate_text_some_with_filter_mode_rejects`: `text: Some, mode: Filter` → `Err(BadInput)`.
  - `validate_limit_clamped_to_100`: `limit: 1000` → result has `limit=100` effectively (asserted via call to underlying).
- [ ] **Step 3:** Run tests. Expected: fail.
- [ ] **Step 4:** Implement validation in `SearchService::search`.
- [ ] **Step 5:** Implement `SearchService::filter` (pure filter path: `ItemStore::query` + count + meta).
- [ ] **Step 6:** Run tests. Expected: pass.
- [ ] **Step 7:** Commit: `feat(search): add SearchService with validation + filter path`.

### Task 9: SearchService hybrid pipeline

**Files:**
- Modify: `crates/rex-search/src/service.rs`
- Create: `crates/rex-search/src/pipeline.rs` (the inner stages, easier to test in isolation)

- [ ] **Step 1: Write failing tests:**
  - `hybrid_mode_calls_all_stages`: spy fakes assert embed, bm25, vector, fuse, rerank all invoked.
  - `bm25_only_mode_skips_embedder_vector_reranker`: spy fakes assert embed/vector/rerank NOT invoked.
  - `vector_only_mode_skips_bm25_reranker`: spy fakes assert bm25/rerank NOT invoked.
  - `no_reranker_skips_hydration_for_rerank`: `SearchService { reranker: None, .. }` records only one `get_many` call (final response hydration, not pre-rerank hydration).
  - `filter_pushdown_consistent`: a filter set restricts both BM25 and vector to the same id-set.
- [ ] **Step 2:** Run tests. Expected: fail.
- [ ] **Step 3:** Implement hybrid pipeline in `pipeline.rs`. Use `tokio::join!` for parallel BM25 + vector. Apply spec §6.2 sequence.
- [ ] **Step 4:** Capture per-stage timings into `TimingBreakdown`. Populate `SearchMeta` with `used_*` flags and `took_breakdown`.
- [ ] **Step 5:** Run tests. Expected: pass.
- [ ] **Step 6:** Commit: `feat(search): implement hybrid search pipeline with mode dispatch`.

### Task 10: Property tests for filter equivalence + RRF stability

**Files:**
- Create: `crates/rex-search/tests/properties.rs`

- [ ] **Step 1:** Add `proptest` to dev-deps.
- [ ] **Step 2:** Write `prop_filter_equivalent_across_stores`: any randomly-generated `Filters` produces the same `HashSet<DocumentId>` from `ItemStore::query`, `FtsIndex::search(*)`, and `VectorStore::search(*)` (using the fakes).
- [ ] **Step 3:** Write `prop_rrf_deterministic`: same fused inputs → same outputs.
- [ ] **Step 4:** Run `cargo test -p rex-search --release`. Expected: pass within 30s.
- [ ] **Step 5:** Commit: `test(search): add property tests for filter and RRF`.

---

## Chunk 3: `rex-sqlite`

### Task 11: Schema + migrations

**Files:**
- Modify: `crates/rex-sqlite/Cargo.toml` (deps: `rex-domain`, `rusqlite` with `bundled` + `serde_json` + `vtab` features, `tokio` with `rt`, `serde_json`, `tracing`)
- Create: `crates/rex-sqlite/src/lib.rs`
- Create: `crates/rex-sqlite/src/schema.rs` (raw SQL migrations as `&'static str` consts)
- Create: `crates/rex-sqlite/src/conn.rs` (opens DB, applies PRAGMAs, loads sqlite-vec extension, runs migrations)

- [ ] **Step 1:** Add `sqlite-vec` as a vendored binary or via a Cargo crate that links the extension statically. Verify: a smoke test loads the extension and creates a `vec0` virtual table.
- [ ] **Step 2:** Implement `open_db(path: &Path) -> Result<Connection>` that opens (or creates) the DB, sets PRAGMAs per spec §10.2, loads the sqlite-vec extension, and runs the schema-creation SQL per spec §10.1.
- [ ] **Step 3:** Implement `schema_version(conn) -> u32`. Implement `assert_vector_dimension(conn, embedder_dim: usize)` per spec §10.4.
- [ ] **Step 4: Write tests:**
  - `open_creates_schema_on_fresh_db`: open a `:memory:` DB, assert all tables and indices exist via `sqlite_master` introspection.
  - `vec0_virtual_table_works`: insert + KNN on `document_vec`.
  - `dimension_mismatch_errors_on_open`: open with embedder dim 1024 when schema says 768 → `Err(SchemaMismatch)`.
  - `pragmas_applied`: query `PRAGMA journal_mode` returns `wal`.
- [ ] **Step 5:** Run tests. Expected: pass.
- [ ] **Step 6:** Commit: `feat(sqlite): schema, migrations, and connection setup`.

### Task 12: `SqliteStore` implementing all three storage traits

**Files:**
- Create: `crates/rex-sqlite/src/store.rs` (`SqliteStore` struct, impls `ItemStore`, `VectorStore`, `FtsIndex`)
- Create: `crates/rex-sqlite/src/sql.rs` (filter-to-SQL builder)

- [ ] **Step 1:** Implement `SqliteStore::new(conn: Arc<Mutex<Connection>>) -> Self`. (Use a Mutex around the connection because rusqlite's `Connection` is not `Sync`. Future: connection pool.)
- [ ] **Step 2:** Implement `filter_to_sql(filters: &Filters) -> (String, Vec<rusqlite::types::Value>)` returning the SQL `WHERE` fragment + bind parameters. Tested as a unit.
- [ ] **Step 3: Write integration tests with :memory: DB:**
  - `put_then_get_roundtrips`: insert a Document, get by id, assert equal.
  - `query_with_topic_filter`: insert 5 docs across 3 topics, filter on one topic, expect 2 results.
  - `query_with_compound_filter`: AND across topic + school + paper_type.
  - `count_matches_query_length`.
  - `clear_subject_removes_only_that_subject`: insert into two subjects, clear one, other unaffected.
  - `list_subjects_returns_distinct`.
  - `facet_counts_respects_filters`: counts decrease appropriately when filters applied.
- [ ] **Step 4:** Run tests. Expected: fail (`SqliteStore` methods unimplemented).
- [ ] **Step 5:** Implement `ItemStore` methods: `put` (multi-row INSERT inside a transaction; upsert via `INSERT OR REPLACE`), `get`, `get_many`, `query`, `count`, `list_subjects`, `list_topics`, `facet_counts`, `clear`. Use the filter SQL builder for `query`/`count`/`facet_counts`.
- [ ] **Step 6:** Run tests. Expected: pass.
- [ ] **Step 7:** Commit: `feat(sqlite): ItemStore impl`.

### Task 13: `FtsIndex` impl on FTS5

**Files:**
- Modify: `crates/rex-sqlite/src/store.rs`

- [ ] **Step 1: Write tests:**
  - `fts_upsert_then_search`: insert 3 docs with different texts, search "tension" returns the right one.
  - `fts_search_filtered`: search within a filtered id-set returns intersection.
  - `fts_bm25_orders_by_relevance`: doc with multiple term hits ranks higher than doc with one.
- [ ] **Step 2:** Implement `FtsIndex::upsert` (`INSERT INTO document_fts(rowid, document_id, search_text)`). Implement `FtsIndex::search` using `MATCH ?` and `bm25(document_fts)`. Apply filters via a CTE intersection.
- [ ] **Step 3:** Run tests. Expected: pass.
- [ ] **Step 4:** Commit: `feat(sqlite): FtsIndex impl on FTS5`.

### Task 14: `VectorStore` impl on sqlite-vec + spike verification

**Files:**
- Modify: `crates/rex-sqlite/src/store.rs`
- Create: `crates/rex-sqlite/tests/pushdown_spike.rs` (verifies `WHERE id IN (...)` push-down per spec §10.3)

- [ ] **Step 1: Write tests:**
  - `vec_upsert_then_search`: insert 5 docs with known vectors, search with target vector returns expected neighbors.
  - `vec_search_filtered`: KNN respects `WHERE document_id IN (filtered)`.
- [ ] **Step 2:** Implement `VectorStore::upsert` and `search` using sqlite-vec's `embedding MATCH ?` syntax with `k = ?`. Apply filters via a CTE.
- [ ] **Step 3:** Run tests. Expected: pass.
- [ ] **Step 4:** Implement the push-down spike: generate 5_000 synthetic vectors, time a heavily-filtered query against an unfiltered query. Verify filtered query time is proportional to filtered-set size, not corpus size. Print results in test output; fail if filtered query is more than 5× slower than expected.
- [ ] **Step 5:** Commit: `feat(sqlite): VectorStore impl + push-down spike`.

---

## Chunk 4: `rex-fs-local` + `rex-pdf`

### Task 15: `rex-fs-local`

**Files:**
- Modify: `crates/rex-fs-local/Cargo.toml` (deps: `rex-domain`, `tokio` with `fs` feature, `bytes`)
- Create: `crates/rex-fs-local/src/lib.rs` (`LocalFsBlobStore` impl `BlobStore`)

- [ ] **Step 1:** Implement `LocalFsBlobStore { root: PathBuf }` with `BlobStore::get`, `exists`, `list`. Paths are joined under `root`. Path traversal (`..`) outside `root` returns `Err(BadInput)`.
- [ ] **Step 2: Write tests with tempdir:**
  - `get_reads_file`.
  - `exists_returns_true_when_present_false_otherwise`.
  - `list_returns_files_under_prefix`.
  - `path_traversal_rejected`: `get("../etc/passwd")` errors.
- [ ] **Step 3:** Run tests. Expected: pass.
- [ ] **Step 4:** Commit: `feat(fs-local): LocalFsBlobStore impl`.

### Task 16: `rex-pdf` — page text extraction

**Files:**
- Modify: `crates/rex-pdf/Cargo.toml` (deps: `rex-domain`, `pdfium-render` with bundled feature if available, `bytes`, `tracing`)
- Create: `crates/rex-pdf/src/lib.rs`
- Create: `crates/rex-pdf/src/extract.rs` (`extract_pages(bytes: &Bytes) -> Result<Vec<(u32, String)>>`)
- Create: `crates/rex-pdf/tests/fixtures/` with 1-2 small public-domain PDFs

- [ ] **Step 1:** If `pdfium-render` builds cleanly, use it. If not, fall back to `pdf-extract` crate (pure Rust, less accurate on complex PDFs but no native deps). Document the chosen library in `rex-pdf/README.md`.
- [ ] **Step 2:** Implement `extract_pages` that returns `(page_number_1_indexed, page_text)` for every successfully-parsed page. Pages that fail individually are skipped with WARN log; whole-PDF failure returns `Err(Pdf)`.
- [ ] **Step 3: Write tests:**
  - `extract_simple_pdf_returns_expected_pages`: known fixture with 2 pages and known text snippets.
  - `extract_corrupt_pdf_returns_err`: feed random bytes, assert error.
- [ ] **Step 4:** Run tests. Expected: pass.
- [ ] **Step 5:** Commit: `feat(pdf): page text extraction`.

### Task 17: `rex-pdf` — fuzzy anchor resolution

**Files:**
- Create: `crates/rex-pdf/src/anchor.rs` (`fuzzy_match_page(target: &str, pages: &[(u32, String)]) -> (Option<u32>, f32)`)
- Create: `crates/rex-pdf/src/ngram.rs` (3-gram Jaccard similarity)

- [ ] **Step 1: Write tests:**
  - `ngram_jaccard_identical_strings_is_one`.
  - `ngram_jaccard_disjoint_strings_is_zero`.
  - `ngram_normalizes_case_and_strips_nonalnum`.
  - `fuzzy_match_picks_correct_page_for_known_question`: synthetic 3-page corpus where page 2 contains the target phrase.
  - `fuzzy_match_returns_low_confidence_for_no_overlap`: target unrelated to all pages.
- [ ] **Step 2:** Run tests. Expected: fail.
- [ ] **Step 3:** Implement `ngram_jaccard`. Implement `fuzzy_match_page`: compute jaccard per page, pick the max, return `(Some(page) if max >= 0.6 else None, max)`.
- [ ] **Step 4:** Run tests. Expected: pass.
- [ ] **Step 5:** Commit: `feat(pdf): fuzzy page anchor resolution`.

---

## Chunk 5: `rex-ingest`

### Task 18: JsonlRow schema + parsing with deny_unknown_fields

**Files:**
- Modify: `crates/rex-ingest/Cargo.toml` (deps: `rex-domain`, `rex-pdf`, `serde`, `serde_json`, `tokio` with `fs` + `io-util`, `tracing`, `uuid`, `thiserror`)
- Create: `crates/rex-ingest/src/lib.rs`
- Create: `crates/rex-ingest/src/jsonl.rs` (`JsonlRow` struct + parsing)

- [ ] **Step 1:** Define `JsonlRow` with `#[serde(deny_unknown_fields)]` mirroring the JSONL fields observed in h2physics and h2history sample rows. Include `id`, `parent_id`, `depends_on`, `number`, `source`, `context`, `question`, `mark`, `keywords`, `options`, `answer`, `notes`, `images`, `answer_images`, `tags` (with the 6 sub-fields). Add `kind: Option<String>` for forward-compat if rob ever distinguishes question vs note in the row.
- [ ] **Step 2:** Implement `parse_row(line: &str) -> Result<JsonlRow>`.
- [ ] **Step 3:** Implement `JsonlRow::into_document(subject: SubjectId, kind: DocumentKind) -> Document` mapping.
- [ ] **Step 4: Write tests:**
  - `parse_known_h2physics_row`: hard-code a known good row (from the sample we saw), assert parse succeeds and all fields map.
  - `parse_rejects_unknown_field`: add `"newfield": 42` to the row, assert error mentions the new field.
  - `parse_missing_optional_fields_succeeds`: row with only `id`, `source`, `tags`, `keywords` parses.
- [ ] **Step 5:** Run tests. Expected: pass.
- [ ] **Step 6:** Commit: `feat(ingest): JsonlRow parsing with strict schema`.

### Task 19: Search text builder + path mapping

**Files:**
- Create: `crates/rex-ingest/src/text.rs` (`build_search_text`)
- Create: `crates/rex-ingest/src/path_map.rs` (`markdown_to_pdf_path`)

- [ ] **Step 1:** Implement `build_search_text(doc: &Document) -> String` per spec §7.3.
- [ ] **Step 2:** Implement `source_to_pdf_path(workspace_root: &Path, docs_root: &Path, subject: &SubjectId, source: &SourcePath) -> PathBuf`. Strip any extension from `source`, append `.pdf`. Rebase from `<workspace>/<subject>/content/...` to `<docs-root>/<subject>/...`. This handles both `.md` (h2physics) and `.txt` (h2history) source extensions.
- [ ] **Step 3: Write tests:**
  - `search_text_includes_all_field_prefixes`.
  - `search_text_omits_absent_fields`.
  - `path_map_md_source`: `content/prelims/2019/HCI/X.md` + `h2physics` → `docs/h2physics/prelims/2019/HCI/X.pdf`.
  - `path_map_txt_source`: `content/holy-grail-sites/2023 - Essay X.txt` + `h2history` → `docs/h2history/holy-grail-sites/2023 - Essay X.pdf`.
  - `path_map_handles_spaces_and_hyphens`.
- [ ] **Step 4:** Run tests. Expected: pass.
- [ ] **Step 5:** Commit: `feat(ingest): search text builder and path mapping`.

### Task 20: Ingest pipeline orchestration

**Files:**
- Create: `crates/rex-ingest/src/pipeline.rs` (the `run(config) -> IngestStats` orchestrator)
- Create: `crates/rex-ingest/src/config.rs` (`IngestConfig`)
- Create: `crates/rex-ingest/src/stats.rs` (`IngestStats`)
- Create: `crates/rex-ingest/src/error.rs` (`IngestError` distinct from domain `Error`, including `SchemaDrift`)

- [ ] **Step 1:** Implement `IngestConfig` struct (subject, workspace path, docs root, batch size = 256, max_skip_pct = 5.0, rebuild = false). Implement `IngestStats` (rows_questions, rows_notes, rows_skipped, pdfs_anchored, pdfs_low_confidence, pdfs_read_failed, pdfs_not_found, took_ms).
- [ ] **Step 2:** Implement `run` as: stream parse → batch fold (Phase 1) → for each batch: resolve anchors via rex-pdf + BlobStore (Phase 2) → embed via Embedder (Phase 2) → write via stores in a transaction (Phase 3). Use `Arc<dyn ItemStore>` etc., not concrete types — this crate stays adapter-agnostic.
- [ ] **Step 3:** Implement drift threshold: after Phase 1 of *each file*, if skip rate exceeds `max_skip_pct`, return `IngestError::SchemaDrift`.
- [ ] **Step 4: Write tests using fakes:**
  - `ingest_happy_path`: 10 well-formed rows, all anchored to PDFs (mock BlobStore returning canned bytes; mock rex-pdf via a test feature flag or test-only seam), assert stats counts correct.
  - `ingest_drift_aborts`: 100 rows, 10 malformed → exceeds 5% threshold → returns `SchemaDrift`.
  - `ingest_pdf_read_failure_falls_back`: BlobStore::get errors → anchor with `fallback_reason=PdfReadFailed`.
  - `ingest_unknown_field_aborts_via_threshold`: 100 rows all with an unknown field → all fail parse → threshold abort.
- [ ] **Step 5:** Run tests. Expected: pass.
- [ ] **Step 6:** Commit: `feat(ingest): full pipeline orchestrator`.

---

## Chunk 6: `rex-api`

### Task 21: Router skeleton + health + error mapping

**Files:**
- Modify: `crates/rex-api/Cargo.toml` (deps: `rex-domain`, `rex-search`, `axum`, `tokio` with `full`, `tower`, `tower-http` with `cors` + `limit`, `tracing`, `serde`, `serde_json`, `uuid`, `metrics`, `metrics-exporter-prometheus`)
- Create: `crates/rex-api/src/lib.rs`
- Create: `crates/rex-api/src/router.rs` (`build_router(service: Arc<SearchService>) -> Router`)
- Create: `crates/rex-api/src/error.rs` (`ApiError` wrapping `rex_domain::Error`, `IntoResponse` impl per spec §8.5)
- Create: `crates/rex-api/src/state.rs` (`AppState`)

- [ ] **Step 1:** Implement `ApiError` + `IntoResponse` mapping per spec §8.5 table.
- [ ] **Step 2:** Implement `build_router` that mounts `/v1/health` returning `200 OK`.
- [ ] **Step 3:** Add tower layers: `RequestBodyLimitLayer::new(1_048_576)`, `TimeoutLayer::new(Duration::from_secs(30))`, CORS layer.
- [ ] **Step 4: Write tests via `Router::oneshot`:**
  - `health_returns_200`.
  - `error_not_found_returns_404_with_code`.
  - `error_bad_input_returns_400`.
  - `body_too_large_returns_413`.
- [ ] **Step 5:** Run tests. Expected: pass.
- [ ] **Step 6:** Commit: `feat(api): router + error mapping + health endpoint`.

### Task 22: Discovery endpoints (subjects, tag-values)

**Files:**
- Create: `crates/rex-api/src/handlers/subjects.rs`

- [ ] **Step 1:** Implement `GET /v1/subjects`, `GET /v1/subjects/:id`, `POST /v1/subjects/:id/tag-values/:field`.
- [ ] **Step 2: Write tests with FakeStore-backed SearchService:**
  - Each endpoint returns expected shape against canned data.
  - `tag_values_unknown_field_returns_400`.
- [ ] **Step 3:** Commit: `feat(api): discovery endpoints`.

### Task 23: Search + filter endpoints

**Files:**
- Create: `crates/rex-api/src/handlers/search.rs`
- Create: `crates/rex-api/src/handlers/filter.rs`

- [ ] **Step 1:** Implement `POST /v1/search` and `POST /v1/filter`. Validation per spec §6.3 + §8.2: rejection rules wired in the handlers (also enforced in the service layer for CLI parity).
- [ ] **Step 2: Write tests:**
  - `search_hybrid_mode_returns_hits_with_score_breakdown`.
  - `search_text_missing_returns_400`.
  - `search_mode_filter_with_text_returns_400`.
  - `filter_returns_total_matches_in_meta`.
  - `filter_pagination_works`.
- [ ] **Step 3:** Commit: `feat(api): search + filter endpoints`.

### Task 24: Document endpoints + PDF endpoints

**Files:**
- Create: `crates/rex-api/src/handlers/documents.rs`
- Create: `crates/rex-api/src/pdf_slicer.rs` (page-slicing via `lopdf` with LRU)

- [ ] **Step 1:** Add `lopdf` dep. Implement `slice_page(pdf_bytes: &[u8], page: u32) -> Result<Vec<u8>>`.
- [ ] **Step 2:** Implement `GET /v1/documents/:id`, `POST /v1/documents/batch`, `GET /v1/documents/:id/pdf-anchor`, `GET /v1/documents/:id/pdf`, `GET /v1/documents/:id/pdf/page/:n`.
- [ ] **Step 3:** Add LRU cache (`lru` crate, 50 entries) over `(document_id, page) -> Bytes`.
- [ ] **Step 4: Write tests:**
  - `get_document_returns_expected_shape`.
  - `pdf_anchor_returns_404_when_doc_absent`.
  - `pdf_page_slice_round_trip` (with a small fixture PDF).
- [ ] **Step 5:** Commit: `feat(api): document + PDF endpoints with page slicing`.

### Task 25: Metrics endpoint + observability wiring

**Files:**
- Create: `crates/rex-api/src/observability.rs`
- Create: `crates/rex-api/src/handlers/metrics.rs`

- [ ] **Step 1:** Initialize `metrics-exporter-prometheus` recorder. Expose `/v1/metrics`.
- [ ] **Step 2:** Spawn a background task that samples process RSS, vsize, threads, FDs every 10s via `sysinfo`.
- [ ] **Step 3:** Emit `rex_requests_total`, `rex_request_duration_ms` via a tower middleware. Emit `rex_search_stage_duration_ms{stage}` from inside `SearchService` (via a hook trait or a context-passed metrics handle).
- [ ] **Step 4: Write test:**
  - `metrics_endpoint_serves_prometheus_text`: GET /v1/metrics returns text starting with `# HELP`.
- [ ] **Step 5:** Commit: `feat(api): metrics endpoint + process observability`.

---

## Chunk 7: `rex-cli`

### Task 26: clap structure + service wiring

**Files:**
- Modify: `crates/rex-cli/Cargo.toml` (deps: all internal crates, `clap` with `derive`, `tokio` with `full`, `anyhow`, `tracing-subscriber`, `serde_json`)
- Create: `crates/rex-cli/src/main.rs`
- Create: `crates/rex-cli/src/cli.rs` (clap structures)
- Create: `crates/rex-cli/src/wire.rs` (concrete-adapter wiring helpers)
- Create: `crates/rex-cli/src/output.rs` (pretty vs `--json` formatters)

- [ ] **Step 1:** Define top-level `Cli { #[command(subcommand)] cmd: Cmd, #[arg(global)] json: bool, #[arg(global)] remote: Option<String> }` with subcommands: `Ingest`, `Serve`, `Search`, `Filter`, `Get`, `GetMany`, `PdfAnchor`, `Pdf`, `Subjects`, `Subject`, `TagValues`. Each subcommand is its own struct with derive(`Args`).
- [ ] **Step 2:** Implement `wire::open_service_for_read(db_path, docs_root, models_dir, lazy_embedder: bool) -> SearchService` and `wire::open_for_ingest(db_path, docs_root) -> (services...)`. Lazy embedder loading: wrap `Arc<dyn Embedder>` behind a `OnceCell` that only loads on first use.
- [ ] **Step 3:** `--remote` returns exit 64 with the message per spec §9.4.
- [ ] **Step 4:** Commit: `feat(cli): clap structure + wiring helpers`.

### Task 27: Read subcommands (search, filter, get, subjects, tag-values, pdf-anchor)

**Files:**
- Create: `crates/rex-cli/src/commands/search.rs`
- Create: `crates/rex-cli/src/commands/filter.rs`
- Create: `crates/rex-cli/src/commands/get.rs`
- Create: `crates/rex-cli/src/commands/subjects.rs`
- Create: `crates/rex-cli/src/commands/tag_values.rs`
- Create: `crates/rex-cli/src/commands/pdf.rs`

- [ ] **Step 1:** Implement each command as a thin wrapper that builds the request struct from clap args, calls the appropriate `SearchService` method, and renders via `output::render` (which switches on `--json`).
- [ ] **Step 2: Write `insta` snapshot tests** for each subcommand's pretty output (with FakeStore-backed service).
- [ ] **Step 3:** Commit: `feat(cli): read subcommands`.

### Task 28: `rex ingest` subcommand

**Files:**
- Create: `crates/rex-cli/src/commands/ingest.rs`

- [ ] **Step 1:** Wire concrete adapters: `rex_sqlite::SqliteStore`, `rex_fs_local::LocalFsBlobStore`, the embedder (real or stub per Chunk 8 status), `rex_pdf::extract_pages` + `fuzzy_match_page`.
- [ ] **Step 2:** Invoke `rex_ingest::pipeline::run(config)` with a progress bar via `indicatif`.
- [ ] **Step 3:** Print the stats summary per spec §7.6 (with the per-reason fallback breakdown).
- [ ] **Step 4:** Commit: `feat(cli): ingest subcommand`.

### Task 29: `rex serve` subcommand

**Files:**
- Create: `crates/rex-cli/src/commands/serve.rs`

- [ ] **Step 1:** Wire `rex_api::build_router` with the `SearchService` constructed via `wire`. Honor `--bind`, `--cors-allow`, `--no-reranker`, `--warm` flags.
- [ ] **Step 2:** Initialize tracing-subscriber with JSON vs pretty per `REX_LOG_FORMAT`.
- [ ] **Step 3:** If `--warm`, pre-load the embedder + reranker before binding.
- [ ] **Step 4:** Commit: `feat(cli): serve subcommand`.

---

## Chunk 8: Embedder integration (`rex-llamacpp`) — best-effort

### Task 30: Stub embedder (always works)

**Files:**
- Modify: `crates/rex-search/src/fakes.rs` (already done in Chunk 2)
- Create: `crates/rex-llamacpp/src/stub.rs` (`StubEmbedder` for use when GGUF unavailable)

- [ ] **Step 1:** Re-purpose the `FakeEmbedder` logic into a public `StubEmbedder` in `rex-llamacpp` (or alternatively a small `rex-embedder-stub` crate if we want to keep `rex-llamacpp` strictly real). For simplicity, gate behind a `stub` feature in `rex-llamacpp`.
- [ ] **Step 2:** Wire CLI to use stub by default when `--embedder=stub` or when GGUF model load fails.
- [ ] **Step 3:** Commit: `feat(llamacpp): stub embedder for fallback`.

### Task 31: Real GGUF embedder (best-effort)

**Files:**
- Modify: `crates/rex-llamacpp/Cargo.toml` (deps: `rex-domain`, `llama-cpp-2`, `tokio`, `tracing`)
- Create: `crates/rex-llamacpp/src/embedder.rs` (`GgufEmbedder`)
- Create: `crates/rex-llamacpp/src/reranker.rs` (`GgufReranker`)
- Create: `scripts/fetch-models.sh`

- [ ] **Step 1:** Write `fetch-models.sh` that downloads `embeddinggemma-300M-Q8_0.gguf` and `Qwen3-Reranker-0.6B-Q8_0.gguf` from HuggingFace into `./models/`. Idempotent (skips if files exist).
- [ ] **Step 2:** Attempt to add `llama-cpp-2` dep and build. If build fails on the current host, log the failure mode in `crates/rex-llamacpp/BUILD-NOTES.md` and proceed with the stub-only path.
- [ ] **Step 3:** If build succeeds: implement `GgufEmbedder::new(model_path) -> Result<Self>` and the `Embedder` trait. Apply the nomic-style prefix conventions (`task: search_query`, `task: search_document`) per the embeddinggemma docs.
- [ ] **Step 4:** Implement `GgufReranker` similarly.
- [ ] **Step 5: Behind `--features llama-tests`:** smoke tests that produce non-trivial embeddings and reranker scores.
- [ ] **Step 6:** Commit: `feat(llamacpp): GGUF embedder + reranker` (or `chore(llamacpp): document build blocker, defer to v1.1` if Step 2 fails).

---

## Chunk 9: End-to-end validation on h2history

### Task 32: Build + first ingest (expect to discover schema gaps)

- [ ] **Step 1:** Run `cargo build --release --bin rex`.
- [ ] **Step 2:** Run:
  ```bash
  ./target/release/rex ingest \
    --subject h2history \
    --workspace /Users/jcjustin/Projects/tippytop/ren-subjects/workspace \
    --docs-root /Users/jcjustin/Projects/tippytop/ren-subjects/docs \
    --db ./rex.db
  ```
- [ ] **Step 3:** Observe the failure. Possible failure modes (record which we hit):
  - `deny_unknown_fields` rejects a field we missed → add it to `JsonlRow`.
  - Drift threshold exceeded → look at the sample errors, decide if the field is real or noise.
  - PDF path mapping wrong for h2history's `.txt` source → fix path_map (might also need source-extension handling).
  - PDFs not found → log per-PDF status (with `PdfNotFound` reason) and continue.
  - Embedder unavailable → fall back to stub.
- [ ] **Step 4:** Fix the discovered issue(s), commit each as `fix(ingest): handle <specific issue>`.
- [ ] **Step 5:** Re-run ingest. Iterate until it completes successfully.

### Task 33: Verify ingested data via filter

- [ ] **Step 1:**
  ```bash
  ./target/release/rex subjects
  # Expected: shows h2history with a positive item count
  ```
- [ ] **Step 2:**
  ```bash
  ./target/release/rex filter --subject h2history --limit 5
  # Expected: 5 documents, JSON or pretty
  ```
- [ ] **Step 3:**
  ```bash
  ./target/release/rex tag-values --subject h2history --field topics
  # Expected: list of topic values with counts
  ```
- [ ] **Step 4:** Commit if any fixes were needed.

### Task 34: Run search end-to-end

- [ ] **Step 1:**
  ```bash
  ./target/release/rex search "United Nations" --subject h2history --mode bm25 --limit 5
  # Expected: hits with BM25 scores, no embedding involved
  ```
- [ ] **Step 2:** If real embedder is wired:
  ```bash
  ./target/release/rex search "international cooperation post-cold-war" --subject h2history --mode hybrid --limit 5
  # Expected: hits with all three scores, rerank applied
  ```
- [ ] **Step 3:** Smoke-test PDF endpoint if PDFs were found:
  ```bash
  DOC=$(./target/release/rex filter --subject h2history --limit 1 --json | jq -r '.hits[0].document.id')
  ./target/release/rex pdf-anchor "$DOC"
  ```
- [ ] **Step 4:** Commit any fixes.

### Task 35: Serve test

- [ ] **Step 1:**
  ```bash
  ./target/release/rex serve --db ./rex.db --docs-root /Users/jcjustin/Projects/tippytop/ren-subjects/docs --bind 127.0.0.1:8080 &
  ```
- [ ] **Step 2:**
  ```bash
  curl -s http://127.0.0.1:8080/v1/health           # expect 200 OK
  curl -s http://127.0.0.1:8080/v1/subjects          # expect h2history listed
  curl -s -X POST http://127.0.0.1:8080/v1/search \
    -H 'Content-Type: application/json' \
    -d '{"text":"United Nations","mode":"Bm25Only","filters":{"subject":"h2history"},"limit":5}'
  ```
- [ ] **Step 3:** Inspect `meta` field of the response; confirm `used_bm25=true`, `used_embedder=false`, timings present.
- [ ] **Step 4:** `curl http://127.0.0.1:8080/v1/metrics` and confirm RSS gauge is emitted.
- [ ] **Step 5:** Kill the server.
- [ ] **Step 6:** Commit a small README.md update with the validated quickstart commands.

### Task 36: Final sweep + CI parity test

- [ ] **Step 1:** Run `cargo test --workspace`. Expected: everything green (except the optional `llama-tests`-feature tests).
- [ ] **Step 2:** Run `cargo clippy --workspace --all-targets -- -D warnings`. Fix any lints.
- [ ] **Step 3:** Run `cargo fmt --check`. Fix formatting.
- [ ] **Step 4:** Confirm the `cli_api_parity` test passes.
- [ ] **Step 5:** Commit: `chore: green workspace tests + clippy clean`.

---

## What's NOT in this plan (explicit YAGNI markers)

These are spec-listed §14 future work; the implementation plan does **not** include them:

- Incremental ingest (no manifest-based mtime tracking).
- PDF text as a second searchable corpus.
- Authentication.
- HTTP admin ingest endpoint.
- Postgres / Qdrant / S3 adapter crates.
- pprof / tokio-console.
- TypeScript / Python clients.
- gRPC.
- Shared JSON Schema with rob-the-crawler.

## When to stop and surface

Stop and ask the user if any of these happen:

1. `pdfium-render` fails to build AND `pdf-extract` also fails → we have no PDF text extraction; spec §7 cannot be implemented.
2. `sqlite-vec` extension cannot be loaded on the target host → no vector store; spec §6 hybrid pipeline degrades to BM25-only.
3. The h2history ingest reveals that the corpus is structurally incompatible with our `JsonlRow` (more than a few discovered fields) — this may signal that h2physics and h2history have diverged enough that they need separate schemas, which is a larger design decision.

Otherwise: proceed autonomously per the user's instruction to skip approval gates.
