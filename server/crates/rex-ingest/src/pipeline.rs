//! End-to-end ingestion: stream JSONL → sanitize cross-refs → resolve PDF
//! anchors → embed → write.
//!
//! Pipeline is two-pass over the JSONL files:
//!   1. **Parse** both `questions.jsonl` and `notes.jsonl` into in-memory
//!      `Vec<Document>`s, collecting every successfully-parsed `DocumentId`
//!      into a `HashSet`. This is the *valid* set — anything not in it was
//!      skipped (usually a malformed UUID; see `rex validate` for a typed
//!      breakdown) and is therefore not safe to reference via FK.
//!   2. **Sanitize.** Walk the materialized docs once; null out any
//!      `parent_id` that points outside the valid set, and filter
//!      `depends_on` to only contain known ids. This is the cascade-skip
//!      fix for the FK constraint failure that bit the previous one-pass
//!      streaming design — see `IngestStats::dangling_parent_nulled` for
//!      the count surfaced to the operator.
//!   3. **Anchor + embed + write** in batches of `config.batch_size`. PDF
//!      panics are caught by `rex_pdf`; if extraction fails the doc still
//!      indexes with a file-only anchor.
//!
//! Materializing both files into memory costs ~tens of MB at h2physics's
//! scale (16k rows × ~few KB each), well within budget. If a subject ever
//! grows past a few hundred thousand rows we'd want to revisit — most
//! likely by writing to a staging table with FK off, then enforcing.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use rex_domain::{
    BlobStore, Document, DocumentId, DocumentKind, Embedder, FallbackReason, FtsIndex, ItemStore,
    PdfAnchor, SubjectId, VectorStore,
};
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::{info, warn};

use crate::config::IngestConfig;
use crate::error::IngestError;
use crate::jsonl::JsonlRow;
use crate::path_map::source_to_pdf_relative;
use crate::stats::IngestStats;
use crate::text::build_search_text;

pub struct IngestServices {
    pub items: Arc<dyn ItemStore>,
    pub vectors: Arc<dyn VectorStore>,
    pub fts: Arc<dyn FtsIndex>,
    pub blobs: Arc<dyn BlobStore>,
    pub embedder: Arc<dyn Embedder>,
}

/// Per-file parse result. Kept separate from the global `IngestStats` so the
/// `max_skip_pct` threshold can be evaluated per-file, matching the previous
/// streaming pipeline's semantics.
struct ParseResult {
    docs: Vec<Document>,
    total: u64,
    skipped: u64,
    sample_errors: Vec<String>,
}

pub async fn run(
    config: IngestConfig,
    services: IngestServices,
) -> Result<IngestStats, IngestError> {
    let started = Instant::now();
    let mut stats = IngestStats::default();

    if config.rebuild {
        services.items.clear(&config.subject).await?;
        services.vectors.clear(&config.subject).await?;
        services.fts.clear(&config.subject).await?;
    }

    let q_path = config
        .workspace_root
        .join(config.subject.0.as_str())
        .join("reference")
        .join("questions.jsonl");
    let n_path = config
        .workspace_root
        .join(config.subject.0.as_str())
        .join("reference")
        .join("notes.jsonl");

    if !q_path.exists() && !n_path.exists() {
        return Err(IngestError::MissingInput(format!(
            "no questions.jsonl or notes.jsonl found under {}",
            config.workspace_root.display()
        )));
    }

    // Phase 1: parse both files into memory.
    let q = parse_file(&q_path, DocumentKind::Question, &config.subject).await?;
    let n = parse_file(&n_path, DocumentKind::Note, &config.subject).await?;

    // Per-file skip-rate gate (preserves prior semantics).
    check_skip_threshold(&q, config.max_skip_pct, "questions.jsonl")?;
    check_skip_threshold(&n, config.max_skip_pct, "notes.jsonl")?;

    stats.rows_questions = q.total.saturating_sub(q.skipped);
    stats.rows_notes = n.total.saturating_sub(n.skipped);
    stats.rows_skipped = q.skipped + n.skipped;

    // Phase 2: sanitize cross-references against the union of valid ids.
    let mut q_docs = q.docs;
    let mut n_docs = n.docs;
    let valid: HashSet<DocumentId> = q_docs.iter().chain(n_docs.iter()).map(|d| d.id).collect();
    sanitize_refs(&mut q_docs, &valid, &mut stats);
    sanitize_refs(&mut n_docs, &valid, &mut stats);

    if stats.dangling_parent_nulled + stats.dangling_depends_pruned > 0 {
        info!(
            parent_nulled = stats.dangling_parent_nulled,
            depends_pruned = stats.dangling_depends_pruned,
            "nulled dangling cross-refs (parent or sibling ids that were themselves skipped)"
        );
    }

    // Phase 3: anchor + embed + write in batches.
    write_docs(q_docs, &config, &services, &mut stats).await?;
    write_docs(n_docs, &config, &services, &mut stats).await?;

    stats.took_ms = started.elapsed().as_millis() as u64;
    Ok(stats)
}

