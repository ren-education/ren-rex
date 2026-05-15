//! Pre-flight JSONL validator.
//!
//! Runs the same `JsonlRow::parse` + `into_document` pipeline as ingest, but
//! without touching the storage layer, without embedding, and without resolving
//! PDFs. Pure CPU + file I/O — typically sub-second on a 12k-row file.
//!
//! Errors are bucketed by *signature* (the serde error message with its
//! `at line N column M` suffix stripped) so a single systemic problem
//! (e.g. `options[*]` being objects instead of strings on every MCQ row) shows
//! up as one bucket of N rows rather than N look-alike log lines. This is the
//! same grouping trick Sentry uses for exception aggregation.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use rex_domain::{DocumentKind, SubjectId};
use serde::Serialize;

use crate::jsonl::JsonlRow;

/// Maximum number of example line numbers retained per error bucket. The full
/// count is always preserved; this just caps how many we print/serialize.
pub const MAX_SAMPLES_PER_SIGNATURE: usize = 5;

/// Result of validating a single JSONL file.
#[derive(Debug, Clone, Serialize)]
pub struct ValidateFileReport {
    pub path: PathBuf,
    pub kind: String,
    pub total_rows: u64,
    pub ok_rows: u64,
    pub failed_rows: u64,
    pub buckets: Vec<ErrorBucket>,
}

/// One distinct error signature observed in the file.
#[derive(Debug, Clone, Serialize)]
pub struct ErrorBucket {
    /// Normalised error message (no line/column suffix).
    pub signature: String,
    /// Best-effort name of the JSON field that triggered the error.
    /// `None` for domain-mapping errors (which print the field themselves).
    pub field: Option<String>,
    /// Total number of rows that failed with this signature.
    pub count: u64,
    /// First few line numbers (1-indexed) — capped at `MAX_SAMPLES_PER_SIGNATURE`.
    pub sample_lines: Vec<u64>,
}

impl ValidateFileReport {
    pub fn is_clean(&self) -> bool {
        self.failed_rows == 0
    }
}

/// Aggregate report across a subject's `questions.jsonl` + `notes.jsonl`.
#[derive(Debug, Clone, Serialize)]
pub struct ValidateSubjectReport {
    pub subject: String,
    pub workspace_root: PathBuf,
    pub files: Vec<ValidateFileReport>,
}

impl ValidateSubjectReport {
    pub fn is_clean(&self) -> bool {
        self.files.iter().all(|f| f.is_clean())
    }
}

/// Validate a single JSONL file at `path`. Returns an empty (clean) report if
/// the file does not exist — matching ingest's "missing file is fine" behaviour.
pub fn validate_file(
    path: &Path,
    kind: DocumentKind,
    subject: &SubjectId,
) -> std::io::Result<ValidateFileReport> {
    let mut report = ValidateFileReport {
        path: path.to_path_buf(),
        kind: format!("{:?}", kind),
        total_rows: 0,
        ok_rows: 0,
        failed_rows: 0,
        buckets: Vec::new(),
    };
    if !path.exists() {
        return Ok(report);
    }

    let mut by_sig: BTreeMap<(String, Option<String>), (u64, Vec<u64>)> = BTreeMap::new();
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut lineno: u64 = 0;
    for line in reader.lines() {
        let line = line?;
        lineno += 1;
        if line.trim().is_empty() {
            continue;
        }
        report.total_rows += 1;

        match JsonlRow::parse(&line) {
            Ok(row) => match row.into_document(subject.clone(), kind) {
                Ok(_) => report.ok_rows += 1,
                Err(domain_err) => {
                    let sig = normalise_domain_error(&domain_err);
                    record(&mut by_sig, sig, None, lineno);
                    report.failed_rows += 1;
                }
            },
            Err(serde_err) => {
                let column = serde_err.column();
                let sig = strip_position(&serde_err.to_string()).to_string();
                let field = field_at_byte_offset(&line, column);
                record(&mut by_sig, sig, field, lineno);
                report.failed_rows += 1;
            }
        }
    }

    report.buckets = by_sig
        .into_iter()
        .map(|((signature, field), (count, samples))| {
            let sample_lines = samples
                .into_iter()
                .take(MAX_SAMPLES_PER_SIGNATURE)
                .collect();
            ErrorBucket {
                signature,
                field,
                count,
                sample_lines,
            }
        })
        .collect();
    report.buckets.sort_by(|a, b| b.count.cmp(&a.count));
    Ok(report)
}

