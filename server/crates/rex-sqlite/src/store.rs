//! `SqliteStore` implementing ItemStore, FtsIndex, VectorStore.
//!
//! All three trait impls share a single `Arc<Mutex<Connection>>` because
//! rusqlite's `Connection` is not `Sync`. Lock scopes are tight (no awaits
//! while holding the lock).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use rex_domain::{
    BoundingBox, Document, DocumentId, DocumentKind, Embedding, Error, FacetCount,
    FallbackReason, Filters, FtsIndex, ItemStore, PdfAnchor, PdfSummary, Result, SourcePath,
    SubjectId, SubjectStats, TagField, TagValue, Tags, VectorStore,
};
use rusqlite::types::Value as SqlValue;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use uuid::Uuid;

use crate::sql::build_filter_fragment;

pub struct SqliteStore {
    conn: Arc<Mutex<Connection>>,
    vector_dim: usize,
}

impl SqliteStore {
    pub fn new(conn: Connection, vector_dim: usize) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
            vector_dim,
        }
    }

    pub fn from_arc(conn: Arc<Mutex<Connection>>, vector_dim: usize) -> Self {
        Self { conn, vector_dim }
    }

    fn ensure_subject(conn: &Connection, subject: &SubjectId) -> rusqlite::Result<()> {
        conn.execute(
            "INSERT OR IGNORE INTO subjects (id, created_at, item_count) VALUES (?, strftime('%s','now'), 0)",
            params![subject.0],
        )?;
        Ok(())
    }
}

fn map_err(e: rusqlite::Error) -> Error {
    Error::Storage {
        source: Box::new(e),
    }
}

fn row_to_document(row: &rusqlite::Row<'_>) -> rusqlite::Result<Document> {
    let id_s: String = row.get("id")?;
    let id = DocumentId(Uuid::parse_str(&id_s).map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))?);
    let subject: String = row.get("subject_id")?;
    let kind_s: String = row.get("kind")?;
    let kind = match kind_s.as_str() {
        "Question" => DocumentKind::Question,
        "Note" => DocumentKind::Note,
        _ => DocumentKind::Question,
    };
    let parent_id_s: Option<String> = row.get("parent_id")?;
    let parent_id = parent_id_s.and_then(|s| Uuid::parse_str(&s).ok().map(DocumentId));
    let depends_on_json: String = row.get("depends_on_json")?;
    let depends_on: Vec<DocumentId> = serde_json::from_str::<Vec<String>>(&depends_on_json)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|s| Uuid::parse_str(&s).ok().map(DocumentId))
        .collect();
    let number: Option<String> = row.get("number")?;
    let source_path: String = row.get("source_path")?;
    let context: Option<String> = row.get("context")?;
    let question: Option<String> = row.get("question")?;
    let answer: Option<String> = row.get("answer")?;
    let notes: Option<String> = row.get("notes")?;
    let mark: Option<u32> = row.get::<_, Option<i64>>("mark")?.map(|m| m as u32);
    let options_json: Option<String> = row.get("options_json")?;
    let options: Option<Vec<String>> =
        options_json.and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok());
    let keywords_json: String = row.get("keywords_json")?;
    let keywords: Vec<String> = serde_json::from_str(&keywords_json).unwrap_or_default();

    let pdf_path: Option<String> = row.get("pdf_path")?;
    let pdf_page: Option<i64> = row.get("pdf_page")?;
    let pdf_bbox_json: Option<String> = row.get("pdf_bbox_json")?;
    let pdf_confidence: Option<f64> = row.get("pdf_confidence")?;
    let pdf_fallback_reason: Option<String> = row.get("pdf_fallback_reason")?;
    let pdf_anchor = pdf_path.map(|p| PdfAnchor {
        pdf_path: PathBuf::from(p),
        page_number: pdf_page.map(|n| n as u32),
        bbox: pdf_bbox_json.and_then(|s| serde_json::from_str::<BoundingBox>(&s).ok()),
        confidence: pdf_confidence.unwrap_or(0.0) as f32,
        fallback_reason: pdf_fallback_reason
            .as_deref()
            .and_then(FallbackReason::from_str),
    });

    Ok(Document {
        id,
        subject: SubjectId::new(subject),
        kind,
        parent_id,
        depends_on,
        number,
        source: SourcePath::new(PathBuf::from(source_path)),
        context,
        question,
        answer,
        notes,
        mark,
        options,
        keywords,
        tags: Tags::default(), // populated by load_tags below
        pdf_anchor,
    })
}

