//! Parsing of `questions.jsonl` and `notes.jsonl` rows.
//!
//! `JsonlRow` mirrors the observed shape of rob-the-crawler's output across
//! both h2physics and h2history. `#[serde(deny_unknown_fields)]` is on so any
//! new field rob adds surfaces as a loud parse failure rather than silent loss.

use std::path::PathBuf;

use rex_domain::{
    Document, DocumentId, DocumentKind, SourcePath, SubjectId, TagValue, Tags,
};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JsonlRow {
    pub id: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub number: Option<String>,
    pub source: String,
    /// Some subjects (h2history) carry a top-level `source_type` field in
    /// addition to `tags.source_types`. We accept and ignore it; the canonical
    /// taxonomy lives under tags.source_types.
    #[serde(default)]
    pub source_type: Option<String>,
    #[serde(default)]
    pub context: Option<String>,
    #[serde(default)]
    pub question: Option<String>,
    #[serde(default)]
    pub mark: Option<u32>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub options: Option<Vec<String>>,
    /// Always-present, often-empty; we ignore the contents in v1.
    #[serde(default)]
    pub images: Vec<serde_json::Value>,
    #[serde(default)]
    pub answer: Option<String>,
    #[serde(default)]
    pub answer_images: Vec<serde_json::Value>,
    #[serde(default)]
    pub notes: Option<String>,
    /// Notes carry their body in `content` (not `question`/`answer`).
    #[serde(default)]
    pub content: Option<String>,
    /// Notes have a `type` field (e.g., "wiki", "knowledge"). Accepted, unused.
    #[serde(default, rename = "type")]
    pub note_type: Option<String>,
    /// Notes reference related question ids instead of `parent_id`/`depends_on`.
    #[serde(default)]
    pub related_question_ids: Vec<String>,
    #[serde(default)]
    pub tags: JsonlTags,
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct JsonlTags {
    #[serde(default)]
    pub topics: Vec<String>,
    #[serde(default)]
    pub question_types: Vec<String>,
    #[serde(default)]
    pub exam_systems: Vec<String>,
    #[serde(default)]
    pub paper_types: Vec<String>,
    #[serde(default)]
    pub schools: Vec<String>,
    #[serde(default)]
    pub source_types: Vec<String>,
}

impl JsonlRow {
    pub fn parse(line: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(line)
    }

    pub fn into_document(
        self,
        subject: SubjectId,
        kind: DocumentKind,
    ) -> Result<Document, String> {
        let id = DocumentId::parse(&self.id).map_err(|e| format!("invalid id {}: {e}", self.id))?;
        let parent_id = self
            .parent_id
            .as_ref()
            .map(|s| DocumentId::parse(s).map_err(|e| format!("invalid parent_id {s}: {e}")))
            .transpose()?;
        let depends_on: Vec<DocumentId> = self
            .depends_on
            .iter()
            .filter_map(|s| Uuid::parse_str(s).ok().map(DocumentId))
            .collect();

        // For notes, the body lives in `content`. Map it into the `question`
        // field for indexing (the search_text builder includes question text
        // with a "Question:" prefix; we accept that prefix on notes too — it's
        // a soft signal that doesn't hurt retrieval).
        let (question_field, notes_field) = match kind {
            DocumentKind::Question => (self.question, self.notes),
            DocumentKind::Note => (self.content.or(self.question), self.notes),
        };

        Ok(Document {
            id,
            subject,
            kind,
            parent_id,
            depends_on,
            number: self.number,
            source: SourcePath::new(PathBuf::from(self.source)),
            context: self.context,
            question: question_field,
            answer: self.answer,
            notes: notes_field,
            mark: self.mark,
            options: self.options,
            keywords: self.keywords,
            tags: Tags {
                topics: self.tags.topics.into_iter().map(TagValue::new).collect(),
                question_types: self
                    .tags
                    .question_types
                    .into_iter()
                    .map(TagValue::new)
                    .collect(),
                exam_systems: self
                    .tags
                    .exam_systems
                    .into_iter()
                    .map(TagValue::new)
                    .collect(),
                paper_types: self.tags.paper_types.into_iter().map(TagValue::new).collect(),
                schools: self.tags.schools.into_iter().map(TagValue::new).collect(),
                source_types: self.tags.source_types.into_iter().map(TagValue::new).collect(),
            },
            pdf_anchor: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const H2PHYSICS_SAMPLE: &str = r#"{"id": "d95d6cc3-5f4e-41ed-9741-a14bad3b6320", "parent_id": null, "depends_on": [], "number": "1", "source": "content/prelims/2019/HCI/X.md", "context": "A large metal ball...", "question": null, "mark": 9, "keywords": ["circular motion"], "options": null, "images": [], "answer": null, "answer_images": [], "notes": null, "tags": {"topics": ["dynamics"], "question_types": ["structured"], "exam_systems": ["a-level"], "paper_types": ["paper-2"], "schools": ["hwa-chong-institution"], "source_types": ["prelims"]}}"#;

    #[test]
    fn parse_h2physics_row() {
        let row = JsonlRow::parse(H2PHYSICS_SAMPLE).unwrap();
        assert_eq!(row.id, "d95d6cc3-5f4e-41ed-9741-a14bad3b6320");
        assert_eq!(row.mark, Some(9));
        assert_eq!(row.tags.topics, vec!["dynamics"]);
    }

    #[test]
    fn deny_unknown_fields_rejects_new_keys() {
        let with_unknown = r#"{"id":"d95d6cc3-5f4e-41ed-9741-a14bad3b6320","source":"x.md","tags":{}, "newfield":"foo"}"#;
        let err = JsonlRow::parse(with_unknown).unwrap_err();
        assert!(err.to_string().to_lowercase().contains("newfield"));
    }

    #[test]
    fn minimal_row_with_only_required_fields() {
        let minimal =
            r#"{"id":"d95d6cc3-5f4e-41ed-9741-a14bad3b6320","source":"x.md","tags":{}}"#;
        let row = JsonlRow::parse(minimal).unwrap();
        assert_eq!(row.id, "d95d6cc3-5f4e-41ed-9741-a14bad3b6320");
    }

    #[test]
    fn into_document_maps_fields() {
        let row = JsonlRow::parse(H2PHYSICS_SAMPLE).unwrap();
        let doc = row
            .into_document(SubjectId::new("h2physics"), DocumentKind::Question)
            .unwrap();
        assert_eq!(doc.subject.0, "h2physics");
        assert_eq!(doc.kind, DocumentKind::Question);
        assert_eq!(doc.tags.topics.len(), 1);
        assert_eq!(doc.mark, Some(9));
    }
}
