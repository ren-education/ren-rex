//! Integration tests against a real :memory: SQLite database.

use std::path::PathBuf;

use rex_domain::{
    Document, DocumentId, DocumentKind, Embedding, Filters, FtsIndex, ItemStore,
    PdfAnchor, SourcePath, SubjectId, TagField, TagValue, Tags, VectorStore,
};
use rex_sqlite::{open_db, SqliteStore};
use uuid::Uuid;

const DIM: usize = 8;

fn make_doc(question: &str, subject: &str, topics: &[&str], schools: &[&str]) -> Document {
    Document {
        id: DocumentId(Uuid::new_v4()),
        subject: SubjectId::new(subject),
        kind: DocumentKind::Question,
        parent_id: None,
        depends_on: vec![],
        number: Some("1(a)".into()),
        source: SourcePath::new(PathBuf::from("content/x.md")),
        context: Some("In a thought experiment...".into()),
        question: Some(question.into()),
        answer: Some("Because of forces.".into()),
        notes: None,
        mark: Some(5),
        options: None,
        keywords: vec!["energy".into(), "kinematics".into()],
        tags: Tags {
            topics: topics.iter().map(|t| TagValue::new(*t)).collect(),
            schools: schools.iter().map(|s| TagValue::new(*s)).collect(),
            paper_types: vec![TagValue::new("paper-2")],
            ..Default::default()
        },
        pdf_anchor: Some(PdfAnchor {
            pdf_path: PathBuf::from("prelims/2019/HCI/X.pdf"),
            page_number: Some(3),
            bbox: None,
            confidence: 0.92,
            fallback_reason: None,
        }),
    }
}

fn store() -> SqliteStore {
    let conn = open_db(std::path::Path::new(":memory:")).unwrap();
    SqliteStore::new(conn, DIM)
}

#[tokio::test]
async fn put_then_get_roundtrip() {
    let s = store();
    let d = make_doc("Explain tension in the cable.", "h2physics", &["dynamics"], &["hci"]);
    s.put(std::slice::from_ref(&d)).await.unwrap();

    let got = s.get(&d.id).await.unwrap().expect("doc present");
    assert_eq!(got.id, d.id);
    assert_eq!(got.question.as_deref(), Some("Explain tension in the cable."));
    assert_eq!(got.subject.0, "h2physics");
    assert_eq!(got.tags.topics.len(), 1);
    assert_eq!(got.tags.topics[0].0, "dynamics");
    assert_eq!(got.pdf_anchor.unwrap().page_number, Some(3));
}

#[tokio::test]
async fn query_with_topic_filter() {
    let s = store();
    let d1 = make_doc("dynamic Q1", "h2physics", &["dynamics"], &["hci"]);
    let d2 = make_doc("dynamic Q2", "h2physics", &["dynamics"], &["njc"]);
    let d3 = make_doc("thermal Q", "h2physics", &["thermal-physics"], &["hci"]);
    s.put(&[d1.clone(), d2.clone(), d3.clone()]).await.unwrap();

    let filters = Filters {
        subject: Some(SubjectId::new("h2physics")),
        topics: vec![TagValue::new("dynamics")],
        ..Default::default()
    };
    let docs = s.query(&filters, 10, 0).await.unwrap();
    assert_eq!(docs.len(), 2);

    let count = s.count(&filters).await.unwrap();
    assert_eq!(count, 2);
}

#[tokio::test]
async fn fts_upsert_and_search() {
    let s = store();
    let d1 = make_doc(
        "Explain why the tension in the cable is not equal to weight",
        "h2physics",
        &["dynamics"],
        &["hci"],
    );
    let d2 = make_doc(
        "Describe centripetal force in uniform circular motion",
        "h2physics",
        &["circular-motion"],
        &["njc"],
    );
    s.put(&[d1.clone(), d2.clone()]).await.unwrap();

    FtsIndex::upsert(
        &s,
        &[
            (d1.id, d1.question.clone().unwrap()),
            (d2.id, d2.question.clone().unwrap()),
        ],
    )
    .await
    .unwrap();

    let results = FtsIndex::search(
        &s,
        "tension cable",
        &Filters {
            subject: Some(SubjectId::new("h2physics")),
            ..Default::default()
        },
        10,
    )
    .await
    .unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].0, d1.id);
}