fn load_tags_into(conn: &Connection, docs: &mut [Document]) -> rusqlite::Result<()> {
    if docs.is_empty() {
        return Ok(());
    }
    let mut by_id: std::collections::HashMap<String, &mut Document> = docs
        .iter_mut()
        .map(|d| (d.id.to_string(), d))
        .collect();
    let ids: Vec<String> = by_id.keys().cloned().collect();
    let placeholders = std::iter::repeat("?").take(ids.len()).collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT document_id, field, value FROM document_tags WHERE document_id IN ({placeholders})"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(ids.iter()), |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?))
    })?;
    for row in rows {
        let (doc_id, field, value) = row?;
        if let Some(doc) = by_id.get_mut(&doc_id) {
            match field.as_str() {
                "topics" => doc.tags.topics.push(TagValue::new(value)),
                "question_types" => doc.tags.question_types.push(TagValue::new(value)),
                "exam_systems" => doc.tags.exam_systems.push(TagValue::new(value)),
                "paper_types" => doc.tags.paper_types.push(TagValue::new(value)),
                "schools" => doc.tags.schools.push(TagValue::new(value)),
                "source_types" => doc.tags.source_types.push(TagValue::new(value)),
                _ => {}
            }
        }
    }
    Ok(())
}

#[async_trait]
impl ItemStore for SqliteStore {
    async fn put(&self, docs: &[Document]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction().map_err(map_err)?;
        for d in docs {
            Self::ensure_subject(&tx, &d.subject).map_err(map_err)?;
            let bbox_json = d
                .pdf_anchor
                .as_ref()
                .and_then(|a| a.bbox.as_ref())
                .map(|b| serde_json::to_string(b).unwrap_or_default());
            let options_json = d
                .options
                .as_ref()
                .map(|o| serde_json::to_string(o).unwrap_or("[]".into()));
            let keywords_json = serde_json::to_string(&d.keywords).unwrap_or("[]".into());
            let depends_on_json = serde_json::to_string(
                &d.depends_on.iter().map(|d| d.to_string()).collect::<Vec<_>>(),
            )
            .unwrap_or("[]".into());
            tx.execute(
                "INSERT OR REPLACE INTO documents (
                    id, subject_id, kind, parent_id, depends_on_json, number, source_path,
                    context, question, answer, notes, mark, options_json, keywords_json,
                    pdf_path, pdf_page, pdf_bbox_json, pdf_confidence, pdf_fallback_reason,
                    created_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, strftime('%s','now'))",
                params![
                    d.id.to_string(),
                    d.subject.0,
                    d.kind.as_str(),
                    d.parent_id.map(|p| p.to_string()),
                    depends_on_json,
                    d.number,
                    d.source.0.to_string_lossy().to_string(),
                    d.context,
                    d.question,
                    d.answer,
                    d.notes,
                    d.mark.map(|m| m as i64),
                    options_json,
                    keywords_json,
                    d.pdf_anchor.as_ref().map(|a| a.pdf_path.to_string_lossy().to_string()),
                    d.pdf_anchor.as_ref().and_then(|a| a.page_number.map(|n| n as i64)),
                    bbox_json,
                    d.pdf_anchor.as_ref().map(|a| a.confidence as f64),
                    d.pdf_anchor.as_ref().and_then(|a| a.fallback_reason.map(|r| r.as_str().to_string())),
                ],
            ).map_err(map_err)?;
            // Wipe and re-insert tags.
            tx.execute(
                "DELETE FROM document_tags WHERE document_id = ?",
                params![d.id.to_string()],
            )
            .map_err(map_err)?;
            for (field, value) in d.tags.flat() {
                tx.execute(
                    "INSERT OR IGNORE INTO document_tags (document_id, field, value) VALUES (?, ?, ?)",
                    params![d.id.to_string(), field.as_db_str(), value.0],
                )
                .map_err(map_err)?;
            }
        }
        // Update item_count.
        tx.execute(
            "UPDATE subjects SET item_count = (SELECT COUNT(*) FROM documents WHERE subject_id = subjects.id)",
            [],
        ).map_err(map_err)?;
        tx.commit().map_err(map_err)?;
        Ok(())
    }

    async fn get(&self, id: &DocumentId) -> Result<Option<Document>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT * FROM documents WHERE id = ?")
            .map_err(map_err)?;
        let mut doc_opt: Option<Document> = stmt
            .query_row(params![id.to_string()], row_to_document)
            .optional()
            .map_err(map_err)?;
        if let Some(doc) = doc_opt.as_mut() {
            let mut docs = std::slice::from_mut(doc);
            load_tags_into(&conn, &mut docs).map_err(map_err)?;
        }
        Ok(doc_opt)
    }

    async fn get_many(&self, ids: &[DocumentId]) -> Result<Vec<Document>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let conn = self.conn.lock().unwrap();
        let placeholders = std::iter::repeat("?").take(ids.len()).collect::<Vec<_>>().join(",");
        let sql = format!("SELECT * FROM documents WHERE id IN ({placeholders})");
        let mut stmt = conn.prepare(&sql).map_err(map_err)?;
        let id_strs: Vec<String> = ids.iter().map(|i| i.to_string()).collect();
        let mut docs: Vec<Document> = stmt
            .query_map(params_from_iter(id_strs.iter()), row_to_document)
            .map_err(map_err)?
            .filter_map(|r| r.ok())
            .collect();
        load_tags_into(&conn, &mut docs).map_err(map_err)?;
        // Preserve input order.
        let mut by_id: std::collections::HashMap<DocumentId, Document> =
            docs.into_iter().map(|d| (d.id, d)).collect();
        Ok(ids.iter().filter_map(|i| by_id.remove(i)).collect())
    }

    async fn query(
        &self,
        filters: &Filters,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Document>> {
        let frag = build_filter_fragment(filters, "d");
        let sql = format!(
            "SELECT * FROM documents d WHERE {} ORDER BY d.created_at, d.id LIMIT ? OFFSET ?",
            frag.where_sql
        );
        let conn = self.conn.lock().unwrap();
        let mut params = frag.params;
        params.push(SqlValue::Integer(limit as i64));
        params.push(SqlValue::Integer(offset as i64));
        let mut stmt = conn.prepare(&sql).map_err(map_err)?;
        let mut docs: Vec<Document> = stmt
            .query_map(params_from_iter(params.iter()), row_to_document)
            .map_err(map_err)?
            .filter_map(|r| r.ok())
            .collect();
        load_tags_into(&conn, &mut docs).map_err(map_err)?;
        Ok(docs)
    }

    async fn count(&self, filters: &Filters) -> Result<u64> {
        let frag = build_filter_fragment(filters, "d");
        let sql = format!("SELECT COUNT(*) FROM documents d WHERE {}", frag.where_sql);
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(&sql).map_err(map_err)?;
        let c: i64 = stmt
            .query_row(params_from_iter(frag.params.iter()), |r| r.get(0))
            .map_err(map_err)?;
        Ok(c as u64)
    }

    async fn list_subjects(&self) -> Result<Vec<SubjectStats>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT
                    s.id,
                    COALESCE(SUM(CASE WHEN d.kind='Question' THEN 1 ELSE 0 END), 0) AS q,
                    COALESCE(SUM(CASE WHEN d.kind='Note' THEN 1 ELSE 0 END), 0) AS n
                 FROM subjects s LEFT JOIN documents d ON d.subject_id = s.id
                 GROUP BY s.id ORDER BY s.id",
            )
            .map_err(map_err)?;
        let rows = stmt
            .query_map([], |r| {
                let id: String = r.get(0)?;
                let q: i64 = r.get(1)?;
                let n: i64 = r.get(2)?;
                Ok(SubjectStats {
                    id: SubjectId::new(id),
                    item_count: (q + n) as u64,
                    question_count: q as u64,
                    note_count: n as u64,
                })
            })
            .map_err(map_err)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    async fn list_topics(&self, subject: &SubjectId) -> Result<Vec<TagValue>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT DISTINCT dt.value FROM document_tags dt
                 JOIN documents d ON d.id = dt.document_id
                 WHERE dt.field = 'topics' AND d.subject_id = ?
                 ORDER BY dt.value",
            )
            .map_err(map_err)?;
        let rows = stmt
            .query_map([&subject.0], |r| {
                let v: String = r.get(0)?;
                Ok(TagValue::new(v))
            })
            .map_err(map_err)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    async fn facet_counts(
        &self,
        subject: &SubjectId,
        field: TagField,
        filters: &Filters,
    ) -> Result<Vec<FacetCount>> {
        let frag = build_filter_fragment(filters, "d");
        let sql = format!(
            "SELECT dt.value, COUNT(*) AS c
             FROM document_tags dt
             JOIN documents d ON d.id = dt.document_id
             WHERE dt.field = ? AND d.subject_id = ? AND {filter}
             GROUP BY dt.value ORDER BY c DESC, dt.value",
            filter = frag.where_sql,
        );
        let conn = self.conn.lock().unwrap();
        let mut params: Vec<SqlValue> = vec![
            SqlValue::Text(field.as_db_str().into()),
            SqlValue::Text(subject.0.clone()),
        ];
        params.extend(frag.params);
        let mut stmt = conn.prepare(&sql).map_err(map_err)?;
        let rows = stmt
            .query_map(params_from_iter(params.iter()), |r| {
                let v: String = r.get(0)?;
                let c: i64 = r.get(1)?;
                Ok(FacetCount {
                    value: TagValue::new(v),
                    count: c as u64,
                })
            })
            .map_err(map_err)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    async fn list_pdfs(&self, subject: &SubjectId) -> Result<Vec<PdfSummary>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT pdf_path,
                        COUNT(*) AS item_count,
                        SUM(CASE WHEN pdf_page IS NOT NULL THEN 1 ELSE 0 END) AS page_anchored_count
                 FROM documents
                 WHERE subject_id = ? AND pdf_path IS NOT NULL
                 GROUP BY pdf_path
                 ORDER BY pdf_path",
            )
            .map_err(map_err)?;
        let rows = stmt
            .query_map([&subject.0], |r| {
                let path: String = r.get(0)?;
                let item_count: i64 = r.get(1)?;
                let page_anchored: i64 = r.get(2)?;
                Ok(PdfSummary {
                    pdf_path: PathBuf::from(path),
                    item_count: item_count as u64,
                    page_anchored_count: page_anchored as u64,
                })
            })
            .map_err(map_err)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    async fn clear(&self, subject: &SubjectId) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction().map_err(map_err)?;
        // FK cascade handles document_tags, document_vec.
        tx.execute(
            "DELETE FROM documents WHERE subject_id = ?",
            params![subject.0],
        )
        .map_err(map_err)?;
        tx.execute(
            "DELETE FROM document_fts WHERE document_id IN (
                SELECT id FROM documents WHERE subject_id = ?)",
            params![subject.0],
        )
        .ok();
        tx.execute(
            "DELETE FROM subjects WHERE id = ?",
            params![subject.0],
        )
        .map_err(map_err)?;
        tx.commit().map_err(map_err)?;
        Ok(())
    }
}

