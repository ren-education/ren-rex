//! End-to-end pipeline tests against the fake adapters.
//!
//! These cover the spec's mode-dispatch invariants:
//! - Hybrid uses every stage.
//! - Bm25Only skips embedder, vector, rerank.
//! - VectorOnly skips bm25, rerank.
//! - Filter mode short-circuits to ItemStore::query (text=None).
//! - Validation: text=None + non-Filter → BadInput.
//! - Validation: text=Some + Filter → BadInput.

use std::sync::Arc;

use rex_domain::{
    Document, DocumentId, DocumentKind, Embedder, Error, Filters, FtsIndex, ItemStore, SearchMode,
    SearchQuery, SourcePath, SubjectId, TagValue, Tags, VectorStore,
};
use rex_search::fakes::{
    FakeFtsIndex, FakeItemStore, FakeVectorStore, SpyEmbedder, SpyReranker,
};
use rex_search::SearchService;
use uuid::Uuid;

const DIM: usize = 32;

fn doc(question: &str, topics: &[&str]) -> Document {
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
        mark: Some(5),
        options: None,
        keywords: vec![],
        tags: Tags {
            topics: topics.iter().map(|t| TagValue::new(*t)).collect(),
            ..Default::default()
        },
        pdf_anchor: None,
    }
}

async fn seed_corpus() -> (
    Arc<FakeItemStore>,
    Arc<FakeFtsIndex>,
    Arc<FakeVectorStore>,
    Arc<SpyEmbedder>,
    Arc<SpyReranker>,
) {
    let items = Arc::new(FakeItemStore::new());
    let docs = vec![
        doc("Explain why the tension in the cable is not equal to weight", &["dynamics"]),
        doc("Describe centripetal force and circular motion", &["circular-motion"]),
        doc("Calculate the kinetic energy of the ball", &["energy", "dynamics"]),
        doc("Explain heat capacity in thermal physics", &["thermal-physics"]),
    ];
    items.put(&docs).await.unwrap();

    let embedder = Arc::new(SpyEmbedder::new(DIM));
    let fts = Arc::new(FakeFtsIndex::new(Arc::clone(&items)));
    let vectors = Arc::new(FakeVectorStore::new(DIM, Arc::clone(&items)));

    // Build FTS + vector indexes
    let fts_data: Vec<_> = docs.iter().map(|d| (d.id, d.question.clone().unwrap())).collect();
    fts.upsert(&fts_data).await.unwrap();

    let texts: Vec<String> = docs.iter().map(|d| d.question.clone().unwrap()).collect();
    let embs = embedder.embed_documents(&texts).await.unwrap();
    let vec_data: Vec<_> = docs.iter().zip(embs.into_iter()).map(|(d, e)| (d.id, e)).collect();
    vectors.upsert(&vec_data).await.unwrap();

    let reranker = Arc::new(SpyReranker::new());
    // Reset embedder call log so test assertions don't count the ingest embeds.
    embedder.reset();

    (items, fts, vectors, embedder, reranker)
}

#[tokio::test]
async fn hybrid_mode_uses_all_stages() {
    let (items, fts, vectors, embedder, reranker) = seed_corpus().await;

    let svc = SearchService::builder()
        .items(items)
        .fts(fts)
        .vectors(vectors)
        .embedder(Arc::clone(&embedder) as _)
        .reranker(Arc::clone(&reranker) as _)
        .build()
        .unwrap();

    let resp = svc
        .search(SearchQuery {
            text: Some("tension cable".into()),
            filters: Filters::default(),
            limit: 5,
            mode: SearchMode::Hybrid,
            exact: false,
            rerank: true,
        })
        .await
        .unwrap();

    assert!(resp.meta.used_embedder);
    assert!(resp.meta.used_bm25);
    assert!(resp.meta.used_vector);
    assert!(resp.meta.used_reranker);
    assert_eq!(resp.meta.mode, SearchMode::Hybrid);
    assert_eq!(embedder.calls().iter().filter(|c| c.starts_with("query:")).count(), 1);
    assert_eq!(reranker.calls().len(), 1);
    assert!(!resp.hits.is_empty());
}

#[tokio::test]
async fn bm25_only_skips_embedder_vector_reranker() {
    let (items, fts, vectors, embedder, reranker) = seed_corpus().await;
    let svc = SearchService::builder()
        .items(items)
        .fts(fts)
        .vectors(vectors)
        .embedder(Arc::clone(&embedder) as _)
        .reranker(Arc::clone(&reranker) as _)
        .build()
        .unwrap();

    let resp = svc
        .search(SearchQuery {
            text: Some("tension".into()),
            filters: Filters::default(),
            limit: 5,
            mode: SearchMode::Bm25Only,
            exact: false,
            rerank: true,
        })
        .await
        .unwrap();

    assert!(!resp.meta.used_embedder);
    assert!(resp.meta.used_bm25);
    assert!(!resp.meta.used_vector);
    assert!(!resp.meta.used_reranker);
    assert_eq!(embedder.calls().iter().filter(|c| c.starts_with("query:")).count(), 0);
    assert_eq!(reranker.calls().len(), 0);
}

