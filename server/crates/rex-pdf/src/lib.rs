//! rex-pdf: PDF page text extraction + fuzzy page anchoring.
//!
//! Uses the pure-Rust `pdf-extract` crate so the build has no native
//! dependencies. PDFs with complex layout may extract imperfectly, which is
//! fine — fuzzy matching tolerates noise.

mod extract;
mod ngram;
mod anchor;

pub use anchor::fuzzy_match_page;
pub use extract::extract_pages;
pub use ngram::ngram_jaccard;
