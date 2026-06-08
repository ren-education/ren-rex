# Related Files (Answer Scheme) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Surface the sibling files (incl. the answer scheme) of a question's PDF in the rex viewer, derived from the parent folder of `pdf_anchor.pdf_path`.

**Architecture:** Two new read-only `rex-api` endpoints — `GET /v1/documents/:id/related-files` (lists immediate-child PDFs of the question's folder, excluding the question's own PDF) and `GET /v1/files/*path` (serves a PDF by root-relative path). The client fetches the list in the PDF viewer and renders clickable links that open in a new tab. No DB/index/schema changes; no reindex.

**Tech Stack:** Rust (axum, `rex-api`/`rex-fs-local`/`rex-domain`), Next.js/React/TypeScript client, PostHog.

---

## File Structure

- `server/crates/rex-api/src/handlers/documents.rs` — add `related_files_in_dir` (pure helper), `get_related_files` + `get_file` handlers, `RelatedFile`/`RelatedFilesResponse` structs; promote `ext_is_pdf` from dead code; unit tests.
- `server/crates/rex-api/src/handlers/mod.rs` — register the two new routes + declared_routes entries.
- `client/src/lib/types.ts` — `RelatedFile`, `RelatedFilesResponse`.
- `client/src/lib/rex.ts` — `relatedFiles(docId)` fetcher.
- `client/src/lib/utils.ts` — `encodeFilePath` (per-segment URL encoder).
- `client/src/components/pdf-viewer.tsx` — fetch + render "Files in this folder" list, PostHog event.

---

## Task 1: Pure helper — filter listing to immediate-child PDFs

**Files:**
- Modify: `server/crates/rex-api/src/handlers/documents.rs`

- [ ] **Step 1: Write the failing test.** Append to the bottom of `documents.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn related_files_in_dir_filters_to_immediate_pdf_siblings() {
        let dir = StdPath::new("h2physics/prelims/2022/TMJC");
        let own = StdPath::new("h2physics/prelims/2022/TMJC/TMJC_2022_H2_Physics_P1_QP.pdf");
        let all = vec![
            PathBuf::from("h2physics/prelims/2022/TMJC/TMJC_2022_H2_Physics_P1_QP.pdf"), // own -> excluded
            PathBuf::from("h2physics/prelims/2022/TMJC/10._P1_Solutions.pdf"),           // sibling pdf -> kept
            PathBuf::from("h2physics/prelims/2022/TMJC/notes.txt"),                      // non-pdf -> excluded
            PathBuf::from("h2physics/prelims/2022/TMJC/nested/extra.pdf"),               // nested -> excluded
            PathBuf::from("h2physics/prelims/2022/RI/RI_P1_QP.pdf"),                     // other dir -> excluded
        ];
        let out = related_files_in_dir(&all, dir, own);
        assert_eq!(out.len(), 1, "only the immediate-child solutions PDF survives");
        assert_eq!(out[0].filename, "10._P1_Solutions.pdf");
        assert_eq!(out[0].path, "h2physics/prelims/2022/TMJC/10._P1_Solutions.pdf");
    }
}
```

- [ ] **Step 2: Run to verify it fails.** Run: `cd server && cargo test -p rex-api related_files_in_dir_filters`. Expected: FAIL (`related_files_in_dir` not found, `RelatedFile` not found).

- [ ] **Step 3: Implement the helper + structs.** In `documents.rs`, change the `Serialize` use (top) and replace the dead `ext_is_pdf` block. Add imports near the top:

```rust
use std::path::{Path as StdPath, PathBuf};
use serde::Serialize;
```

Add before the `#[cfg(test)]` block:

