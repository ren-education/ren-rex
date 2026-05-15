//! Clap definitions. Every subcommand here must have a parity counterpart on
//! the rex-api side, except the explicit CLI-only set (ingest, serve).

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "rex", version, about = "rex: PDF search & navigator")]
pub struct Cli {
    /// Emit JSON results instead of the pretty-printed default.
    #[arg(long, global = true)]
    pub json: bool,

    /// Stubbed for future remote-client mode. Currently exits with code 64.
    #[arg(long, global = true)]
    pub remote: Option<String>,

    /// Path to the rex SQLite database. Default: ./rex.db
    #[arg(long, global = true, env = "REX_DB", default_value = "rex.db")]
    pub db: PathBuf,

    /// Root containing PDF files. Used by all subcommands that may resolve PDFs.
    #[arg(long, global = true, env = "REX_DOCS_ROOT")]
    pub docs_root: Option<PathBuf>,

    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Ingest a subject's questions.jsonl + notes.jsonl into the index.
    Ingest(IngestArgs),
    /// Dry-run schema check over JSONL files. No DB, no embedder, no PDFs.
    ///
    /// Use this BEFORE `rex ingest` to catch shape mismatches between the
    /// input files and the rex JsonlRow schema. Validation reuses the exact
    /// same serde parser as ingest (including `#[serde(deny_unknown_fields)]`
    /// on both `JsonlRow` and `JsonlTags`), so "passes validate" guarantees
    /// "will not be skipped by ingest". Typically sub-second on a 10k-row file.
    ///
    /// TWO INPUT MODES:
    ///   * --file <path> --kind <question|note>
    ///       Validate one JSONL file directly. `--kind` tells the
    ///       domain-mapping step which body field to expect (`question`
    ///       for questions.jsonl, `content` for notes.jsonl).
    ///   * --subject <id> --workspace <path>
    ///       Validate the standard pair under
    ///       <workspace>/<subject>/reference/{questions,notes}.jsonl —
    ///       the same layout `rex ingest` reads.
    ///
    /// OUTPUT:
    ///   Errors are bucketed by signature (serde's `at line N column M`
    ///   suffix is stripped) so a single systemic problem like "all 10k MCQ
    ///   rows have object-shaped `options`" prints as one bucket with a
    ///   count, not 10k log lines. Each bucket also includes a best-effort
    ///   field name and the first few line numbers so you can
    ///   `sed -n '3p' file.jsonl` and eyeball the row.
    ///
    /// EXIT CODES:
    ///   0  every row parsed cleanly
    ///   1  I/O error (file unreadable, etc.)
    ///   2  one or more rows failed validation
    ///
    /// Pass `--json` (the global flag) to emit a machine-readable report
    /// suitable for scripting or CI.
    ///
    /// EXAMPLES:
    ///   rex validate --subject h2physics --workspace ren-subjects/workspace
    ///   rex validate --file ren-subjects/workspace/h2physics/reference/notes.jsonl --kind note
    ///   rex validate --json --subject h2history --workspace ren-subjects/workspace > report.json
    #[command(verbatim_doc_comment)]
    Validate(ValidateArgs),
    /// Serve the rex HTTP API. (Stub in v1 until rex-api lands.)
    Serve(ServeArgs),
    /// Hybrid / keyword / vector search.
    Search(SearchArgs),
    /// Filter-only browse (no text query).
    Filter(FilterArgs),
    /// Fetch a single document by id.
    Get { id: String },
    /// List subjects.
    Subjects,
    /// Show stats for one subject.
    Subject { id: String },
    /// Enumerate values + counts for a tag field within a subject.
    TagValues(TagValuesArgs),
    /// Return the PDF anchor for a document.
    PdfAnchor { id: String },
}

#[derive(Debug, clap::Args)]
pub struct IngestArgs {
    /// Subject id (e.g. h2physics, h2history).
    #[arg(long)]
    pub subject: String,
    /// ren-subjects/workspace path.
    #[arg(long)]
    pub workspace: PathBuf,
    /// ren-subjects/docs path. Defaults to --docs-root if omitted.
    #[arg(long)]
    pub docs_root_override: Option<PathBuf>,
    /// Replace this subject's data entirely (default).
    #[arg(long, default_value_t = true)]
    pub rebuild: bool,
    /// Abort ingest if percentage of rows skipped exceeds this threshold.
    /// Default 20.0 — calibrated to h2physics's ~16% malformed-UUID rate.
    /// Run `rex validate` first to see the actual error mix.
    #[arg(long, default_value_t = 20.0)]
    pub max_skip_pct: f64,
    /// Embedding batch size.
    #[arg(long, default_value_t = 256)]
    pub batch_size: usize,
}

