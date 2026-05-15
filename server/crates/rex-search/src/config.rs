//! Tunable knobs for the search pipeline. Defaults track spec §6.1.

#[derive(Debug, Clone, Copy)]
pub struct SearchConfig {
    /// Reciprocal Rank Fusion constant. Standard value from Cormack et al.
    pub rrf_k: u32,
    /// Top-K retrieved from each of BM25 and vector before fusion.
    pub retrieve_k: usize,
    /// Top-N candidates fed to the reranker (truncated post-fusion).
    pub rerank_top_n: usize,
    /// Max characters of query text accepted before truncation.
    pub max_query_text: usize,
    /// Minimum fuzzy confidence to consider a page-level PDF anchor. Used in
    /// rex-ingest, mirrored here so a future explainability hook can quote it.
    pub anchor_confidence_threshold: f32,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            rrf_k: 60,
            retrieve_k: 50,
            rerank_top_n: 20,
            max_query_text: 1024,
            anchor_confidence_threshold: 0.6,
        }
    }
}