```rust
#[derive(Serialize)]
pub struct RelatedFile {
    /// Root-relative path, for the `/v1/files/*path` serve endpoint.
    pub path: String,
    /// Basename, for display.
    pub filename: String,
}

#[derive(Serialize)]
pub struct RelatedFilesResponse {
    pub dir: String,
    pub files: Vec<RelatedFile>,
}

fn ext_is_pdf(p: &StdPath) -> bool {
    p.extension()
        .and_then(|s| s.to_str())
        .map(|s| s.eq_ignore_ascii_case("pdf"))
        .unwrap_or(false)
}

/// Filter a recursive blob listing down to the immediate-child PDFs of `dir`,
/// excluding the question's own PDF. Pure, so it is unit-tested without HTTP.
fn related_files_in_dir(all: &[PathBuf], dir: &StdPath, exclude: &StdPath) -> Vec<RelatedFile> {
    let mut out: Vec<RelatedFile> = all
        .iter()
        .filter(|p| p.parent() == Some(dir))
        .filter(|p| p.as_path() != exclude)
        .filter(|p| ext_is_pdf(p))
        .filter_map(|p| {
            let filename = p.file_name()?.to_str()?.to_string();
            Some(RelatedFile {
                path: p.to_string_lossy().into_owned(),
                filename,
            })
        })
        .collect();
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}
```

Delete the old `#[allow(dead_code)] fn ext_is_pdf(...)` at the bottom of the file (replaced above).

- [ ] **Step 4: Run to verify it passes.** Run: `cd server && cargo test -p rex-api related_files_in_dir_filters`. Expected: PASS.

- [ ] **Step 5: Commit.**

```bash
git add server/crates/rex-api/src/handlers/documents.rs
git commit -m "rex: add related_files_in_dir helper for sibling PDFs"
```

---

## Task 2: `GET /v1/documents/:id/related-files` handler + route

**Files:**
- Modify: `server/crates/rex-api/src/handlers/documents.rs`
- Modify: `server/crates/rex-api/src/handlers/mod.rs`

- [ ] **Step 1: Add the handler.** In `documents.rs`, after `get_pdf`:

```rust
pub async fn get_related_files(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<RelatedFilesResponse>, ApiError> {
    let id = DocumentId::parse(&id)
        .map_err(|e| Error::bad_input_field(format!("invalid id: {e}"), "id"))?;
    let doc = state.service.get(&id).await?;

    // No blob store, or the document has no PDF anchor -> empty list (not an error).
    let (blobs, pdf_path) = match (state.blobs.clone(), doc.pdf_anchor) {
        (Some(b), Some(a)) => (b, a.pdf_path),
        _ => {
            return Ok(Json(RelatedFilesResponse {
                dir: String::new(),
                files: vec![],
            }))
        }
    };

    let dir = pdf_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_default();
    // list() swallows missing-dir errors and returns []. Fall back to [] on any error.
    let all = blobs.list(&dir).await.unwrap_or_default();
    let files = related_files_in_dir(&all, &dir, &pdf_path);

    Ok(Json(RelatedFilesResponse {
        dir: dir.to_string_lossy().into_owned(),
        files,
    }))
}
```

- [ ] **Step 2: Register the route.** In `mod.rs`, add after the `/v1/documents/:id/pdf` route:

```rust
        .route(
            "/v1/documents/:id/related-files",
            get(documents::get_related_files),
        )
```

And add to `declared_routes()` vec after `"GET /v1/documents/:id/pdf"`:

```rust
        "GET /v1/documents/:id/related-files",
```

- [ ] **Step 3: Build to verify it compiles.** Run: `cd server && cargo build -p rex-api`. Expected: success.

- [ ] **Step 4: Commit.**

```bash
git add server/crates/rex-api/src/handlers/documents.rs server/crates/rex-api/src/handlers/mod.rs
git commit -m "rex: add GET /v1/documents/:id/related-files"
```

---

## Task 3: `GET /v1/files/*path` serve-by-path handler + route

**Files:**
- Modify: `server/crates/rex-api/src/handlers/documents.rs`
- Modify: `server/crates/rex-api/src/handlers/mod.rs`

- [ ] **Step 1: Add the handler.** In `documents.rs`, after `get_related_files`:

```rust
pub async fn get_file(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> Result<Response, ApiError> {
    let blobs = state.blobs.clone().ok_or_else(|| {
        Error::not_found("PDF blob store not configured on this server")
    })?;

    let rel = StdPath::new(&path);
    // Only PDFs are served from this arbitrary-path surface.
    if !ext_is_pdf(rel) {
        return Err(Error::not_found("only PDF files are served"));
    }

    // safe_join inside the blob store rejects `..` traversal (-> BadInput -> 400).
    let bytes = blobs.get(rel).await?;
    let filename = rel
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("document.pdf")
        .to_string();
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/pdf"),
            (
                header::CONTENT_DISPOSITION,
                Box::leak(format!("inline; filename=\"{}\"", filename).into_boxed_str()),
            ),
        ],
        Body::from(bytes),
    )
        .into_response())
}
```

- [ ] **Step 2: Register the route.** In `mod.rs`, add after the related-files route:

```rust
        .route("/v1/files/*path", get(documents::get_file))
```

And to `declared_routes()`:

```rust
        "GET /v1/files/*path",
```

- [ ] **Step 3: Add a unit test for the extension guard.** Add to the `tests` mod in `documents.rs`:

```rust
    #[test]
    fn ext_is_pdf_is_case_insensitive_and_rejects_others() {
        assert!(ext_is_pdf(StdPath::new("a/b/c.pdf")));
        assert!(ext_is_pdf(StdPath::new("a/b/c.PDF")));
        assert!(!ext_is_pdf(StdPath::new("a/b/c.txt")));
        assert!(!ext_is_pdf(StdPath::new("a/b/c")));
    }
```

> Note: `..`-traversal rejection is already covered by `rex-fs-local`'s `path_traversal_rejected` test (the same `safe_join` this handler relies on), so it is not re-tested here.

- [ ] **Step 4: Build + test.** Run: `cd server && cargo test -p rex-api`. Expected: PASS (all tests, incl. both new ones).

- [ ] **Step 5: Commit.**

```bash
git add server/crates/rex-api/src/handlers/documents.rs server/crates/rex-api/src/handlers/mod.rs
git commit -m "rex: add GET /v1/files/*path serve-by-path endpoint"
```

---

## Task 4: Client types + fetcher + path encoder

**Files:**
- Modify: `client/src/lib/types.ts`
- Modify: `client/src/lib/rex.ts`
- Modify: `client/src/lib/utils.ts`

- [ ] **Step 1: Add types.** Append to `types.ts`:

```ts
export interface RelatedFile {
  path: string;
  filename: string;
}

export interface RelatedFilesResponse {
  dir: string;
  files: RelatedFile[];
}
```

- [ ] **Step 2: Add the fetcher.** In `rex.ts`, add the import to the existing type import block (`RelatedFilesResponse`) and add after `tagValues`:

```ts
export function relatedFiles(docId: string): Promise<RelatedFilesResponse> {
  return request<RelatedFilesResponse>(
    `/v1/documents/${encodeURIComponent(docId)}/related-files`,
  );
}
```

- [ ] **Step 3: Add the path encoder.** Append to `utils.ts`:

```ts
/**
 * Encode a root-relative file path for the `/v1/files/*path` endpoint:
 * percent-encode each segment (spaces, parens, etc.) but keep the slashes
 * so the axum `*path` wildcard captures the full path.
 */
