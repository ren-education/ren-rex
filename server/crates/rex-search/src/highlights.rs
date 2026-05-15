//! Term-based span highlighting over document fields.
//!
//! After ranking, for each hit we extract up to 3 ~120-char snippets per field
//! that contain any query term. Matched terms are wrapped in `<em>`.

use rex_domain::{Document, Highlight, HighlightField};

const STOPWORDS: &[&str] = &[
    "the", "a", "an", "of", "to", "for", "in", "is", "it", "and", "or", "on", "at", "by", "as",
    "be", "are", "was", "were", "from", "with", "this", "that", "these", "those",
];

const SNIPPET_RADIUS: usize = 60; // chars before + after the first match
const MAX_HIGHLIGHTS: usize = 3;

pub fn extract_highlights(query: &str, doc: &Document) -> Vec<Highlight> {
    let terms = tokenize_query(query);
    if terms.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::new();
    for (field, text) in [
        (HighlightField::Context, doc.context.as_deref()),
        (HighlightField::Question, doc.question.as_deref()),
        (HighlightField::Answer, doc.answer.as_deref()),
        (HighlightField::Notes, doc.notes.as_deref()),
    ] {
        if let Some(text) = text {
            for hl in extract_field(field, text, &terms) {
                out.push(hl);
                if out.len() >= MAX_HIGHLIGHTS {
                    return out;
                }
            }
        }
    }
    out
}

fn tokenize_query(q: &str) -> Vec<String> {
    q.split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() >= 3)
        .map(|t| t.to_lowercase())
        .filter(|t| !STOPWORDS.contains(&t.as_str()))
        .collect()
}

fn extract_field(field: HighlightField, text: &str, terms: &[String]) -> Vec<Highlight> {
    let lower = text.to_lowercase();
    let mut hits = Vec::new();
    for term in terms {
        let mut from = 0usize;
        while let Some(pos) = lower[from..].find(term.as_str()) {
            let abs = from + pos;
            hits.push((abs, abs + term.len()));
            from = abs + term.len().max(1);
            if hits.len() > 20 {
                break;
            }
        }
    }
    if hits.is_empty() {
        return Vec::new();
    }
    hits.sort();

    // Build snippets, merging overlapping match spans into one snippet.
    let mut snippets = Vec::new();
    let mut idx = 0usize;
    while idx < hits.len() && snippets.len() < MAX_HIGHLIGHTS {
        let (start, _end) = hits[idx];
        let snip_start = start.saturating_sub(SNIPPET_RADIUS);
        let snip_end = (start + SNIPPET_RADIUS).min(text.len());
        // Skip subsequent hits that fall within this snippet.
        let mut j = idx + 1;
        while j < hits.len() && hits[j].0 < snip_end {
            j += 1;
        }
        let snippet = render_snippet(text, snip_start, snip_end, terms);
        snippets.push(Highlight {
            field,
            text: snippet,
        });
        idx = j;
    }
    snippets
}

fn render_snippet(text: &str, start: usize, end: usize, terms: &[String]) -> String {
    // Snap to char boundaries (text is UTF-8; byte indices may be mid-char).
    let start = snap_to_char_boundary(text, start, false);
    let end = snap_to_char_boundary(text, end, true);
    let slice = &text[start..end];

    // Build a result string with case-insensitive term wrapping.
    let lower = slice.to_lowercase();
    let mut result = String::with_capacity(slice.len() + 16);
    let mut cursor = 0;
    while cursor < slice.len() {
        let remaining_lower = &lower[cursor..];
        if let Some((term, pos)) = terms
            .iter()
            .filter_map(|t| remaining_lower.find(t.as_str()).map(|p| (t, p)))
            .min_by_key(|(_, p)| *p)
        {
            let abs = cursor + pos;
            result.push_str(&slice[cursor..abs]);
            result.push_str("<em>");
            result.push_str(&slice[abs..abs + term.len()]);
            result.push_str("</em>");
            cursor = abs + term.len();
        } else {
            result.push_str(&slice[cursor..]);
            break;
        }
    }

    if start > 0 {
        result.insert_str(0, "…");
    }
    if end < text.len() {
        result.push('…');
    }
    result
}

fn snap_to_char_boundary(s: &str, idx: usize, ceiling: bool) -> usize {
    let mut i = idx.min(s.len());
    while i < s.len() && !s.is_char_boundary(i) {
        if ceiling {
            i += 1;
        } else {
            i = i.saturating_sub(1);
            if i == 0 {
                break;
            }
        }
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;
    use rex_domain::{
        Document, DocumentId, DocumentKind, SourcePath, SubjectId, Tags,
    };
    use uuid::Uuid;

    fn make_doc(question: &str) -> Document {
        Document {
            id: DocumentId(Uuid::new_v4()),
            subject: SubjectId::new("h2physics"),
            kind: DocumentKind::Question,
            parent_id: None,
            depends_on: vec![],
            number: None,
            source: SourcePath::new("x.md"),
            context: None,
            question: Some(question.into()),
            answer: None,
            notes: None,
            mark: None,
            options: None,
            keywords: vec![],
            tags: Tags::default(),
            pdf_anchor: None,
        }
    }

    #[test]
    fn highlights_finds_query_term_in_question() {
        let doc = make_doc("Explain why the tension in the cable is not equal to weight.");
        let h = extract_highlights("tension", &doc);
        assert!(!h.is_empty());
        assert!(h[0].text.contains("<em>tension</em>"));
        assert_eq!(h[0].field, HighlightField::Question);
    }

    #[test]
    fn highlights_skip_stopwords() {
        let doc = make_doc("The cable holds the weight in the air.");
        let h = extract_highlights("the cable", &doc);
        assert!(!h.is_empty());
        assert!(h[0].text.contains("<em>cable</em>"));
        assert!(!h[0].text.contains("<em>the</em>"));
    }

    #[test]
    fn highlights_capped_at_three_per_hit() {
        let text = "tension. tension. tension. tension. tension. tension.";
        let doc = make_doc(text);
        let h = extract_highlights("tension", &doc);
        assert!(h.len() <= MAX_HIGHLIGHTS);
    }

    #[test]
    fn no_match_returns_empty() {
        let doc = make_doc("Explain Newton's first law of motion.");
        let h = extract_highlights("photosynthesis", &doc);
        assert!(h.is_empty());
    }

    #[test]
    fn case_insensitive_matching() {
        let doc = make_doc("Tension is a force.");
        let h = extract_highlights("TENSION", &doc);
        assert!(!h.is_empty());
        assert!(h[0].text.contains("<em>Tension</em>"));
    }
}
