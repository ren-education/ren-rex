//! 3-gram Jaccard similarity over normalized strings.

use std::collections::HashSet;

const N: usize = 3;

/// Normalize text: lowercase, strip non-alphanumeric, collapse whitespace.
fn normalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_space = false;
    for c in s.chars() {
        if c.is_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            last_space = false;
        } else if !last_space {
            out.push(' ');
            last_space = true;
        }
    }
    out.trim().into()
}

fn ngrams(s: &str) -> HashSet<[u8; N]> {
    let bytes = s.as_bytes();
    if bytes.len() < N {
        return HashSet::new();
    }
    let mut set = HashSet::with_capacity(bytes.len());
    for w in bytes.windows(N) {
        let mut arr = [0u8; N];
        arr.copy_from_slice(w);
        set.insert(arr);
    }
    set
}

/// Compute 3-gram Jaccard similarity between two strings.
pub fn ngram_jaccard(a: &str, b: &str) -> f32 {
    let a_norm = normalize(a);
    let b_norm = normalize(b);
    let a_set = ngrams(&a_norm);
    let b_set = ngrams(&b_norm);
    if a_set.is_empty() || b_set.is_empty() {
        return 0.0;
    }
    let inter = a_set.intersection(&b_set).count() as f32;
    let union = a_set.union(&b_set).count() as f32;
    inter / union
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_strings_is_one() {
        assert!((ngram_jaccard("hello world", "hello world") - 1.0).abs() < 1e-6);
    }

    #[test]
    fn disjoint_strings_is_zero() {
        let j = ngram_jaccard("aaaaaa", "bbbbbb");
        assert!(j < 0.05);
    }

    #[test]
    fn normalizes_case_and_punct() {
        // Same content with different punctuation/case should be very similar.
        let j = ngram_jaccard("Hello, World!", "hello world");
        assert!(j > 0.7);
    }
}