// ─── FtsIndex ──────────────────────────────────────────────────────────

#[async_trait]
impl FtsIndex for SqliteStore {
    async fn upsert(&self, items: &[(DocumentId, String)]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction().map_err(map_err)?;
        for (id, text) in items {
            // Delete existing row first.
            tx.execute(
                "DELETE FROM document_fts WHERE document_id = ?",
                params![id.to_string()],
            )
            .map_err(map_err)?;
            tx.execute(
                "INSERT INTO document_fts (document_id, search_text) VALUES (?, ?)",
                params![id.to_string(), text],
            )
            .map_err(map_err)?;
        }
        tx.commit().map_err(map_err)?;
        Ok(())
    }

    async fn search(
        &self,
        query: &str,
        filters: &Filters,
        k: usize,
    ) -> Result<Vec<(DocumentId, f32)>> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }
        let frag = build_filter_fragment(filters, "d");
        let sql = format!(
            "WITH filtered AS (SELECT id FROM documents d WHERE {filter})
             SELECT f.document_id, bm25(document_fts) AS score
             FROM document_fts f
             WHERE document_fts MATCH ?
               AND f.document_id IN (SELECT id FROM filtered)
             ORDER BY score LIMIT ?",
            filter = frag.where_sql
        );
        let conn = self.conn.lock().unwrap();
        let mut params = frag.params;
        let sanitized = sanitize_fts_query(query);
        params.push(SqlValue::Text(sanitized));
        params.push(SqlValue::Integer(k as i64));
        let mut stmt = conn.prepare(&sql).map_err(map_err)?;
        let rows = stmt
            .query_map(params_from_iter(params.iter()), |r| {
                let id_s: String = r.get(0)?;
                let score: f64 = r.get(1)?;
                // bm25 returns negative scores; invert sign so higher = better.
                Ok((id_s, -score as f32))
            })
            .map_err(map_err)?;
        let mut out: Vec<(DocumentId, f32)> = rows
            .filter_map(|r| r.ok())
            .filter_map(|(s, score)| Uuid::parse_str(&s).ok().map(|u| (DocumentId(u), score)))
            .collect();
        // SQL ORDER BY score is ascending (most negative = best with bm25); we
        // flipped sign so re-sort descending.
        out.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(out)
    }

    async fn clear(&self, subject: &SubjectId) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM document_fts WHERE document_id IN (
                SELECT id FROM documents WHERE subject_id = ?)",
            params![subject.0],
        )
        .map_err(map_err)?;
        Ok(())
    }
}

