//! Pretty vs JSON output rendering.

use rex_domain::{Document, FacetCount, PdfAnchor, SearchResponse, SubjectStats, TagValue};

pub fn render_search(resp: &SearchResponse, json: bool) {
    if json {
        println!("{}", serde_json::to_string_pretty(resp).unwrap_or_default());
        return;
    }
    println!(
        "{} hits in {}ms  (mode={:?}, embedder={}, bm25={}, vector={}, reranker={})",
        resp.hits.len(),
        resp.meta.took_ms,
        resp.meta.mode,
        resp.meta.used_embedder,
        resp.meta.used_bm25,
        resp.meta.used_vector,
        resp.meta.used_reranker,
    );
    if let Some(t) = resp.meta.total_matches {
        println!("(total filtered: {})", t);
    }
    println!();
    for (i, hit) in resp.hits.iter().enumerate() {
        let snippet = hit
            .document
            .question
            .as_deref()
            .or(hit.document.context.as_deref())
            .unwrap_or("(no text)")
            .chars()
            .take(160)
            .collect::<String>();
        println!(
            "{:>3}. [{}] {} {}",
            i + 1,
            hit.document.kind.as_str(),
            hit.document.number.as_deref().unwrap_or(""),
            snippet,
        );
        println!(
            "     id={} subject={} score={:.3} (bm25={:?} vec={:?} rerank={:?})",
            hit.document.id,
            hit.document.subject,
            hit.score,
            hit.scores.bm25,
            hit.scores.vector,
            hit.scores.rerank,
        );
        if let Some(anchor) = &hit.document.pdf_anchor {
            print!("     pdf: {} ", anchor.pdf_path.display());
            match anchor.page_number {
                Some(p) => println!("(page {}, conf={:.2})", p, anchor.confidence),
                None => println!(
                    "({})",
                    anchor
                        .fallback_reason
                        .map(|r| r.as_str())
                        .unwrap_or("file-only")
                ),
            }
        }
        for h in hit.highlights.iter().take(2) {
            println!("     {:?}: {}", h.field, h.text);
        }
        println!();
    }
}

pub fn render_document(doc: &Document, json: bool) {
    if json {
        println!("{}", serde_json::to_string_pretty(doc).unwrap_or_default());
        return;
    }
    println!("id      : {}", doc.id);
    println!("subject : {}", doc.subject);
    println!("kind    : {}", doc.kind.as_str());
    if let Some(n) = &doc.number {
        println!("number  : {}", n);
    }
    println!("source  : {}", doc.source);
    if let Some(c) = &doc.context {
        println!("context : {}", c);
    }
    if let Some(q) = &doc.question {
        println!("question: {}", q);
    }
    if let Some(a) = &doc.answer {
        println!("answer  : {}", a);
    }
    if let Some(m) = doc.mark {
        println!("mark    : {}", m);
    }
    if !doc.tags.topics.is_empty() {
        println!("topics  : {}", join_tags(&doc.tags.topics));
    }
    if let Some(anchor) = &doc.pdf_anchor {
        println!("pdf     : {} (page={:?})", anchor.pdf_path.display(), anchor.page_number);
    }
}

pub fn render_subjects(subjects: &[SubjectStats], json: bool) {
    if json {
        println!("{}", serde_json::to_string_pretty(subjects).unwrap_or_default());
        return;
    }
    println!("{:<20} {:>10} {:>10} {:>10}", "subject", "items", "questions", "notes");
    println!("{}", "─".repeat(56));
    for s in subjects {
        println!(
            "{:<20} {:>10} {:>10} {:>10}",
            s.id, s.item_count, s.question_count, s.note_count
        );
    }
}

pub fn render_subject(s: &SubjectStats, json: bool) {
    if json {
        println!("{}", serde_json::to_string_pretty(s).unwrap_or_default());
        return;
    }
    println!("subject       : {}", s.id);
    println!("item_count    : {}", s.item_count);
    println!("question_count: {}", s.question_count);
    println!("note_count    : {}", s.note_count);
}

pub fn render_tag_values(field: &str, values: &[FacetCount], json: bool) {
    if json {
        let body = serde_json::json!({
            "field": field,
            "values": values,
        });
        println!("{}", serde_json::to_string_pretty(&body).unwrap_or_default());
        return;
    }
    println!("{:<40} {:>8}", field, "count");
    println!("{}", "─".repeat(50));
    for v in values {
        println!("{:<40} {:>8}", v.value, v.count);
    }
}

pub fn render_pdf_anchor(anchor: &PdfAnchor, json: bool) {
    if json {
        println!("{}", serde_json::to_string_pretty(anchor).unwrap_or_default());
        return;
    }
    println!("pdf_path  : {}", anchor.pdf_path.display());
    println!("page      : {:?}", anchor.page_number);
    println!("confidence: {:.3}", anchor.confidence);
    println!("fallback  : {:?}", anchor.fallback_reason);
}

fn join_tags(tags: &[TagValue]) -> String {
    tags.iter()
        .map(|t| t.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}