#[tokio::test]
async fn vector_upsert_and_search_with_filter() {
    let s = store();
    let d1 = make_doc("question 1", "h2physics", &["dynamics"], &["hci"]);
    let d2 = make_doc("question 2", "h2physics", &["thermal-physics"], &["hci"]);
    s.put(&[d1.clone(), d2.clone()]).await.unwrap();

    let v1 = vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    let v2 = vec![0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    VectorStore::upsert(
        &s,
        &[
            (d1.id, Embedding::new(DIM, v1.clone()).unwrap()),
            (d2.id, Embedding::new(DIM, v2.clone()).unwrap()),
        ],
    )
    .await
    .unwrap();

    // Search with query close to v1: should return d1 first.
    let query = Embedding::new(DIM, vec![0.9, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]).unwrap();
    let r = VectorStore::search(&s, &query, &Filters::default(), 5)
        .await
        .unwrap();
    assert!(!r.is_empty());
    assert_eq!(r[0].0, d1.id);

    // Now apply a topic filter restricting to thermal-physics — only d2 should appear.
    let filtered = Filters {
        topics: vec![TagValue::new("thermal-physics")],
        ..Default::default()
    };
    let r = VectorStore::search(&s, &query, &filtered, 5).await.unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].0, d2.id);
}

#[tokio::test]
async fn facet_counts_respect_filters() {
    let s = store();
    let d1 = make_doc("a", "h2physics", &["dynamics"], &["hci"]);
    let d2 = make_doc("b", "h2physics", &["dynamics"], &["njc"]);
    let d3 = make_doc("c", "h2physics", &["thermal-physics"], &["hci"]);
    s.put(&[d1.clone(), d2.clone(), d3.clone()]).await.unwrap();

    // Without filters: dynamics = 2, thermal-physics = 1
    let counts = s
        .facet_counts(
            &SubjectId::new("h2physics"),
            TagField::Topics,
            &Filters::default(),
        )
        .await
        .unwrap();
    let m: std::collections::HashMap<_, _> =
        counts.into_iter().map(|c| (c.value.0, c.count)).collect();
    assert_eq!(m.get("dynamics"), Some(&2));
    assert_eq!(m.get("thermal-physics"), Some(&1));

    // With school=hci filter: dynamics = 1 (d1), thermal-physics = 1 (d3)
    let counts = s
        .facet_counts(
            &SubjectId::new("h2physics"),
            TagField::Topics,
            &Filters {
                schools: vec![TagValue::new("hci")],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let m: std::collections::HashMap<_, _> =
        counts.into_iter().map(|c| (c.value.0, c.count)).collect();
    assert_eq!(m.get("dynamics"), Some(&1));
    assert_eq!(m.get("thermal-physics"), Some(&1));
}

#[tokio::test]
async fn list_subjects_returns_question_note_counts() {
    let s = store();
    let d1 = make_doc("a", "h2physics", &["dynamics"], &["hci"]);
    let mut d2 = make_doc("b", "h2physics", &["dynamics"], &["hci"]);
    d2.kind = DocumentKind::Note;
    s.put(&[d1.clone(), d2.clone()]).await.unwrap();

    let subjects = s.list_subjects().await.unwrap();
    assert_eq!(subjects.len(), 1);
    assert_eq!(subjects[0].id.0, "h2physics");
    assert_eq!(subjects[0].question_count, 1);
    assert_eq!(subjects[0].note_count, 1);
    assert_eq!(subjects[0].item_count, 2);
}

#[tokio::test]
async fn clear_removes_only_that_subject() {
    let s = store();
    let d1 = make_doc("a", "h2physics", &["dynamics"], &["hci"]);
    let d2 = make_doc("b", "hcchem", &["acids"], &["hci"]);
    s.put(&[d1.clone(), d2.clone()]).await.unwrap();

    ItemStore::clear(&s, &SubjectId::new("h2physics")).await.unwrap();
    let docs = s.query(&Filters::default(), 100, 0).await.unwrap();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].subject.0, "hcchem");
}
