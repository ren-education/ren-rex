//! Subcommand dispatch.

use anyhow::{Context, Result};
use rex_domain::{
    DocumentId, Filters, SearchQuery, SubjectId, TagField, TagValue,
};
use rex_ingest::{IngestConfig, IngestServices};

use crate::cli::{
    Cli, Cmd, FilterArgs, IngestArgs, SearchArgs, ServeArgs, TagValuesArgs, ValidateArgs,
};
use crate::output;
use crate::wire;

pub async fn dispatch(cli: Cli) -> i32 {
    // `validate` is the one subcommand that uses a 3-valued exit code, so
    // dispatch it directly and bypass the Result<()> -> {0,1} mapping below.
    if let Cmd::Validate(args) = &cli.cmd {
        return match validate(&cli, args) {
            Ok(code) => code,
            Err(e) => {
                eprintln!("error: {e:#}");
                1
            }
        };
    }

    let result: Result<()> = match &cli.cmd {
        Cmd::Ingest(args) => ingest(&cli, args).await,
        Cmd::Validate(_) => unreachable!("handled above"),
        Cmd::Serve(args) => serve(&cli, args).await,
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
    if stats.dangling_parent_nulled + stats.dangling_depends_pruned > 0 {
        eprintln!(
            "  refs: {} dangling parent_id nulled, {} dangling depends_on pruned \
             (parents of these refs were themselves skipped)",
            stats.dangling_parent_nulled, stats.dangling_depends_pruned,
        );
    }
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

/// Returns the process exit code directly: 0 clean, 2 if any rows failed.
fn validate(cli: &Cli, args: &ValidateArgs) -> Result<i32> {
    use rex_ingest::{validate_file, validate_subject, ValidateFileReport};

    let reports: Vec<ValidateFileReport> = match (&args.file, &args.subject, &args.workspace) {
        (Some(path), _, _) => {
            let kind = args
                .kind
                .as_ref()
                .context("--file requires --kind question|note")?
                .to_domain();
            let subject = SubjectId::new("validate".to_string());
            vec![validate_file(path, kind, &subject)
                .with_context(|| format!("reading {}", path.display()))?]
        }
        (None, Some(subject), Some(workspace)) => {
            let sub = SubjectId::new(subject.clone());
            validate_subject(&sub, workspace)
                .with_context(|| format!("reading subject {} under {}", subject, workspace.display()))?
                .files
        }
        _ => {
            anyhow::bail!(
                "specify either --file <path> --kind <question|note> \
                 or --subject <id> --workspace <path>"
            );
        }
    };

    let any_failed = reports.iter().any(|r| !r.is_clean());
    output::render_validate_reports(&reports, cli.json);
    Ok(if any_failed { 2 } else { 0 })
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

async fn serve(cli: &Cli, args: &ServeArgs) -> Result<()> {
    use std::sync::Arc;

    let adapters = wire::open_adapters(&cli.db, cli.docs_root.as_deref())?;
    let svc = Arc::new(wire::build_search_service(&adapters)?);

    let state = Arc::new(
        rex_api::AppState::builder()
            .service(svc)
            .blobs(adapters.blobs.clone())
            .build()
            .map_err(|e| anyhow::anyhow!("failed to build AppState: {e}"))?,
    );

    let addr: std::net::SocketAddr = args
        .bind
        .parse()
        .with_context(|| format!("invalid --bind {:?}", args.bind))?;

    eprintln!("  db        = {}", cli.db.display());
    eprintln!(
        "  docs-root = {}",
        cli.docs_root
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(none)".into())
    );
    let _ = args.cors_allow.clone();
    let _ = args.no_reranker;
    let _ = args.warm;

    rex_api::run(state, addr).await.map_err(anyhow::Error::from)
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