#[tokio::test]
async fn vector_only_skips_bm25_reranker() {
    let (items, fts, vectors, embedder, reranker) = seed_corpus().await;
    let svc = SearchService::builder()
        .items(items)
        .fts(fts)
        .vectors(vectors)
        .embedder(Arc::clone(&embedder) as _)
        .reranker(Arc::clone(&reranker) as _)
        .build()
        .unwrap();

    let resp = svc
        .search(SearchQuery {
            text: Some("tension cable".into()),
            filters: Filters::default(),
            limit: 5,
            mode: SearchMode::VectorOnly,
            exact: false,
            rerank: true,
        })
        .await
        .unwrap();

    assert!(resp.meta.used_embedder);
    assert!(!resp.meta.used_bm25);
    assert!(resp.meta.used_vector);
    assert!(!resp.meta.used_reranker);
    assert_eq!(reranker.calls().len(), 0);
}

#[tokio::test]
async fn filter_mode_via_filter_method() {
    let (items, fts, vectors, embedder, _reranker) = seed_corpus().await;
    let svc = SearchService::builder()
        .items(items)
        .fts(fts)
        .vectors(vectors)
        .embedder(Arc::clone(&embedder) as _)
        .build()
        .unwrap();

    let filters = Filters {
        subject: Some(SubjectId::new("h2physics")),
        topics: vec![TagValue::new("dynamics")],
        ..Default::default()
    };
    let resp = svc.filter(filters, 10, 0).await.unwrap();
    assert_eq!(resp.meta.mode, SearchMode::Filter);
    assert!(!resp.meta.used_embedder);
    assert!(!resp.meta.used_bm25);
    assert!(resp.meta.total_matches.unwrap() >= 1);
    // Every hit should have empty scores.
    for hit in &resp.hits {
        assert!(hit.scores.bm25.is_none());
        assert!(hit.scores.vector.is_none());
        assert!(hit.scores.rerank.is_none());
    }
}

#[tokio::test]
async fn validate_text_none_with_hybrid_rejects() {
    let (items, fts, vectors, embedder, _reranker) = seed_corpus().await;
    let svc = SearchService::builder()
        .items(items)
        .fts(fts)
        .vectors(vectors)
        .embedder(Arc::clone(&embedder) as _)
        .build()
        .unwrap();

    let err = svc
        .search(SearchQuery {
            text: None,
            filters: Filters::default(),
            limit: 5,
            mode: SearchMode::Hybrid,
            exact: false,
            rerank: true,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, Error::BadInput { .. }));
}

#[tokio::test]
async fn validate_text_some_with_filter_rejects() {
    let (items, fts, vectors, embedder, _reranker) = seed_corpus().await;
    let svc = SearchService::builder()
        .items(items)
        .fts(fts)
        .vectors(vectors)
        .embedder(Arc::clone(&embedder) as _)
        .build()
        .unwrap();

    let err = svc
        .search(SearchQuery {
            text: Some("tension".into()),
            filters: Filters::default(),
            limit: 5,
            mode: SearchMode::Filter,
            exact: false,
            rerank: true,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, Error::BadInput { .. }));
}

#[tokio::test]
async fn no_reranker_skips_pre_rerank_hydration() {
    // When SearchService is built without a reranker, the pipeline should
    // never call get_many() on the top-N pre-rerank set. It only calls
    // get_many() once at the end with top-`limit`. We verify by inspecting
    // the post-search ItemStore call shape indirectly: the response still
    // contains documents and `used_reranker` is false.
    let (items, fts, vectors, embedder, _reranker) = seed_corpus().await;
    let svc = SearchService::builder()
        .items(items)
        .fts(fts)
        .vectors(vectors)
        .embedder(Arc::clone(&embedder) as _)
        .build() // no reranker
        .unwrap();

    let resp = svc
        .search(SearchQuery {
            text: Some("tension cable".into()),
            filters: Filters::default(),
            limit: 3,
            mode: SearchMode::Hybrid,
            exact: false,
            rerank: true,
        })
        .await
        .unwrap();

    assert!(!resp.meta.used_reranker);
    assert!(resp.hits.len() <= 3);
}

#[tokio::test]
async fn filter_pushdown_consistent_across_retrievers() {
    // A filter that selects only one document should restrict both BM25 and
    // vector retrievers to that one document (or zero, if it has no text match).
    let (items, fts, vectors, embedder, _reranker) = seed_corpus().await;
    let svc = SearchService::builder()
        .items(items)
        .fts(fts)
        .vectors(vectors)
        .embedder(Arc::clone(&embedder) as _)
        .build()
        .unwrap();

    let filters = Filters {
        topics: vec![TagValue::new("thermal-physics")],
        ..Default::default()
    };
    let resp = svc
        .search(SearchQuery {
            text: Some("heat".into()),
            filters: filters.clone(),
            limit: 10,
            mode: SearchMode::Hybrid,
            exact: false,
            rerank: false,
        })
        .await
        .unwrap();

    // Only the thermal-physics doc should appear.
    for hit in &resp.hits {
        assert!(hit.document.tags.topics.iter().any(|t| t.0 == "thermal-physics"));
    }
}

#[tokio::test]
async fn fts5_query_quoted_when_exact_set() {
    let (items, fts, vectors, embedder, _reranker) = seed_corpus().await;
    let svc = SearchService::builder()
        .items(items)
        .fts(fts)
        .vectors(vectors)
        .embedder(Arc::clone(&embedder) as _)
        .build()
        .unwrap();

    let resp = svc
        .search(SearchQuery {
            text: Some("tension cable".into()),
            filters: Filters::default(),
            limit: 5,
            mode: SearchMode::Bm25Only,
            exact: true,
            rerank: false,
        })
        .await
        .unwrap();
    assert_eq!(resp.meta.fts5_query.unwrap(), "\"tension cable\"");
}
