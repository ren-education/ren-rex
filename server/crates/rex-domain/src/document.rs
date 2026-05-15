//! Document and tag types. These are the core entities the rex index stores.

use serde::{Deserialize, Serialize};

use crate::ids::{DocumentId, SourcePath, SubjectId, TagValue};
use crate::pdf::PdfAnchor;

/// A single searchable item (question or note) from a subject's compiled corpus.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub id: DocumentId,
    pub subject: SubjectId,
    pub kind: DocumentKind,
    pub parent_id: Option<DocumentId>,
    /// Sibling-dependency ids (e.g., "this part depends on the answer to 1(a)(i)").
    pub depends_on: Vec<DocumentId>,
    /// Human-readable identifier within the source document, e.g. "1(a)(i)".
    pub number: Option<String>,
    pub source: SourcePath,
    pub context: Option<String>,
    pub question: Option<String>,
    pub answer: Option<String>,
    pub notes: Option<String>,
    pub mark: Option<u32>,
    pub options: Option<Vec<String>>,
    pub keywords: Vec<String>,
    pub tags: Tags,
    pub pdf_anchor: Option<PdfAnchor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentKind {
    Question,
    Note,
}

impl DocumentKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            DocumentKind::Question => "Question",
            DocumentKind::Note => "Note",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tags {
    #[serde(default)]
    pub topics: Vec<TagValue>,
    #[serde(default)]
    pub question_types: Vec<TagValue>,
    #[serde(default)]
    pub exam_systems: Vec<TagValue>,
    #[serde(default)]
    pub paper_types: Vec<TagValue>,
    #[serde(default)]
    pub schools: Vec<TagValue>,
    #[serde(default)]
    pub source_types: Vec<TagValue>,
}

/// The six tag axes the corpus carries. Stable closed set — drives schema and APIs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TagField {
    Topics,
    QuestionTypes,
    ExamSystems,
    PaperTypes,
    Schools,
    SourceTypes,
}

impl TagField {
    pub const ALL: &'static [TagField] = &[
        TagField::Topics,
        TagField::QuestionTypes,
        TagField::ExamSystems,
        TagField::PaperTypes,
        TagField::Schools,
        TagField::SourceTypes,
    ];

    /// SQL/wire column name for this field.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            TagField::Topics => "topics",
            TagField::QuestionTypes => "question_types",
            TagField::ExamSystems => "exam_systems",
            TagField::PaperTypes => "paper_types",
            TagField::Schools => "schools",
            TagField::SourceTypes => "source_types",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "topics" => Some(TagField::Topics),
            "question_types" => Some(TagField::QuestionTypes),
            "exam_systems" => Some(TagField::ExamSystems),
            "paper_types" => Some(TagField::PaperTypes),
            "schools" => Some(TagField::Schools),
            "source_types" => Some(TagField::SourceTypes),
            _ => None,
        }
    }
}

impl Tags {
    /// Iterate all tag (field, value) pairs. Useful for indexing.
    pub fn flat(&self) -> impl Iterator<Item = (TagField, &TagValue)> {
        let topics = self.topics.iter().map(|v| (TagField::Topics, v));
        let qt = self.question_types.iter().map(|v| (TagField::QuestionTypes, v));
        let es = self.exam_systems.iter().map(|v| (TagField::ExamSystems, v));
        let pt = self.paper_types.iter().map(|v| (TagField::PaperTypes, v));
        let sch = self.schools.iter().map(|v| (TagField::Schools, v));
        let st = self.source_types.iter().map(|v| (TagField::SourceTypes, v));
        topics.chain(qt).chain(es).chain(pt).chain(sch).chain(st)
    }

    pub fn values_for(&self, field: TagField) -> &[TagValue] {
        match field {
            TagField::Topics => &self.topics,
            TagField::QuestionTypes => &self.question_types,
            TagField::ExamSystems => &self.exam_systems,
            TagField::PaperTypes => &self.paper_types,
            TagField::Schools => &self.schools,
            TagField::SourceTypes => &self.source_types,
        }
    }
}
