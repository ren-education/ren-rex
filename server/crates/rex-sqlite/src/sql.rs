//! Build SQL WHERE fragments + bound parameters from `Filters`.

use rex_domain::{DocumentKind, Filters};
use rusqlite::types::Value as SqlValue;

pub struct FilterFragment {
    /// SQL fragment usable as an additional WHERE clause (without the leading WHERE).
    /// References table alias `d` for documents and uses correlated EXISTS for tags.
    pub where_sql: String,
    pub params: Vec<SqlValue>,
}

pub fn build_filter_fragment(f: &Filters, doc_alias: &str) -> FilterFragment {
    let mut conds: Vec<String> = Vec::new();
    let mut params: Vec<SqlValue> = Vec::new();

    if let Some(s) = &f.subject {
        conds.push(format!("{doc_alias}.subject_id = ?"));
        params.push(SqlValue::Text(s.0.clone()));
    }
    if let Some(k) = f.kind {
        let s = match k {
            DocumentKind::Question => "Question",
            DocumentKind::Note => "Note",
        };
        conds.push(format!("{doc_alias}.kind = ?"));
        params.push(SqlValue::Text(s.into()));
    }
    if let Some((lo, hi)) = f.marks_range {
        conds.push(format!(
            "{doc_alias}.mark IS NOT NULL AND {doc_alias}.mark >= ? AND {doc_alias}.mark <= ?"
        ));
        params.push(SqlValue::Integer(lo as i64));
        params.push(SqlValue::Integer(hi as i64));
    }

    add_tag_or_filter(&mut conds, &mut params, "topics", &f.topics, doc_alias);
    add_tag_or_filter(
        &mut conds,
        &mut params,
        "question_types",
        &f.question_types,
        doc_alias,
    );
    add_tag_or_filter(&mut conds, &mut params, "paper_types", &f.paper_types, doc_alias);
    add_tag_or_filter(&mut conds, &mut params, "schools", &f.schools, doc_alias);
    add_tag_or_filter(
        &mut conds,
        &mut params,
        "source_types",
        &f.source_types,
        doc_alias,
    );
    add_tag_or_filter(
        &mut conds,
        &mut params,
        "exam_systems",
        &f.exam_systems,
        doc_alias,
    );

    let where_sql = if conds.is_empty() {
        "1=1".into()
    } else {
        conds.join(" AND ")
    };
    FilterFragment { where_sql, params }
}

fn add_tag_or_filter(
    conds: &mut Vec<String>,
    params: &mut Vec<SqlValue>,
    field: &str,
    values: &[rex_domain::TagValue],
    doc_alias: &str,
) {
    if values.is_empty() {
        return;
    }
    let placeholders = std::iter::repeat("?")
        .take(values.len())
        .collect::<Vec<_>>()
        .join(",");
    conds.push(format!(
        "EXISTS (SELECT 1 FROM document_tags dt WHERE dt.document_id = {doc_alias}.id \
         AND dt.field = ? AND dt.value IN ({placeholders}))"
    ));
    params.push(SqlValue::Text(field.into()));
    for v in values {
        params.push(SqlValue::Text(v.0.clone()));
    }
}
