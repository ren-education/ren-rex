# Related files ("answer scheme") for a question — design

**Date:** 2026-06-08
**Status:** Approved, ready for implementation plan

## Problem

Students search/filter questions in the rex client and open the question's PDF in
the viewer. There is no answer scheme: `questions.jsonl` carries no pointer to the
solutions PDF, so a student who finds a question cannot get to its worked answer.

In the source corpus the answer scheme already lives **next to** the question paper.
For example `docs/h2physics/prelims/2022/TMJC/` contains both:

- `TMJC_2022_H2_Physics_P1_QP.pdf` (question paper)
- `10._2022_J2_H2_Prelim_P1_Solutions.pdf` (answer scheme)

Each question already resolves to a specific PDF via `pdf_anchor.pdf_path`. So we can
derive "relevant files" from the folder that PDF lives in, and surface the sibling
files — including the solutions — without any new metadata.

## Approach

Show the **immediate sibling files** of the question's PDF (the parent directory of
`pdf_anchor.pdf_path`) as a plain, clickable list in the PDF viewer. Clicking opens
the file in a new browser tab via a new serve-by-path endpoint.

### Decisions (locked during brainstorming)

- **Scope:** exact parent folder of `pdf_path`. Immediate children only — if a subject
  nests papers in per-paper subfolders, those nested files are not pulled in.
- **Presentation:** raw filenames, clickable. No classification, no badges, no
  answer-scheme detection, no reordering.
- **Placement:** a "Files in this folder" list under the existing filename header in
  the PDF viewer. Click opens in a new tab. No in-viewer swap.

### Why this approach

The storage layer (`rex-fs-local`) already exposes `list(prefix)` with path-traversal
protection, and every question already has a resolved `pdf_path`. The whole feature is
therefore a thin slice: two small read endpoints plus one viewer section, with zero new
ingest/metadata work and zero changes to `questions.jsonl`.

## Architecture

### Backend — `rex-api`, two new endpoints

#### `GET /v1/documents/:id/related-files`

Lists the PDF siblings of a question's own PDF.

1. Parse `:id`; resolve `pdf_anchor` via `service.pdf_anchor` (same path the existing
   `get_pdf` handler uses).
2. If the blob store is not configured, the anchor is absent, or it has no usable
   `pdf_path` → return `{ "files": [] }`. This endpoint **never errors** on "no files";
   the viewer simply hides the section.
3. `dir = pdf_path.parent()`.
4. `blobs.list(dir)` (recursive, root-relative, sorted), then filter to:
   - **immediate children** of `dir` (drop anything in a nested subfolder),
   - **`.pdf` extension only** (reuse the currently-dead `ext_is_pdf` helper in
     `documents.rs`),
   - **excluding the question's own `pdf_path`**.
5. Respond:

```json
{
  "dir": "h2physics/prelims/2022/TMJC",
  "files": [
    { "path": "h2physics/prelims/2022/TMJC/10._2022_J2_H2_Prelim_P1_Solutions.pdf",
      "filename": "10._2022_J2_H2_Prelim_P1_Solutions.pdf" }
  ]
}
```

`path` is the root-relative path (for the serve endpoint); `filename` is the basename
(for display). Order follows `list`'s sort.

#### `GET /v1/files/*path`

Serves a sibling PDF by root-relative path (the existing `get_pdf` serves by doc id
only; siblings have no doc id).

1. `blobs.get(path)` — `safe_join` already rejects `..` traversal (→ `BadInput`/400).
2. **Restrict to `.pdf`** — any other extension → 404. This is a new arbitrary-path
   read surface; the extension allowlist plus the existing root-jail are the security
   boundary.
3. On success: `200`, `Content-Type: application/pdf`,
   `Content-Disposition: inline; filename="<basename>"`.

### Client

- **`lib/types.ts`** — add:
  - `RelatedFile { path: string; filename: string }`
  - `RelatedFilesResponse { dir: string; files: RelatedFile[] }`
- **`lib/rex.ts`** — add `relatedFiles(docId: string): Promise<RelatedFilesResponse>`
  (GET `/v1/documents/:id/related-files`, same `request` helper / proxy behavior as the
  other calls).
- **`components/pdf-viewer.tsx`** — when a hit is selected, fetch related files in an
  abortable `useEffect` keyed on the hit (abort + refetch when the selected hit changes).
  Render a compact **"Files in this folder"** list beneath the existing filename header.
  Each entry:

  ```tsx
  <a href={`/v1/files/${file.path.split("/").map(encodeURIComponent).join("/")}`}
     target="_blank" rel="noopener noreferrer">
    {file.filename}
  </a>
  ```

  Per-segment `encodeURIComponent` preserves the slashes for the axum `*path` wildcard
  while encoding spaces/parens in folder names (e.g. `Physics H2 (Set 2 of 5)`). The
  section is hidden while loading, on error, and when `files` is empty.
- **PostHog** — fire an event on related-file click, matching the existing
  "Track search and PDF interactions in PostHog" pattern (include `dir`, `filename`,
  and the source document id).

## Error handling

| Case | Behavior |
|------|----------|
| Doc has no `pdf_anchor` / no `pdf_path` | `related-files` → `{ "files": [] }`; section hidden |
| Blob store not configured | `related-files` → `{ "files": [] }` |
| Folder missing on disk | `list` swallows the metadata error → empty list |
| `/v1/files` path not found | 404 |
| `/v1/files` non-pdf extension | 404 |
| `/v1/files` `..` traversal | 400 (`BadInput`) |
| Client fetch fails | Section hidden; question PDF still renders |

## Testing

**Rust (`rex-api`):**
- `related-files` returns immediate-children PDFs, excludes the question's own PDF,
  and ignores files in nested subfolders.
- `related-files` returns `{ files: [] }` when the doc has no anchor.
- `/v1/files` serves a PDF body with `application/pdf`.
- `/v1/files` 404s a non-pdf path.
- `/v1/files` 400s on a `..` traversal path.

**Client:**
- Unit-test the per-segment path encoder (spaces, parens, slashes preserved).

## Out of scope (YAGNI)

- File-type classification / badges / answer-scheme detection.
- Reordering so solutions float to the top.
- In-viewer swap (loading a sibling into the existing viewer in place).
- Broader-than-folder scope (school/year subtree).
- Caching the directory listing.
- Serving non-PDF files.