/// FTS5 query sanitization. The goal is to accept arbitrary user text without
/// triggering FTS5 syntax errors, while still supporting explicit phrase
/// queries when the user passes them in via the `--exact` flag.
fn sanitize_fts_query(q: &str) -> String {
    // If the query is already a phrase-quoted string (likely from --exact),
    // strip the outer quotes and treat the contents as one phrase.
    let trimmed = q.trim();
    if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        let inner = &trimmed[1..trimmed.len() - 1];
        // Sanitize: keep alphanumerics + spaces, quote the whole thing as a
        // single FTS5 phrase.
        let cleaned: String = inner
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect();
        return format!("\"{}\"", cleaned);
    }
    // Otherwise: strip non-alphanumerics, lowercase, join with spaces. FTS5
    // treats space-separated tokens as implicit AND.
    let tokens: Vec<String> = trimmed
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_lowercase())
        .collect();
    tokens.join(" ")
}

// ─── VectorStore ───────────────────────────────────────────────────────

#[async_trait]
impl VectorStore for SqliteStore {
    async fn upsert(&self, items: &[(DocumentId, Embedding)]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction().map_err(map_err)?;
        for (id, emb) in items {
            if emb.dimension() != self.vector_dim {
                return Err(Error::bad_input(format!(
                    "embedding dimension {} does not match store dimension {}",
                    emb.dimension(),
                    self.vector_dim
                )));
            }
            let bytes = floats_to_bytes(emb.as_slice());
            // Look up subject_id to denormalize.
            let subj: String = tx
                .query_row(
                    "SELECT subject_id FROM documents WHERE id = ?",
                    params![id.to_string()],
                    |r| r.get(0),
                )
                .unwrap_or_default();
            tx.execute(
                "INSERT OR REPLACE INTO document_vec (document_id, subject_id, embedding)
                 VALUES (?, ?, ?)",
                params![id.to_string(), subj, bytes],
            )
            .map_err(map_err)?;
        }
        tx.commit().map_err(map_err)?;
        Ok(())
    }

