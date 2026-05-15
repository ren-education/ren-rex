//! Unit tests for the foundational types.

use std::path::PathBuf;

use rex_domain::{
    DocumentId, DocumentKind, Embedding, Error, Filters, SearchMode, SourcePath, SubjectId,
    TagField, TagValue, Tags,
};

#[test]
fn embedding_dimension_mismatch_errors() {
    let result = Embedding::new(768, vec![0.0; 512]);
    assert!(matches!(result, Err(Error::BadInput { .. })));
}

#[test]
fn embedding_correct_dimension_succeeds() {
    let result = Embedding::new(4, vec![1.0, 2.0, 3.0, 4.0]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().dimension(), 4);
}

#[test]
fn filters_default_is_match_everything() {
    let f = Filters::default();
    assert!(f.is_match_all());
}

#[test]
fn filters_with_subject_is_not_match_all() {
    let f = Filters {
        subject: Some(SubjectId::new("h2physics")),
        ..Default::default()
    };
    assert!(!f.is_match_all());
}

#[test]
fn document_kind_serde_roundtrip() {
    for k in [DocumentKind::Question, DocumentKind::Note] {
        let s = serde_json::to_string(&k).unwrap();
        let back: DocumentKind = serde_json::from_str(&s).unwrap();
        assert_eq!(k, back);
    }
}

#[test]
fn search_mode_default_is_hybrid() {
    let m = SearchMode::default();
    assert_eq!(m, SearchMode::Hybrid);
}

#[test]
fn tag_field_enumeration_has_six_values() {
    assert_eq!(TagField::ALL.len(), 6);
}

#[test]
fn tag_field_db_str_roundtrip() {
    for f in TagField::ALL {
        let s = f.as_db_str();
        let back = TagField::from_db_str(s).unwrap();
        assert_eq!(*f, back);
    }
}

#[test]
fn tags_flat_iterates_all_pairs() {
    let tags = Tags {
        topics: vec![TagValue::new("dynamics"), TagValue::new("forces")],
        schools: vec![TagValue::new("hci")],
        ..Default::default()
    };
    let pairs: Vec<_> = tags.flat().collect();
    assert_eq!(pairs.len(), 3);
}

#[test]
fn document_id_parse_and_display_roundtrip() {
    let id = DocumentId::new();
    let s = id.to_string();
    let parsed = DocumentId::parse(&s).unwrap();
    assert_eq!(id, parsed);
}

#[test]
fn subject_id_display() {
    let s = SubjectId::new("h2physics");
    assert_eq!(s.to_string(), "h2physics");
}

#[test]
fn source_path_round_trip() {
    let p = SourcePath::new(PathBuf::from("content/foo/bar.md"));
    assert_eq!(p.as_path(), std::path::Path::new("content/foo/bar.md"));
}

#[test]
fn error_constructors_set_fields() {
    let e = Error::bad_input_field("invalid", "text");
    if let Error::BadInput { field, .. } = e {
        assert_eq!(field.as_deref(), Some("text"));
    } else {
        panic!("expected BadInput variant");
    }
}

#[test]
fn filters_matches_subject_correctly() {
    use rex_domain::Document;
    use uuid::Uuid;

    let doc = Document {
        id: DocumentId(Uuid::new_v4()),
        subject: SubjectId::new("h2physics"),
        kind: DocumentKind::Question,
        parent_id: None,
        depends_on: vec![],
        number: None,
        source: SourcePath::new("x.md"),
        context: None,
        question: Some("test".into()),
        answer: None,
        notes: None,
        mark: Some(5),
        options: None,
        keywords: vec![],
        tags: Tags {
            topics: vec![TagValue::new("dynamics")],
            ..Default::default()
        },
        pdf_anchor: None,
    };

    let f_match = Filters {
        subject: Some(SubjectId::new("h2physics")),
        topics: vec![TagValue::new("dynamics")],
        ..Default::default()
    };
    assert!(f_match.matches(&doc));

    let f_wrong_subject = Filters {
        subject: Some(SubjectId::new("hcchem")),
        ..Default::default()
    };
    assert!(!f_wrong_subject.matches(&doc));

    let f_wrong_topic = Filters {
        topics: vec![TagValue::new("thermal")],
        ..Default::default()
    };
    assert!(!f_wrong_topic.matches(&doc));

    let f_marks_in_range = Filters {
        marks_range: Some((1, 10)),
        ..Default::default()
    };
    assert!(f_marks_in_range.matches(&doc));

    let f_marks_out_of_range = Filters {
        marks_range: Some((10, 20)),
        ..Default::default()
    };
    assert!(!f_marks_out_of_range.matches(&doc));
}