#[derive(Debug, clap::Args)]
pub struct ValidateArgs {
    /// Single JSONL file to validate. Mutually exclusive with `--subject`.
    /// When used, `--kind` is required.
    #[arg(long, conflicts_with_all = ["subject", "workspace"])]
    pub file: Option<PathBuf>,

    /// Row kind for `--file` mode. Determines which body field is expected
    /// (`question` for questions, `content` for notes). Ignored in subject mode.
    #[arg(long, value_enum, requires = "file")]
    pub kind: Option<KindArg>,

    /// Subject id (e.g. h2physics, h2history). Validates both
    /// questions.jsonl and notes.jsonl under the workspace.
    #[arg(long, requires = "workspace")]
    pub subject: Option<String>,

    /// ren-subjects/workspace path — the same path you'd pass to `rex ingest`.
    #[arg(long, requires = "subject")]
    pub workspace: Option<PathBuf>,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum KindArg {
    Question,
    Note,
}

impl KindArg {
    pub fn to_domain(&self) -> rex_domain::DocumentKind {
        match self {
            KindArg::Question => rex_domain::DocumentKind::Question,
            KindArg::Note => rex_domain::DocumentKind::Note,
        }
    }
}

#[derive(Debug, clap::Args)]
pub struct ServeArgs {
    #[arg(long, default_value = "127.0.0.1:8080")]
    pub bind: String,
    #[arg(long, default_value = "*")]
    pub cors_allow: String,
    #[arg(long)]
    pub no_reranker: bool,
    #[arg(long)]
    pub warm: bool,
}

#[derive(Debug, clap::Args)]
pub struct SearchArgs {
    /// Query text.
    pub text: String,
    #[arg(long)]
    pub subject: Option<String>,
    #[arg(long = "topic", value_name = "TAG")]
    pub topics: Vec<String>,
    #[arg(long = "school", value_name = "TAG")]
    pub schools: Vec<String>,
    #[arg(long = "paper-type", value_name = "TAG")]
    pub paper_types: Vec<String>,
    #[arg(long, value_enum, default_value_t = ModeArg::Hybrid)]
    pub mode: ModeArg,
    /// FTS5 phrase exact match (auto-quotes the whole query). Only honored
    /// when mode=Bm25Only.
    #[arg(long)]
    pub exact: bool,
    /// Disable reranking even in Hybrid mode.
    #[arg(long)]
    pub no_rerank: bool,
    #[arg(long, default_value_t = 10)]
    pub limit: usize,
}

#[derive(Debug, clap::Args)]
pub struct FilterArgs {
    #[arg(long)]
    pub subject: Option<String>,
    #[arg(long = "topic", value_name = "TAG")]
    pub topics: Vec<String>,
    #[arg(long = "school", value_name = "TAG")]
    pub schools: Vec<String>,
    #[arg(long = "paper-type", value_name = "TAG")]
    pub paper_types: Vec<String>,
    #[arg(long, default_value_t = 20)]
    pub limit: usize,
    #[arg(long, default_value_t = 0)]
    pub offset: usize,
}

#[derive(Debug, clap::Args)]
pub struct TagValuesArgs {
    #[arg(long)]
    pub subject: String,
    /// One of: topics, question_types, exam_systems, paper_types, schools, source_types.
    #[arg(long)]
    pub field: String,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum ModeArg {
    Hybrid,
    Bm25,
    Vector,
}

impl ModeArg {
    pub fn to_domain(&self) -> rex_domain::SearchMode {
        match self {
            ModeArg::Hybrid => rex_domain::SearchMode::Hybrid,
            ModeArg::Bm25 => rex_domain::SearchMode::Bm25Only,
            ModeArg::Vector => rex_domain::SearchMode::VectorOnly,
        }
    }
}
