//! Subcommand dispatch.

use anyhow::{Context, Result};
use rex_domain::{
    DocumentId, Filters, SearchQuery, SubjectId, TagField, TagValue,
};
use rex_ingest::{IngestConfig, IngestServices};

use crate::cli::{Cli, Cmd, FilterArgs, IngestArgs, SearchArgs, TagValuesArgs};
use crate::output;
use crate::wire;

pub async fn dispatch(cli: Cli) -> i32 {
    let result: Result<()> = match &cli.cmd {
        Cmd::Ingest(args) => ingest(&cli, args).await,
        Cmd::Serve(_) => {
            eprintln!("`rex serve` is not yet wired in v1 (rex-api crate pending).");
            std::process::exit(64);
        }
        Cmd::Search(args) => search(&cli, args).await,
        Cmd::Filter(args) => filter(&cli, args).await,
        Cmd::Get { id } => get(&cli, id).await,
        Cmd::Subjects => subjects(&cli).await,
        Cmd::Subject { id } => subject(&cli, id).await,
        Cmd::TagValues(args) => tag_values(&cli, args).await,
        Cmd::PdfAnchor { id } => pdf_anchor(&cli, id).await,
    };
    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("error: {e:#}");
            1
        }
    }
}

async fn ingest(cli: &Cli, args: &IngestArgs) -> Result<()> {
    let docs_root = args
        .docs_root_override
        .clone()
        .or_else(|| cli.docs_root.clone())
        .context("ingest requires --docs-root or --docs-root-override")?;

    let adapters = wire::open_adapters(&cli.db, Some(&docs_root))?;
    let mut config = IngestConfig::new(
        SubjectId::new(args.subject.clone()),
        args.workspace.clone(),
        docs_root,
    );
    config.rebuild = args.rebuild;
    config.batch_size = args.batch_size;
    config.max_skip_pct = args.max_skip_pct;

    let services = IngestServices {
        items: adapters.items.clone(),
        vectors: adapters.vectors.clone(),
        fts: adapters.fts.clone(),
        blobs: adapters.blobs.clone(),
        embedder: adapters.embedder.clone(),
    };

    eprintln!("ingesting subject={} ...", args.subject);
    let stats = rex_ingest::run(config, services)
        .await
        .map_err(anyhow::Error::from)?;
    eprintln!(
        "✔ ingested {} documents ({} questions, {} notes, {} skipped) in {:.1}s",
        stats.rows_questions + stats.rows_notes,
        stats.rows_questions,
        stats.rows_notes,
        stats.rows_skipped,
        stats.took_ms as f64 / 1000.0
    );
    eprintln!(
        "  pdfs: {} seen, {} anchored at page level, {} low-confidence, {} read-failed, {} not-found",
        stats.pdfs_seen,
        stats.pdfs_anchored,
        stats.pdfs_low_confidence,
        stats.pdfs_read_failed,
        stats.pdfs_not_found
    );
    Ok(())
}

async fn search(cli: &Cli, args: &SearchArgs) -> Result<()> {
    let adapters = wire::open_adapters(&cli.db, cli.docs_root.as_deref())?;
    let svc = wire::build_search_service(&adapters)?;
    let filters = filters_from(&args.subject, &args.topics, &args.schools, &args.paper_types);
    let query = SearchQuery {
        text: Some(args.text.clone()),
        filters,
        limit: args.limit,
        mode: args.mode.to_domain(),
        exact: args.exact,
        rerank: !args.no_rerank,
    };
    let resp = svc.search(query).await.map_err(anyhow::Error::from)?;
    output::render_search(&resp, cli.json);
    Ok(())
}

async fn filter(cli: &Cli, args: &FilterArgs) -> Result<()> {
    let adapters = wire::open_adapters(&cli.db, cli.docs_root.as_deref())?;
    let svc = wire::build_search_service(&adapters)?;
    let filters = filters_from(&args.subject, &args.topics, &args.schools, &args.paper_types);
    let resp = svc
        .filter(filters, args.limit, args.offset)
        .await
        .map_err(anyhow::Error::from)?;
    output::render_search(&resp, cli.json);
    Ok(())
}

async fn get(cli: &Cli, id: &str) -> Result<()> {
    let adapters = wire::open_adapters(&cli.db, cli.docs_root.as_deref())?;
    let svc = wire::build_search_service(&adapters)?;
    let id = DocumentId::parse(id).context("invalid document id")?;
    let doc = svc.get(&id).await.map_err(anyhow::Error::from)?;
    output::render_document(&doc, cli.json);
    Ok(())
}

async fn subjects(cli: &Cli) -> Result<()> {
    let adapters = wire::open_adapters(&cli.db, cli.docs_root.as_deref())?;
    let svc = wire::build_search_service(&adapters)?;
    let s = svc.list_subjects().await.map_err(anyhow::Error::from)?;
    output::render_subjects(&s, cli.json);
    Ok(())
}

async fn subject(cli: &Cli, id: &str) -> Result<()> {
    let adapters = wire::open_adapters(&cli.db, cli.docs_root.as_deref())?;
    let svc = wire::build_search_service(&adapters)?;
    let stats = svc
        .list_subject(&SubjectId::new(id))
        .await
        .map_err(anyhow::Error::from)?;
    output::render_subject(&stats, cli.json);
    Ok(())
}

async fn tag_values(cli: &Cli, args: &TagValuesArgs) -> Result<()> {
    let adapters = wire::open_adapters(&cli.db, cli.docs_root.as_deref())?;
    let svc = wire::build_search_service(&adapters)?;
    let field = TagField::from_db_str(&args.field)
        .with_context(|| format!("unknown tag field: {}", args.field))?;
    let counts = svc
        .facet_counts(&SubjectId::new(args.subject.clone()), field, Filters::default())
        .await
        .map_err(anyhow::Error::from)?;
    output::render_tag_values(&args.field, &counts, cli.json);
    Ok(())
}

async fn pdf_anchor(cli: &Cli, id: &str) -> Result<()> {
    let adapters = wire::open_adapters(&cli.db, cli.docs_root.as_deref())?;
    let svc = wire::build_search_service(&adapters)?;
    let id = DocumentId::parse(id).context("invalid document id")?;
    let anchor = svc.pdf_anchor(&id).await.map_err(anyhow::Error::from)?;
    output::render_pdf_anchor(&anchor, cli.json);
    Ok(())
}

fn filters_from(
    subject: &Option<String>,
    topics: &[String],
    schools: &[String],
    paper_types: &[String],
) -> Filters {
    Filters {
        subject: subject.as_ref().map(|s| SubjectId::new(s.clone())),
        topics: topics.iter().cloned().map(TagValue::new).collect(),
        schools: schools.iter().cloned().map(TagValue::new).collect(),
        paper_types: paper_types.iter().cloned().map(TagValue::new).collect(),
        ..Default::default()
    }
}