/// Validate the JSONL files for a single subject under `workspace_root`.
/// Mirrors the same paths ingest reads from:
///   `<workspace_root>/<subject>/reference/questions.jsonl`
///   `<workspace_root>/<subject>/reference/notes.jsonl`
pub fn validate_subject(
    subject: &SubjectId,
    workspace_root: &Path,
) -> std::io::Result<ValidateSubjectReport> {
    let dir = workspace_root.join(subject.0.as_str()).join("reference");
    let q_path = dir.join("questions.jsonl");
    let n_path = dir.join("notes.jsonl");
    let files = vec![
        validate_file(&q_path, DocumentKind::Question, subject)?,
        validate_file(&n_path, DocumentKind::Note, subject)?,
    ];
    Ok(ValidateSubjectReport {
        subject: subject.0.clone(),
        workspace_root: workspace_root.to_path_buf(),
        files,
    })
}

fn record(
    by_sig: &mut BTreeMap<(String, Option<String>), (u64, Vec<u64>)>,
    signature: String,
    field: Option<String>,
    lineno: u64,
) {
    let entry = by_sig.entry((signature, field)).or_default();
    entry.0 += 1;
    if entry.1.len() < MAX_SAMPLES_PER_SIGNATURE {
        entry.1.push(lineno);
    }
}

fn strip_position(msg: &str) -> &str {
    if let Some(idx) = msg.find(" at line ") {
        &msg[..idx]
    } else {
        msg
    }
}

fn normalise_domain_error(msg: &str) -> String {
    // Domain errors like `into_document` emits are formatted as
    // "invalid <field> <bad-value>: <inner cause>" — both the bad value AND
    // the inner cause vary per row, so naively keeping either splinters the
    // signature into hundreds of buckets. Collapse to "<field-prefix>: <value>".
    //
    // Recognised prefixes (extend here if `into_document` grows new error
    // shapes): "invalid id", "invalid parent_id".
    for prefix in ["invalid id ", "invalid parent_id "] {
        if let Some(rest) = msg.strip_prefix(prefix) {
            // Keep the inner-cause word (e.g., the part after `:`), since
            // "invalid id <uuid>: invalid length" and
            // "invalid id <uuid>: invalid character" are different bugs.
            let inner = rest.split(':').nth(1).map(|s| s.trim()).unwrap_or("");
            let trimmed_prefix = prefix.trim_end();
            return if inner.is_empty() {
                format!("{} <value>", trimmed_prefix)
            } else {
                format!("{} <value>: {}", trimmed_prefix, inner)
            };
        }
    }
    // Unknown shape: keep only the part before the first `:` to avoid
    // accidental cardinality from per-row detail.
    msg.split(':').next().unwrap_or(msg).trim().to_string()
}

