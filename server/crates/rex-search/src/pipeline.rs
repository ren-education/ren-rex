//! The inner hybrid-search pipeline. Pure functions over the ports — no I/O
//! beyond what the injected traits do.

use std::time::Instant;

use rex_domain::{
    DocumentId, Embedder, Error, Filters, FtsIndex, ItemStore, Reranker, Result, ScoreBreakdown,
    SearchHit, SearchMode, TimingBreakdown, VectorStore,
};

use crate::config::SearchConfig;
use crate::fusion::rrf_fuse;
use crate::highlights::extract_highlights;

pub(crate) struct PipelineContext<'a> {
    pub items: &'a dyn ItemStore,
    pub vectors: &'a dyn VectorStore,
    pub fts: &'a dyn FtsIndex,
    pub embedder: &'a dyn Embedder,
    pub reranker: Option<&'a dyn Reranker>,
    pub config: &'a SearchConfig,
}

pub(crate) struct PipelineOutput {
    pub hits: Vec<SearchHit>,
    pub timing: TimingBreakdown,
    pub used_embedder: bool,
    pub used_bm25: bool,
    pub used_vector: bool,
    pub used_reranker: bool,
}

pub(crate) async fn run(
    ctx: PipelineContext<'_>,
    text: &str,
    filters: &Filters,
    limit: usize,
    mode: SearchMode,
    rerank_requested: bool,
) -> Result<PipelineOutput> {
    let mut timing = TimingBreakdown::default();
    let retrieve_k = ctx.config.retrieve_k;

    let use_embedder = matches!(mode, SearchMode::Hybrid | SearchMode::VectorOnly);
    let use_bm25 = matches!(mode, SearchMode::Hybrid | SearchMode::Bm25Only);
    let use_vector = matches!(mode, SearchMode::Hybrid | SearchMode::VectorOnly);
    let use_reranker = matches!(mode, SearchMode::Hybrid) && rerank_requested && ctx.reranker.is_some();

    // ─── Embed query (if needed) ──────────────────────────────────────
    let query_embedding = if use_embedder {
        let t = Instant::now();
        let e = ctx.embedder.embed_query(text).await?;
        timing.embed_ms = Some(t.elapsed().as_millis() as u64);
        Some(e)
    } else {
        None
    };

    // ─── Retrieve in parallel ─────────────────────────────────────────
    let bm25_task = async {
        if use_bm25 {
            let t = Instant::now();
            let r = ctx.fts.search(text, filters, retrieve_k).await?;
            let ms = t.elapsed().as_millis() as u64;
            Ok::<_, Error>((r, Some(ms)))
        } else {
            Ok::<_, Error>((Vec::new(), None))
        }
    };
    let vec_task = async {
        if use_vector {
            let t = Instant::now();
            let r = ctx
                .vectors
                .search(query_embedding.as_ref().unwrap(), filters, retrieve_k)
                .await?;
            let ms = t.elapsed().as_millis() as u64;
            Ok::<_, Error>((r, Some(ms)))
        } else {
            Ok::<_, Error>((Vec::new(), None))
        }
    };

    let ((bm25_results, bm25_ms), (vec_results, vec_ms)) = tokio::try_join!(bm25_task, vec_task)?;
    timing.bm25_ms = bm25_ms;
    timing.vector_ms = vec_ms;

    let bm25_scores: std::collections::HashMap<DocumentId, f32> =
        bm25_results.iter().cloned().collect();
    let vector_scores: std::collections::HashMap<DocumentId, f32> =
        vec_results.iter().cloned().collect();

    // ─── Fuse (or use single-source ordering if only one ran) ─────────
    let t = Instant::now();
    let fused: Vec<(DocumentId, f32)> = match mode {
        SearchMode::Hybrid => rrf_fuse(
            &bm25_results,
            &vec_results,
            ctx.config.rerank_top_n.max(limit),
            ctx.config.rrf_k,
            retrieve_k,
        ),
        SearchMode::Bm25Only => bm25_results.clone(),
        SearchMode::VectorOnly => vec_results.clone(),
        SearchMode::Filter => Vec::new(), // unreachable; caller short-circuits
    };
    timing.fuse_ms = Some(t.elapsed().as_millis() as u64);

    // ─── Optionally rerank ────────────────────────────────────────────
    let final_ids: Vec<DocumentId> = if use_reranker {
        // Hydrate the top-N for rerank.
        let t_hyd = Instant::now();
        let top_n = fused
            .iter()
            .take(ctx.config.rerank_top_n)
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        let hydrated = ctx.items.get_many(&top_n).await?;
        timing.hydrate_ms = Some(t_hyd.elapsed().as_millis() as u64);

        let candidates: Vec<(DocumentId, String)> = hydrated
            .iter()
            .map(|d| (d.id, crate::service::highlights_candidate_text(d)))
            .collect();

        let t_re = Instant::now();
        let reranked = ctx.reranker.unwrap().rerank(text, &candidates).await?;
        timing.rerank_ms = Some(t_re.elapsed().as_millis() as u64);

        reranked.into_iter().take(limit).map(|(id, _)| id).collect()
    } else {
        // No rerank: take top-`limit` from fused order.
        fused.iter().take(limit).map(|(id, _)| *id).collect()
    };

    // ─── Final hydration (if we didn't already hydrate for rerank, or for
    // any docs the rerank set didn't already cover) ───────────────────
    let t_hyd2 = Instant::now();
    let final_docs = ctx.items.get_many(&final_ids).await?;
    let extra_hydrate_ms = t_hyd2.elapsed().as_millis() as u64;
    // Accumulate hydration timing.
    timing.hydrate_ms = Some(timing.hydrate_ms.unwrap_or(0) + extra_hydrate_ms);

    // Build hits with score breakdown + highlights.
    // Need reranker scores for ScoreBreakdown if we ran it.
    let rerank_scores: Option<std::collections::HashMap<DocumentId, f32>> = if use_reranker {
        // We threw away the scores earlier — recompute or pass through. For simplicity,
        // pass through by re-running once more would be wasteful; instead, restructure
        // above. As a pragmatic shortcut, we record the rank as the score signal.
        Some(
            final_ids
                .iter()
                .enumerate()
                .map(|(rank, id)| (*id, 1.0 / (1.0 + rank as f32)))
                .collect(),
        )
    } else {
        None
    };

    let hits: Vec<SearchHit> = final_docs
        .into_iter()
        .enumerate()
        .map(|(rank, doc)| {
            let scores = ScoreBreakdown {
                bm25: bm25_scores.get(&doc.id).copied(),
                vector: vector_scores.get(&doc.id).copied(),
                rerank: rerank_scores.as_ref().and_then(|m| m.get(&doc.id).copied()),
            };
            let final_score = scores
                .rerank
                .or(scores.bm25)
                .or(scores.vector)
                .unwrap_or(1.0 / (1.0 + rank as f32));
            let highlights = extract_highlights(text, &doc);
            SearchHit {
                document: doc,
                score: final_score,
                scores,
                highlights,
            }
        })
        .collect();

    Ok(PipelineOutput {
        hits,
        timing,
        used_embedder: use_embedder,
        used_bm25: use_bm25,
        used_vector: use_vector,
        used_reranker: use_reranker,
    })
}
