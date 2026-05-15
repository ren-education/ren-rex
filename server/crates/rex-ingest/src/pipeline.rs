//! End-to-end ingestion: stream JSONL → resolve PDF anchors → embed → write.

use std::sync::Arc;
use std::time::Instant;

use rex_domain::{
    BlobStore, Document, DocumentKind, Embedder, FallbackReason, FtsIndex, ItemStore, PdfAnchor,
    VectorStore,
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

    process_file(
        &q_path,
        DocumentKind::Question,
        &config,
        &services,
        &mut stats,
    )
    .await?;
    process_file(
        &n_path,
        DocumentKind::Note,
        &config,
        &services,
        &mut stats,
    )
    .await?;

    stats.took_ms = started.elapsed().as_millis() as u64;
    Ok(stats)
}

async fn process_file(
    path: &std::path::Path,
    kind: DocumentKind,
    config: &IngestConfig,
    services: &IngestServices,
    stats: &mut IngestStats,
) -> Result<(), IngestError> {
    if !path.exists() {
        return Ok(());
    }
    info!(path = %path.display(), kind = ?kind, "ingesting");

    let f = File::open(path).await?;
    let mut reader = BufReader::new(f).lines();

    let mut batch: Vec<Document> = Vec::with_capacity(config.batch_size);
    let mut sample_errors: Vec<String> = Vec::new();
    let mut local_total: u64 = 0;
    let mut local_skipped: u64 = 0;

    while let Some(line) = reader.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        local_total += 1;
        let row = match JsonlRow::parse(&line) {
            Ok(r) => r,
            Err(e) => {
                local_skipped += 1;
                if sample_errors.len() < 10 {
                    sample_errors.push(format!("line {}: {}", local_total, e));
                }
                warn!(error = %e, "skipping malformed JSONL row");
                continue;
            }
        };
        let doc = match row.into_document(config.subject.clone(), kind) {
            Ok(d) => d,
            Err(e) => {
                local_skipped += 1;
                if sample_errors.len() < 10 {
                    sample_errors.push(format!("line {}: {}", local_total, e));
                }
                warn!(error = %e, "skipping row with bad domain mapping");
                continue;
            }
        };
        batch.push(doc);
        if batch.len() >= config.batch_size {
            flush_batch(&mut batch, config, services, stats).await?;
        }
    }
    if !batch.is_empty() {
        flush_batch(&mut batch, config, services, stats).await?;
    }

    match kind {
        DocumentKind::Question => stats.rows_questions += local_total - local_skipped,
        DocumentKind::Note => stats.rows_notes += local_total - local_skipped,
    }
    stats.rows_skipped += local_skipped;

    if local_total > 0 {
        let pct = (local_skipped as f64 / local_total as f64) * 100.0;
        if pct > config.max_skip_pct {
            return Err(IngestError::SchemaDrift {
                skipped: local_skipped,
                total: local_total,
                threshold_pct: config.max_skip_pct,
                sample: sample_errors,
            });
        }
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
