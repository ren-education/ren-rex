//! Embedding newtype with dimension checking.

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Embedding(Vec<f32>);

impl Embedding {
    /// Construct an embedding, asserting the expected dimension.
    pub fn new(expected_dim: usize, v: Vec<f32>) -> Result<Self> {
        if v.len() != expected_dim {
            return Err(Error::BadInput {
                message: format!(
                    "embedding dimension mismatch: expected {}, got {}",
                    expected_dim,
                    v.len()
                ),
                field: Some("embedding".into()),
            });
        }
        Ok(Self(v))
    }

    /// Construct without dimension check (use sparingly; only when the dimension
    /// is established elsewhere). Prefer `new` at trait boundaries.
    pub fn from_vec_unchecked(v: Vec<f32>) -> Self {
        Self(v)
    }

    pub fn dimension(&self) -> usize {
        self.0.len()
    }

    pub fn as_slice(&self) -> &[f32] {
        &self.0
    }

    pub fn into_vec(self) -> Vec<f32> {
        self.0
    }
}
