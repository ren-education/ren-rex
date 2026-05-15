//! Reciprocal Rank Fusion (RRF).
//!
//! Combines ranked lists from multiple retrievers without needing score
//! normalization. Documents appearing in multiple top-K lists are rewarded.

use std::collections::HashMap;

use rex_domain::DocumentId;

/// Fuse two ranked lists via RRF. Each list is (DocumentId, retriever_score).
/// The retriever_score is unused — only rank matters — but kept so the per-
/// retriever score can be surfaced in `ScoreBreakdown` later.
///
/// `k` is the RRF constant (typically 60). Missing rank treated as
/// `(retrieve_k + 1)` to give one-retriever-only docs a small but nonzero score.
pub fn rrf_fuse(
    bm25: &[(DocumentId, f32)],
    vector: &[(DocumentId, f32)],
    top_n: usize,
    k: u32,
    retrieve_k: usize,
) -> Vec<(DocumentId, f32)> {
    let missing_rank = (retrieve_k + 1) as f32;
    let mut scores: HashMap<DocumentId, f32> = HashMap::new();

    for (rank, (id, _)) in bm25.iter().enumerate() {
        let r = (rank + 1) as f32;
        let s = 1.0 / (k as f32 + r);
        *scores.entry(*id).or_insert(0.0) += s;
    }
    for (rank, (id, _)) in vector.iter().enumerate() {
        let r = (rank + 1) as f32;
        let s = 1.0 / (k as f32 + r);
        *scores.entry(*id).or_insert(0.0) += s;
    }

    // For documents that appear in only one list, add the "missing rank"
    // contribution from the absent list so the formula is symmetric.
    let bm25_ids: std::collections::HashSet<DocumentId> = bm25.iter().map(|(id, _)| *id).collect();
    let vec_ids: std::collections::HashSet<DocumentId> = vector.iter().map(|(id, _)| *id).collect();
    for id in bm25_ids.symmetric_difference(&vec_ids) {
        *scores.entry(*id).or_insert(0.0) += 1.0 / (k as f32 + missing_rank);
    }

    let mut ranked: Vec<(DocumentId, f32)> = scores.into_iter().collect();
    ranked.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.as_uuid().cmp(&b.0.as_uuid())) // stable tiebreaker
    });
    ranked.truncate(top_n);
    ranked
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn id(n: u8) -> DocumentId {
        DocumentId(Uuid::from_bytes([n; 16]))
    }

    #[test]
    fn doc_in_both_lists_dominates() {
        let bm25 = vec![(id(1), 10.0), (id(2), 5.0)];
        let vector = vec![(id(2), 0.9), (id(3), 0.7)];
        let r = rrf_fuse(&bm25, &vector, 10, 60, 50);
        // id(2) appears in both and should be on top.
        assert_eq!(r[0].0, id(2));
    }

    #[test]
    fn doc_in_only_one_list_still_scored() {
        let bm25 = vec![(id(1), 10.0)];
        let vector = vec![];
        let r = rrf_fuse(&bm25, &vector, 10, 60, 50);
        assert_eq!(r.len(), 1);
        assert!(r[0].1 > 0.0);
    }

    #[test]
    fn k_default_60_constant_present_in_signature() {
        // sanity: ensure callers can pass 60 and produce expected scoring.
        let bm25 = vec![(id(1), 1.0)];
        let vector = vec![(id(1), 1.0)];
        let r = rrf_fuse(&bm25, &vector, 10, 60, 50);
        // Both at rank 1, k=60: score = 2 * (1 / 61)
        let expected = 2.0 / 61.0;
        assert!((r[0].1 - expected).abs() < 1e-6);
    }

    #[test]
    fn empty_inputs_returns_empty() {
        let r = rrf_fuse(&[], &[], 10, 60, 50);
        assert!(r.is_empty());
    }

    #[test]
    fn ranking_deterministic_for_same_input() {
        let bm25 = vec![(id(1), 10.0), (id(2), 8.0), (id(3), 6.0)];
        let vector = vec![(id(3), 0.95), (id(2), 0.8), (id(1), 0.5)];
        let r1 = rrf_fuse(&bm25, &vector, 5, 60, 50);
        let r2 = rrf_fuse(&bm25, &vector, 5, 60, 50);
        assert_eq!(r1, r2);
    }

    #[test]
    fn top_n_truncates() {
        let bm25: Vec<_> = (1u8..=10).map(|n| (id(n), 10.0 - n as f32)).collect();
        let r = rrf_fuse(&bm25, &[], 3, 60, 50);
        assert_eq!(r.len(), 3);
    }
}