export function encodeFilePath(path: string): string {
  return path.split("/").map(encodeURIComponent).join("/");
}
```

- [ ] **Step 4: Verify lint/build.** Run: `cd client && pnpm lint`. Expected: no errors in the changed files.

- [ ] **Step 5: Commit.**

```bash
git add client/src/lib/types.ts client/src/lib/rex.ts client/src/lib/utils.ts
git commit -m "rex client: add relatedFiles fetcher, types, path encoder"
```

---

## Task 5: Render "Files in this folder" in the PDF viewer

**Files:**
- Modify: `client/src/components/pdf-viewer.tsx`

- [ ] **Step 1: Add imports.** Update the imports in `pdf-viewer.tsx`:

```ts
import { relatedFiles } from "@/lib/rex";
import { encodeFilePath } from "@/lib/utils";
import type { RelatedFile, SearchHit } from "@/lib/types";
```

- [ ] **Step 2: Add fetch state + effect.** Inside `PdfViewer`, after the `containerWidth` state (around line 38), add:

```ts
  const [related, setRelated] = useState<RelatedFile[]>([]);
```

And after the "Reset state when the hit changes" effect (around line 53), add:

```ts
  // Fetch sibling files in the question's folder (answer scheme + related papers).
  useEffect(() => {
    const ctrl = new AbortController();
    setRelated([]);
    relatedFiles(hit.document.id)
      .then((res) => {
        if (!ctrl.signal.aborted) setRelated(res.files);
      })
      .catch(() => {
        if (!ctrl.signal.aborted) setRelated([]);
      });
    return () => ctrl.abort();
  }, [hit.document.id]);