async fn parse_file(
    path: &std::path::Path,
    kind: DocumentKind,
    subject: &SubjectId,
) -> Result<ParseResult, IngestError> {
    let mut result = ParseResult {
        docs: Vec::new(),
        total: 0,
        skipped: 0,
        sample_errors: Vec::new(),
    };
    if !path.exists() {
        return Ok(result);
    }
    info!(path = %path.display(), kind = ?kind, "parsing");

    let f = File::open(path).await?;
    let mut reader = BufReader::new(f).lines();
    let mut lineno: u64 = 0;
    while let Some(line) = reader.next_line().await? {
        lineno += 1;
        if line.trim().is_empty() {
            continue;
        }
        result.total += 1;
        let row = match JsonlRow::parse(&line) {
            Ok(r) => r,
            Err(e) => {
                result.skipped += 1;
                if result.sample_errors.len() < 10 {
                    result.sample_errors.push(format!("line {lineno}: {e}"));
                }
                warn!(error = %e, "skipping malformed JSONL row");
                continue;
            }
        };
        match row.into_document(subject.clone(), kind) {
            Ok(d) => result.docs.push(d),
            Err(e) => {
                result.skipped += 1;
                if result.sample_errors.len() < 10 {
                    result.sample_errors.push(format!("line {lineno}: {e}"));
                }
                warn!(error = %e, "skipping row with bad domain mapping");
            }
        }
    }
    Ok(result)
}

fn check_skip_threshold(
    r: &ParseResult,
    threshold_pct: f64,
    label: &str,
) -> Result<(), IngestError> {
    if r.total == 0 {
        return Ok(());
    }
    let pct = (r.skipped as f64 / r.total as f64) * 100.0;
    if pct > threshold_pct {
        warn!(file = label, skipped = r.skipped, total = r.total, pct, "skip rate exceeded");
        return Err(IngestError::SchemaDrift {
            skipped: r.skipped,
            total: r.total,
            threshold_pct,
            sample: r.sample_errors.clone(),
        });
    }
    Ok(())
}

fn sanitize_refs(docs: &mut [Document], valid: &HashSet<DocumentId>, stats: &mut IngestStats) {
    for d in docs.iter_mut() {
        if let Some(pid) = d.parent_id {
            if !valid.contains(&pid) {
                d.parent_id = None;
                stats.dangling_parent_nulled += 1;
            }
        }
        let before = d.depends_on.len();
        d.depends_on.retain(|id| valid.contains(id));
        stats.dangling_depends_pruned += (before - d.depends_on.len()) as u64;
    }
}

async fn write_docs(
    docs: Vec<Document>,
    config: &IngestConfig,
    services: &IngestServices,
    stats: &mut IngestStats,
) -> Result<(), IngestError> {
    let mut batch: Vec<Document> = Vec::with_capacity(config.batch_size);
    for doc in docs.into_iter() {
        batch.push(doc);
        if batch.len() >= config.batch_size {
            flush_batch(&mut batch, config, services, stats).await?;
        }
    }
    if !batch.is_empty() {
        flush_batch(&mut batch, config, services, stats).await?;
    }
    Ok(())
}