    async fn search(
        &self,
        query: &Embedding,
        filters: &Filters,
        k: usize,
    ) -> Result<Vec<(DocumentId, f32)>> {
        if query.dimension() != self.vector_dim {
            return Err(Error::bad_input(format!(
                "query dimension {} does not match store dimension {}",
                query.dimension(),
                self.vector_dim
            )));
        }
        let frag = build_filter_fragment(filters, "d");
        let sql = format!(
            "SELECT v.document_id, v.embedding
             FROM document_vec v
             JOIN documents d ON d.id = v.document_id
             WHERE {filter}",
            filter = frag.where_sql
        );
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(&sql).map_err(map_err)?;
        let rows = stmt
            .query_map(params_from_iter(frag.params.iter()), |r| {
                let id_s: String = r.get(0)?;
                let bytes: Vec<u8> = r.get(1)?;
                Ok((id_s, bytes))
            })
            .map_err(map_err)?;

        let qv = query.as_slice();
        let mut scored: Vec<(DocumentId, f32)> = rows
            .filter_map(|r| r.ok())
            .filter_map(|(s, bytes)| {
                let id = Uuid::parse_str(&s).ok().map(DocumentId)?;
                let vec = bytes_to_floats(&bytes)?;
                if vec.len() != qv.len() {
                    return None;
                }
                Some((id, cosine(qv, &vec)))
            })
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(k);
        Ok(scored)
    }

    async fn clear(&self, subject: &SubjectId) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM document_vec WHERE subject_id = ?",
            params![subject.0],
        )
        .map_err(map_err)?;
        Ok(())
    }

    fn dimension(&self) -> usize {
        self.vector_dim
    }
}

fn floats_to_bytes(v: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(v.len() * 4);
    for x in v {
        out.extend_from_slice(&x.to_le_bytes());
    }
    out
}

fn bytes_to_floats(b: &[u8]) -> Option<Vec<f32>> {
    if b.len() % 4 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(b.len() / 4);
    for chunk in b.chunks_exact(4) {
        out.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    Some(out)
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