```

- [ ] **Step 3: Render the list.** Insert between the toolbar's closing `</div>` (line 177) and the `{/* Render area */}` comment (line 179):

```tsx
      {related.length > 0 && (
        <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted-foreground">
          <span className="text-foreground/70">Files in this folder:</span>
          {related.map((f) => (
            <a
              key={f.path}
              href={`/v1/files/${encodeFilePath(f.path)}`}
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-1 border-b border-accent text-primary hover:border-primary"
              onClick={() =>
                captureRexEvent("related_file_open", {
                  document_id: hit.document.id,
                  subject: hit.document.subject,
                  dir: f.path.split("/").slice(0, -1).join("/"),
                  filename: f.filename,
                })
              }
            >
              {f.filename} <ExternalLink className="size-3" />
            </a>
          ))}
        </div>
      )}
```

(`ExternalLink` and `captureRexEvent` are already imported.)

- [ ] **Step 4: Verify lint/build.** Run: `cd client && pnpm lint && pnpm build`. Expected: success, no type errors.

- [ ] **Step 5: Commit.**

```bash
git add client/src/components/pdf-viewer.tsx
git commit -m "rex client: show related files in PDF viewer"
```

---

## Task 6: Manual end-to-end verification

**Files:** none (verification only).

- [ ] **Step 1: Run the stack.** Start rex (`cd server && ./target/release/rex serve --db ./rex.db --docs-root <ren-subjects>/docs --bind 127.0.0.1:8080`) and the client (`cd client && pnpm dev`).

- [ ] **Step 2: Hit the API directly.** Pick a known doc id and run:
  `curl -s localhost:8080/v1/documents/<id>/related-files | jq` — expect a `files` array of sibling PDFs (e.g. the `..._Solutions.pdf`), not including the question's own PDF.

- [ ] **Step 3: Serve a sibling.** `curl -sI "localhost:8080/v1/files/<path-from-step-2>"` — expect `200` + `content-type: application/pdf`. Then `curl -sI "localhost:8080/v1/files/../etc/passwd"` — expect non-200.

- [ ] **Step 4: UI check.** In the client, search a subject, select a result, confirm "Files in this folder:" appears with the solutions link, and clicking opens the PDF in a new tab.

- [ ] **Step 5: Deploy note (manual).** Production `ren-prod-backend` (`i-0d8c7de2a048a4a69`) already mounts `--docs-root`, so deploy = rebuild the rex binary + restart the service. **No reindex, no DB migration.**

---

## Self-Review

- **Spec coverage:** related-files endpoint (Tasks 1–2), serve-by-path endpoint (Task 3), client types/fetcher/encoder (Task 4), viewer UI + PostHog (Task 5), tests + manual E2E (Tasks 1/3/6). All spec sections mapped.
- **Placeholders:** none — every code step is concrete.
- **Type consistency:** `RelatedFile{path,filename}` / `RelatedFilesResponse{dir,files}` identical across Rust (serde) and TS; `related_files_in_dir` / `get_related_files` / `get_file` / `ext_is_pdf` / `encodeFilePath` / `relatedFiles` names consistent across tasks.
