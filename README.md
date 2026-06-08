# rex

**rex** is a PDF search & navigator for educational content.

Given a corpus of compiled exam questions, notes, and source PDFs (produced
upstream by `rob-the-crawler`), rex makes that material findable. Students
and teachers can search by keyword and by meaning, filter by topic / school
/ paper / year, and jump directly to the PDF page a question originated
from.

## Who it is for

- **Students** revising for A-Level subjects: "show me every Cold War
  question across all schools, ranked by relevance."
- **Teachers** assembling resources: "find me five questions on circular
  motion at Paper 2 difficulty from 2018-2024."
- **Curriculum designers** auditing coverage: "what's the distribution
  of topics across HCI's question bank?"

The first subjects loaded are Singapore A-Level: h2physics, h2history,
hcchem, h2econs, english, hcl, gp.

## What's in this repo

`ren-rex` is a small monorepo. Three top-level directories:

```
ren-rex/
├── server/     ← the rex server, a Rust workspace (Cargo)
├── client/     ← the rex client, a Next.js + shadcn app
└── docs/       ← the design spec and implementation plan
```

### `server/`

A Rust Cargo workspace with **9 crates** organized as ports-and-adapters.
The boundaries are enforced by the crate graph — see
[`docs/superpowers/specs/2026-05-15-rex-pdf-search-server-design.md`](docs/superpowers/specs/2026-05-15-rex-pdf-search-server-design.md)
§4 for the dependency diagram.

Ships two binary modes:

- **`rex ingest`** — reads `ren-subjects/workspace/<subject>/reference/{questions,notes}.jsonl`,
  resolves PDF page anchors via fuzzy matching, embeds each item, writes
  everything to a single SQLite file (`rex.db`).
- **`rex serve`** — reads `rex.db`, exposes the search/filter/document
  endpoints over HTTP (axum), serves PDF bytes by path.

Plus the same surface via CLI (`rex search`, `rex filter`, `rex get`, …)
for scripting and ops. CLI/API parity is enforced by a CI test.

### `client/`

A Next.js 15 + React 19 app using shadcn primitives. Talks to the rex
server via HTTP rewrites under `/v1/*`. Theming follows the W3C Design
Tokens spec with a three-tier architecture (reference → semantic →
component). The active aesthetic is "Sage & Linen" (A4); the system is
set up so additional themes can be added by dropping in a `themes/<name>.css`
file — see [`client/DESIGN.md`](client/DESIGN.md).

### `docs/`

- [`docs/superpowers/specs/2026-05-15-rex-pdf-search-server-design.md`](docs/superpowers/specs/2026-05-15-rex-pdf-search-server-design.md)
  — the design spec for the server. Covers architecture, domain model,
  port traits, search pipeline (BM25 + vector + cross-encoder rerank with
  RRF fusion), ingest pipeline, HTTP API, storage schema, testing.
- [`docs/superpowers/plans/2026-05-15-rex-implementation.md`](docs/superpowers/plans/2026-05-15-rex-implementation.md)
  — the chunked implementation plan that was executed to produce the
  current code.

## How rex relates to the rest of ren-education

```
                            ┌─────────────────────────┐
   Singapore JC exam        │   ren-subjects/         │
   PDFs (prelims, promos,  ─┼─→ workspace + docs       │
   holy-grail, notes)       │                         │
                            └────────────┬────────────┘
                                         │ rob-the-crawler ingests
                                         ▼  PDFs → markdown → LLM
                            ┌─────────────────────────┐
                            │  questions.jsonl /      │
                            │  notes.jsonl per subject│
                            └────────────┬────────────┘
                                         │ rex ingest
                                         ▼
                            ┌─────────────────────────┐
                            │  rex.db (SQLite + FTS5  │
                            │  + sqlite-vec)          │
                            └────────────┬────────────┘
                                         │ rex serve
                                         ▼
                            ┌─────────────────────────┐
                            │  rex client (Next.js)   │
                            │  /v1/*  →  rex-api       │
                            └─────────────────────────┘
```

- **`rob-the-crawler`** (sibling repo) crawls PDFs → markdown → compiled
  JSONL. rex is downstream of rob.
- **`ren-subjects`** holds the content (raw PDFs, extracted markdown,
  compiled JSONL).
- **rex** is the read side: makes that content searchable + navigable.

## Quickstart

```bash
# ── Server ─────────────────────────────────────────
cd server/
cargo build --release --bin rex

# Ingest a subject (~60s for h2history)
./target/release/rex ingest \
  --subject h2history \
  --workspace /path/to/ren-subjects/workspace \
  --docs-root /path/to/ren-subjects/docs

# CLI search
./target/release/rex search "United Nations" --subject h2history --mode bm25 --limit 5

# HTTP server (once rex-api is wired)
./target/release/rex serve --bind 127.0.0.1:8080 --docs-root /path/to/ren-subjects/docs

# ── Client ─────────────────────────────────────────
cd ../client/
pnpm install
cp .env.example .env.local
pnpm dev   # → http://localhost:3000
```

Full server-side commands: see [`server/README.md`](server/README.md).
Client-side: see [`client/README.md`](client/README.md).

## Deploy

The server runs on an AWS Lightsail box (`rex-prod`). For a **code-only**
change, deploy with the manual GitHub Action — it builds the binary in CI,
ships it over SSH, swaps `/usr/local/bin/rex`, restarts `rex.service`, and
smoke-tests:

```bash
# after pushing the code you want live:
gh workflow run "Deploy rex server (Lightsail)" -R ren-education/ren-rex --ref main
gh run watch -R ren-education/ren-rex
```

This is a pure binary swap — it does **not** touch `rex.db` or the PDF corpus,
so no reindex. After ingesting new content (new DB / PDFs), use the data-aware
deploy from the dev box instead: `ren-infra/aws/rex/deploy.sh --rsync-pdfs`.

The client is a separate Next.js app deployed on Vercel (auto-deploys on push).
Full deploy + first-time setup: [`ren-infra/runbooks/rex-deploy.md`](../ren-infra/runbooks/rex-deploy.md).

## Status

| Area | Status |
|---|---|
| Domain model + ports | ✅ shipped (`rex-domain`) |
| Search pipeline (BM25 + vector + rerank, RRF fusion) | ✅ shipped (`rex-search`) |
| SQLite + FTS5 storage | ✅ shipped (`rex-sqlite`) |
| Local filesystem blob store | ✅ shipped (`rex-fs-local`) |
| PDF text extraction + fuzzy anchor | ✅ shipped (`rex-pdf`) |
| Ingest orchestrator | ✅ shipped (`rex-ingest`) |
| CLI | ✅ shipped (`rex-cli`) |
| Embedder | 🟡 deterministic stub shipped; real GGUF deferred |
| HTTP server | 🟡 scaffold only; endpoints in progress (`rex-api`) |
| Client (search + filter + browse) | 🟡 design system shipped; live data wiring in progress |
| Client (PDF renderer) | ⏳ next |
| Auth / multi-tenant | ⏳ future |

## License

MIT. See [LICENSE](LICENSE) if present.
