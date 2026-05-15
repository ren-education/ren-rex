//! PDF anchor types. Each Document optionally carries a PdfAnchor pointing to
//! the source PDF page.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PdfAnchor {
    /// PDF path relative to the configured BlobStore root.
    pub pdf_path: PathBuf,
    /// One-indexed PDF page number. `None` if the fuzzy match was below
    /// the confidence threshold or the PDF could not be read.
    pub page_number: Option<u32>,
    /// Optional bounding box on the page. Rarely populated in v1.
    pub bbox: Option<BoundingBox>,
    /// Fuzzy-match confidence in [0.0, 1.0]. `0.0` if no match was attempted.
    pub confidence: f32,
    /// Why we fell back to file-level (or worse). `None` means an exact
    /// page anchor was resolved successfully.
    pub fallback_reason: Option<FallbackReason>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FallbackReason {
    /// Fuzzy match was below the confidence threshold.
    LowConfidence,
    /// BlobStore::get or PDF parsing errored.
    PdfReadFailed,
    /// The derived PDF path does not exist in the blob store.
    PdfNotFound,
}

impl FallbackReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            FallbackReason::LowConfidence => "LowConfidence",
            FallbackReason::PdfReadFailed => "PdfReadFailed",
            FallbackReason::PdfNotFound => "PdfNotFound",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "LowConfidence" => Some(FallbackReason::LowConfidence),
            "PdfReadFailed" => Some(FallbackReason::PdfReadFailed),
            "PdfNotFound" => Some(FallbackReason::PdfNotFound),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}
