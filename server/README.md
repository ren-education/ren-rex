# rex — server

PDF search & navigator server for `ren-subjects` content.

See the design spec: [`../docs/superpowers/specs/2026-05-15-rex-pdf-search-server-design.md`](../docs/superpowers/specs/2026-05-15-rex-pdf-search-server-design.md).

## Quickstart (CLI)

```bash
# Build
cargo build --release --bin rex

# Ingest a subject (example: h2history from ren-subjects)
./target/release/rex ingest \
  --subject h2history \
  --workspace /path/to/ren-subjects/workspace \
  --docs-root /path/to/ren-subjects/docs \
  --db ./rex.db

# Search
./target/release/rex search "United Nations" --subject h2history --mode bm25 --limit 5

# Filter only
./target/release/rex filter --subject h2history --topic dynamics --limit 10

# List subjects
./target/release/rex subjects
```

## Quickstart (HTTP)

```bash
./target/release/rex serve --db ./rex.db --docs-root /path/to/ren-subjects/docs --bind 127.0.0.1:8080

# Health
curl http://127.0.0.1:8080/v1/health

# Search
curl -s -X POST http://127.0.0.1:8080/v1/search \
  -H 'Content-Type: application/json' \
  -d '{"text":"United Nations","mode":"Bm25Only","filters":{"subject":"h2history"},"limit":5}'
```

## Layout

Cargo workspace with 9 crates under [`crates/`](crates/):

- `rex-domain` — pure types + trait definitions (ports). Zero external runtime deps.
- `rex-search` — query pipeline (BM25 + vector + rerank fusion).
- `rex-ingest` — JSONL parser + indexing orchestrator.
- `rex-pdf` — PDF text extraction + fuzzy page anchoring.
- `rex-sqlite` — SQLite + sqlite-vec + FTS5 adapter (ItemStore, VectorStore, FtsIndex).
- `rex-llamacpp` — GGUF embedder + cross-encoder reranker via `llama-cpp-2` (best-effort).
- `rex-fs-local` — local filesystem `BlobStore`.
- `rex-api` — axum HTTP layer.
- `rex-cli` — the `rex` binary; wires concrete adapters together.

All adapter crates depend only on `rex-domain`. No adapter-to-adapter dependencies — the Cargo graph enforces hexagonal boundaries.
