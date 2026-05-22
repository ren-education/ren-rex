//! Fuzzy page-anchor resolution: given a question's target text and a set of
//! per-page texts, pick the page with the highest n-gram overlap.

use crate::ngram::ngram_jaccard;

/// Resolve a question's PDF page by 3-gram Jaccard similarity.
///
/// Returns `(Some(page), score)` if best match >= `confidence_threshold`.
/// Returns `(None, score)` otherwise (file-level fallback signal).
pub fn fuzzy_match_page(
    target: &str,
    pages: &[(u32, String)],
    confidence_threshold: f32,
) -> (Option<u32>, f32) {
    // Truncate to ~500 bytes for n-gram perf, but never split a UTF-8 char:
    // chemistry content (`dm⁻³`, `Δ`) places multi-byte chars at arbitrary
    // offsets, so a raw `&target[..500]` can land inside a char and panic.
    let target = if target.len() > 500 {
        let mut end = 500;
        while !target.is_char_boundary(end) {
            end -= 1;
        }
        &target[..end]
    } else {
        target
    };
    let mut best: (Option<u32>, f32) = (None, 0.0);
    for (page, text) in pages {
        let score = ngram_jaccard(target, text);
        if score > best.1 {
            best = (Some(*page), score);
        }
    }
    if best.1 >= confidence_threshold {
        best
    } else {
        (None, best.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_correct_page_for_known_question() {
        let pages = vec![
            (1u32, "Welcome to the document. This is some intro text.".into()),
            (
                2,
                "Question 1: Explain why the tension in the cable is not equal to weight."
                    .into(),
            ),
            (3, "Final remarks and acknowledgements.".into()),
        ];
        let (page, score) = fuzzy_match_page(
            "Explain why the tension in the cable is not equal to weight.",
            &pages,
            0.6,
        );
        assert_eq!(page, Some(2));
        assert!(score >= 0.6);
    }

    #[test]
    fn handles_multibyte_char_straddling_truncation_boundary() {
        // 499 ASCII bytes + a 2-byte char ('³') => byte 500 falls inside the
        // char. A raw `&target[..500]` panics; truncation must floor to a
        // char boundary. Regression test for the h2chemistry ingest crash.
        let target = format!("{}³ and some trailing question text here", "a".repeat(499));
        assert!(target.len() > 500);
        let pages = vec![(1u32, "unrelated page text".to_string())];
        // Must not panic.
        let _ = fuzzy_match_page(&target, &pages, 0.6);
    }

    #[test]
    fn returns_none_when_below_threshold() {
        let pages = vec![
            (1u32, "completely unrelated content here".into()),
            (2, "more unrelated stuff".into()),
        ];
        let (page, _) = fuzzy_match_page(
            "Explain why the tension in the cable is not equal to weight.",
            &pages,
            0.6,
        );
        assert_eq!(page, None);
    }
}
