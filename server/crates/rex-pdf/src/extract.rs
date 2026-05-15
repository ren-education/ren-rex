//! Page-level text extraction.

use bytes::Bytes;
use rex_domain::{Error, Result};

/// Extract `(page_number_1_indexed, page_text)` from PDF bytes.
///
/// Uses pdf-extract; pages are delimited by form-feed (\u{0C}) characters in
/// pdf-extract's whole-document output. We extract the full text once and
/// split on form-feed to get per-page texts.
pub fn extract_pages(bytes: &Bytes) -> Result<Vec<(u32, String)>> {
    let text = pdf_extract::extract_text_from_mem(bytes).map_err(|e| Error::Pdf {
        message: format!("pdf-extract failed: {e}"),
        path: None,
    })?;
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
