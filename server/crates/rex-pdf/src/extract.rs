//! Page-level text extraction.

use std::panic::{catch_unwind, AssertUnwindSafe};

use bytes::Bytes;
use rex_domain::{Error, Result};

/// Extract `(page_number_1_indexed, page_text)` from PDF bytes.
///
/// Uses pdf-extract; pages are delimited by form-feed (\u{0C}) characters in
/// pdf-extract's whole-document output. We extract the full text once and
/// split on form-feed to get per-page texts.
///
/// **Panic safety:** pdf-extract uses several `assert!` macros internally
/// that panic on malformed-but-not-rejected PDFs (e.g., the
/// `name == "Identity-H"` assertion at lib.rs:942 fires on certain CID
/// fonts). One bad PDF in a corpus would otherwise abort an entire ingest
/// run, so we wrap the call in `catch_unwind` and convert panics into a
/// regular `Error::Pdf`. The caller's per-PDF fallback logic
/// (`fallback_reason: PdfReadFailed`) takes over from there.
///
/// `AssertUnwindSafe` is used because `Bytes` is not `UnwindSafe` by
/// default — and that's the right call here: we don't share mutable
/// state across the boundary, and pdf-extract has no observable side
/// effects we depend on after a panic.
pub fn extract_pages(bytes: &Bytes) -> Result<Vec<(u32, String)>> {
    let result = catch_unwind(AssertUnwindSafe(|| {
        pdf_extract::extract_text_from_mem(bytes)
    }));
    let text = match result {
        Ok(Ok(t)) => t,
        Ok(Err(e)) => {
            return Err(Error::Pdf {
                message: format!("pdf-extract failed: {e}"),
                path: None,
            });
        }
        Err(_) => {
            return Err(Error::Pdf {
                message: "pdf-extract panicked on this PDF (likely a malformed font or page table)"
                    .into(),
                path: None,
            });
        }
    };
    let pages: Vec<(u32, String)> = text
        .split('\u{000C}')
        .enumerate()
        .map(|(i, t)| ((i + 1) as u32, t.to_string()))
        .filter(|(_, t)| !t.trim().is_empty())
        .collect();
    Ok(pages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_corrupt_pdf_returns_err() {
        let junk = Bytes::from_static(b"this is not a pdf");
        let r = extract_pages(&junk);
        assert!(r.is_err());
    }
}