/// Best-effort: given a one-line JSON document and a byte column (1-indexed),
/// return the nearest enclosing JSON object key. Walks backwards from
/// `column - 1` looking for the last `"<key>":` pattern that opened the value
/// containing this position. Handles nested objects by tracking brace depth.
///
/// This is intentionally heuristic — serde_json does not expose the JSON
/// pointer of the failure, and pulling in `serde_path_to_error` for this
/// would be heavier than needed. Wrong field names are surfaced as `None`
/// rather than misleading guesses by requiring a clean key-quote match.
fn field_at_byte_offset(line: &str, column: usize) -> Option<String> {
    let bytes = line.as_bytes();
    let upto = column.min(bytes.len()).saturating_sub(1);
    let head = &bytes[..upto];

    // Walk backwards. The first quote we hit is the *closing* of some string;
    // the next quote is its *opening*. If the closing quote is followed by `:`
    // (modulo whitespace) we've found a JSON key. Return the innermost
    // containing key — i.e., the one whose value brackets we crossed back out
    // of. Brace/bracket depth tracks that crossing.
    let mut depth: i32 = 0;
    let mut closing_quote: Option<usize> = None;
    let mut i = head.len();
    while i > 0 {
        i -= 1;
        let b = head[i];
        if let Some(close) = closing_quote {
            if b == b'"' && (i == 0 || head[i - 1] != b'\\') {
                let mut j = close + 1;
                while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                    j += 1;
                }
                let is_key = j < bytes.len() && bytes[j] == b':';
                if is_key {
                    if let Ok(s) = std::str::from_utf8(&head[i + 1..close]) {
                        return Some(s.to_string());
                    }
                }
                closing_quote = None;
            }
            continue;
        }
        match b {
            b'"' => closing_quote = Some(i),
            b'}' | b']' => depth += 1,
            b'{' | b'[' => depth -= 1,
            _ => {}
        }
        let _ = depth; // depth is informational; we always return innermost key
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_position_removes_serde_suffix() {
        assert_eq!(
            strip_position("invalid type: map, expected a string at line 1 column 472"),
            "invalid type: map, expected a string"
        );
        assert_eq!(strip_position("no position here"), "no position here");
    }

    #[test]
    fn field_lookup_finds_innermost_key() {
        // The inner `{` opens the value for the "options" array; failing at
        // that byte offset should attribute the error to "options".
        let line = r#"{"id":"x","options":[{"label":"A"}]}"#;
        let inner_brace = line.rfind('{').unwrap() + 1; // serde columns are 1-indexed
        assert_eq!(
            field_at_byte_offset(line, inner_brace).as_deref(),
            Some("options")
        );

        // Failure deeper inside should resolve to the innermost containing key.
        let label_value = line.rfind("\"A\"").unwrap() + 1;
        assert_eq!(
            field_at_byte_offset(line, label_value).as_deref(),
            Some("label")
        );
    }

    #[test]
    fn validate_file_buckets_systemic_failures() {
        use std::io::Write;
        let dir = std::env::temp_dir();
        let path = dir.join("rex-validate-test-buckets.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        // Two rows broken in the same way (`id` is required but missing);
        // one clean row. "missing field" errors have a constant signature
        // across rows (no payload values inlined), so they're the cleanest
        // illustration of bucketing. Type-mismatch errors bucket less
        // perfectly because serde inlines the bad value into the message.
        writeln!(f, r#"{{"source":"x.md","tags":{{}}}}"#).unwrap();
        writeln!(f, r#"{{"source":"y.md","tags":{{}}}}"#).unwrap();
        writeln!(f, r#"{{"id":"d95d6cc3-5f4e-41ed-9741-a14bad3b6322","source":"z.md","tags":{{}}}}"#).unwrap();
        let r = validate_file(&path, DocumentKind::Note, &SubjectId::new("test")).unwrap();
        assert_eq!(r.total_rows, 3);
        assert_eq!(r.ok_rows, 1);
        assert_eq!(r.failed_rows, 2);
        assert_eq!(r.buckets.len(), 1, "two same-signature failures should bucket together");
        assert_eq!(r.buckets[0].count, 2);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn validate_file_accepts_unknown_top_level_and_tag_fields() {
        // Policy: unknown fields are silently dropped. `rex validate` should
        // report 0 failures even when rob adds a field rex doesn't model.
        // This is the assertion that catches a future maintainer accidentally
        // re-introducing `#[serde(deny_unknown_fields)]`.
        use std::io::Write;
        let dir = std::env::temp_dir();
        let path = dir.join("rex-validate-test-drift.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, r#"{{"id":"d95d6cc3-5f4e-41ed-9741-a14bad3b6320","source":"x.md","tags":{{"brand_new_facet":["x"]}},"future_field":"y"}}"#).unwrap();
        let r = validate_file(&path, DocumentKind::Note, &SubjectId::new("test")).unwrap();
        assert_eq!(r.ok_rows, 1);
        assert_eq!(r.failed_rows, 0);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn missing_file_is_clean_empty_report() {
        let r = validate_file(
            Path::new("/tmp/definitely-not-here-12345.jsonl"),
            DocumentKind::Note,
            &SubjectId::new("test"),
        )
        .unwrap();
        assert_eq!(r.total_rows, 0);
        assert!(r.is_clean());
    }
}
