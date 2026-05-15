//! Deterministic stub embedder + reranker.
//!
//! Same input → same output. Different inputs share weak similarity based on
//! token overlap. Used for v1 fallback when GGUF models are unavailable, and
//! as the default in tests.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use async_trait::async_trait;
use rex_domain::{DocumentId, Embedder, Embedding, Error, Reranker, Result};

pub struct StubEmbedder {
    dim: usize,
}

impl StubEmbedder {
    pub fn new(dim: usize) -> Self {
        Self { dim }
    }

    fn embed_text(&self, text: &str) -> Vec<f32> {
        // Build a token-bag hash: tokens contribute independently so similar
        // texts get similar vectors. Each token seeds a few dimensions.
        let mut v = vec![0.0f32; self.dim];
        for tok in tokenize(text) {
            let mut hasher = DefaultHasher::new();
            tok.hash(&mut hasher);
            let h = hasher.finish();
            // Each token sets `spread` distinct dimensions to +/- 1.
            let spread = 4u64;
            for i in 0..spread {
                let idx = ((h.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(i)) as usize)
                    % self.dim;
                let sign = if (h >> (i + 1)) & 1 == 0 { 1.0 } else { -1.0 };
                v[idx] += sign;
            }
        }
        // L2 normalize so dot product = cosine similarity.
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut v {
                *x /= norm;
            }
        }
        v
    }
}

#[async_trait]
impl Embedder for StubEmbedder {
    async fn embed_query(&self, text: &str) -> Result<Embedding> {
        Embedding::new(self.dim, self.embed_text(text))
    }

    async fn embed_documents(&self, texts: &[String]) -> Result<Vec<Embedding>> {
        let mut out = Vec::with_capacity(texts.len());
        for t in texts {
            out.push(Embedding::new(self.dim, self.embed_text(t))?);
        }
        Ok(out)
    }

    fn dimension(&self) -> usize {
        self.dim
    }
}

/// Stub reranker: scores by token-overlap fraction of the query against the candidate.
/// Stable: identical inputs produce identical scores.
pub struct StubReranker;

#[async_trait]
impl Reranker for StubReranker {
    async fn rerank(
        &self,
        query: &str,
        candidates: &[(DocumentId, String)],
    ) -> Result<Vec<(DocumentId, f32)>> {
        let q: std::collections::HashSet<String> = tokenize(query).collect();
        let mut scored: Vec<_> = candidates
            .iter()
            .map(|(id, text)| {
                let ts: std::collections::HashSet<String> = tokenize(text).collect();
                let inter = q.intersection(&ts).count() as f32;
                let union = q.union(&ts).count().max(1) as f32;
                (*id, inter / union)
            })
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(scored)
    }
}

/// No-op reranker: returns candidates in input order with uniform score 0.5.
pub struct NoOpReranker;

#[async_trait]
impl Reranker for NoOpReranker {
    async fn rerank(
        &self,
        _query: &str,
        candidates: &[(DocumentId, String)],
    ) -> Result<Vec<(DocumentId, f32)>> {
        Ok(candidates.iter().map(|(id, _)| (*id, 0.5)).collect())
    }
}

fn tokenize(s: &str) -> impl Iterator<Item = String> + '_ {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() >= 2)
        .map(|t| t.to_lowercase())
}

// Silence the unused-import warning on Error in some configurations.
#[allow(dead_code)]
fn _force_error_use(e: Error) -> Error {
    e
}
