use thiserror::Error;

#[derive(Debug, Error)]
pub enum IngestError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("schema drift: {skipped}/{total} rows skipped (>{threshold_pct}%); sample errors:\n{}", sample.join("\n"))]
    SchemaDrift {
        skipped: u64,
        total: u64,
        threshold_pct: f64,
        sample: Vec<String>,
    },

    #[error("domain error: {0}")]
    Domain(#[from] rex_domain::Error),

    #[error("missing input file: {0}")]
    MissingInput(String),
}
