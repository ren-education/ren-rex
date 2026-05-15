//! Map a question's `source` markdown/txt path to its PDF path.

use std::path::PathBuf;

use rex_domain::{SourcePath, SubjectId};

/// Returns a PDF path *relative* to the docs-root (the form stored on PdfAnchor).
pub fn source_to_pdf_relative(subject: &SubjectId, source: &SourcePath) -> PathBuf {
    let rel = source.as_path();
    let stripped = rel.strip_prefix("content").unwrap_or(rel);
    let mut buf = PathBuf::from(stripped);
    buf.set_extension("pdf");
    PathBuf::from(subject.0.as_str()).join(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn md_source_maps_to_pdf() {
        let src = SourcePath::new("content/prelims/2019/HCI/X.md");
        let rel = source_to_pdf_relative(&SubjectId::new("h2physics"), &src);
        assert_eq!(rel, Path::new("h2physics/prelims/2019/HCI/X.pdf"));
    }

    #[test]
    fn txt_source_maps_to_pdf() {
        let src = SourcePath::new("content/holy-grail-sites/2023 - Essay X 11785.txt");
        let rel = source_to_pdf_relative(&SubjectId::new("h2history"), &src);
        assert_eq!(
            rel,
            Path::new("h2history/holy-grail-sites/2023 - Essay X 11785.pdf")
        );
    }
}
