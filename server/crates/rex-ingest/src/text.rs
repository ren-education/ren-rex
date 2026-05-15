//! Build the embedded/indexed text for a Document.

use rex_domain::Document;

/// Build the search text from a Document. Field prefixes are soft signals
/// to the embedder about field semantics.
pub fn build_search_text(doc: &Document) -> String {
    let mut s = String::new();
    if let Some(c) = &doc.context {
        s.push_str("Context: ");
        s.push_str(c);
        s.push('\n');
    }
    if let Some(q) = &doc.question {
        s.push_str("Question: ");
        s.push_str(q);
        s.push('\n');
    }
    if let Some(a) = &doc.answer {
        s.push_str("Answer: ");
        s.push_str(a);
        s.push('\n');
    }
    if let Some(n) = &doc.notes {
        s.push_str("Notes: ");
        s.push_str(n);
        s.push('\n');
    }
    if !doc.keywords.is_empty() {
        s.push_str("Keywords: ");
        s.push_str(&doc.keywords.join(", "));
    }
    s
}