async fn flush_batch(
    batch: &mut Vec<Document>,
    config: &IngestConfig,
    services: &IngestServices,
    stats: &mut IngestStats,
) -> Result<(), IngestError> {
    for doc in batch.iter_mut() {
        let pdf_rel = source_to_pdf_relative(&doc.subject, &doc.source);

        let exists = services.blobs.exists(&pdf_rel).await.unwrap_or(false);
        if !exists {
            doc.pdf_anchor = Some(PdfAnchor {
                pdf_path: pdf_rel,
                page_number: None,
                bbox: None,
                confidence: 0.0,
                fallback_reason: Some(FallbackReason::PdfNotFound),
            });
            stats.pdfs_not_found += 1;
            continue;
        }
        stats.pdfs_seen += 1;
        let bytes_res = services.blobs.get(&pdf_rel).await;
        let bytes = match bytes_res {
            Ok(b) => b,
            Err(e) => {
                warn!(path = %pdf_rel.display(), error = %e, "blob get failed");
                doc.pdf_anchor = Some(PdfAnchor {
                    pdf_path: pdf_rel,
                    page_number: None,
                    bbox: None,
                    confidence: 0.0,
                    fallback_reason: Some(FallbackReason::PdfReadFailed),
                });
                stats.pdfs_read_failed += 1;
                continue;
            }
        };
        match rex_pdf::extract_pages(&bytes) {
            Ok(pages) => {
                let target = format!(
                    "{} {}",
                    doc.context.as_deref().unwrap_or(""),
                    doc.question.as_deref().unwrap_or(""),
                );
                let (page, score) = rex_pdf::fuzzy_match_page(
                    &target,
                    &pages,
                    config.anchor_confidence_threshold,
                );
                if page.is_some() {
                    doc.pdf_anchor = Some(PdfAnchor {
                        pdf_path: pdf_rel,
                        page_number: page,
                        bbox: None,
                        confidence: score,
                        fallback_reason: None,
                    });
                    stats.pdfs_anchored += 1;
                } else {
                    doc.pdf_anchor = Some(PdfAnchor {
                        pdf_path: pdf_rel,
                        page_number: None,
                        bbox: None,
                        confidence: score,
                        fallback_reason: Some(FallbackReason::LowConfidence),
                    });
                    stats.pdfs_low_confidence += 1;
                }
            }
            Err(e) => {
                warn!(path = %pdf_rel.display(), error = %e, "pdf extract failed");
                doc.pdf_anchor = Some(PdfAnchor {
                    pdf_path: pdf_rel,
                    page_number: None,
                    bbox: None,
                    confidence: 0.0,
                    fallback_reason: Some(FallbackReason::PdfReadFailed),
                });
                stats.pdfs_read_failed += 1;
            }
        }
    }

    let texts: Vec<String> = batch.iter().map(build_search_text).collect();
    let embeddings = services.embedder.embed_documents(&texts).await?;

    services.items.put(batch).await?;
    let fts_items: Vec<_> = batch
        .iter()
        .zip(texts.iter().cloned())
        .map(|(d, t)| (d.id, t))
        .collect();
    services.fts.upsert(&fts_items).await?;
    let vec_items: Vec<_> = batch
        .iter()
        .zip(embeddings.into_iter())
        .map(|(d, e)| (d.id, e))
        .collect();
    services.vectors.upsert(&vec_items).await?;

    batch.clear();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rex_domain::{Document, DocumentId, DocumentKind, SourcePath, SubjectId, Tags};
    use std::path::PathBuf;
    use uuid::Uuid;

    fn doc(id_str: &str, parent_id: Option<&str>, depends_on: Vec<&str>) -> Document {
        Document {
            id: DocumentId(Uuid::parse_str(id_str).unwrap()),
            subject: SubjectId::new("test"),
            kind: DocumentKind::Question,
            parent_id: parent_id.map(|s| DocumentId(Uuid::parse_str(s).unwrap())),
            depends_on: depends_on
                .into_iter()
                .map(|s| DocumentId(Uuid::parse_str(s).unwrap()))
                .collect(),
            number: None,
            source: SourcePath::new(PathBuf::from("x.md")),
            context: None,
            question: None,
            answer: None,
            notes: None,
            mark: None,
            options: None,
            keywords: vec![],
            tags: Tags::default(),
            pdf_anchor: None,
        }
    }

    const A: &str = "11111111-1111-4111-8111-111111111111";
    const B: &str = "22222222-2222-4222-8222-222222222222";
    const GHOST: &str = "99999999-9999-4999-8999-999999999999";

    #[test]
    fn sanitize_nulls_dangling_parent_id() {
        let mut docs = vec![doc(B, Some(GHOST), vec![])];
        let valid: HashSet<DocumentId> = docs.iter().map(|d| d.id).collect();
        let mut stats = IngestStats::default();
        sanitize_refs(&mut docs, &valid, &mut stats);
        assert!(docs[0].parent_id.is_none(), "ghost parent should be nulled");
        assert_eq!(stats.dangling_parent_nulled, 1);
    }

    #[test]
    fn sanitize_keeps_valid_parent_id() {
        let mut docs = vec![doc(A, None, vec![]), doc(B, Some(A), vec![])];
        let valid: HashSet<DocumentId> = docs.iter().map(|d| d.id).collect();
        let mut stats = IngestStats::default();
        sanitize_refs(&mut docs, &valid, &mut stats);
        assert_eq!(
            docs[1].parent_id,
            Some(DocumentId(Uuid::parse_str(A).unwrap())),
            "valid parent should survive"
        );
        assert_eq!(stats.dangling_parent_nulled, 0);
    }

    #[test]
    fn sanitize_prunes_dangling_depends_on() {
        let mut docs = vec![doc(A, None, vec![]), doc(B, None, vec![A, GHOST])];
        let valid: HashSet<DocumentId> = docs.iter().map(|d| d.id).collect();
        let mut stats = IngestStats::default();
        sanitize_refs(&mut docs, &valid, &mut stats);
        assert_eq!(docs[1].depends_on.len(), 1, "ghost dep should be pruned");
        assert_eq!(stats.dangling_depends_pruned, 1);
    }
}
