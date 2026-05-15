use std::path::PathBuf;

use rex_domain::SubjectId;

#[derive(Debug, Clone)]
pub struct IngestConfig {
    pub subject: SubjectId,
    /// Root of `ren-subjects/workspace/`.
    pub workspace_root: PathBuf,
    /// Root of `ren-subjects/docs/`.
    pub docs_root: PathBuf,
    /// Replace this subject's data entirely before ingesting. Defaults to true
    /// in v1 (no incremental ingest).
    pub rebuild: bool,
    /// Embedding batch size.
    pub batch_size: usize,
    /// Abort if (rows_skipped / rows_total) exceeds this percentage.
    pub max_skip_pct: f64,
    /// Minimum n-gram-Jaccard confidence to attach a page-level anchor.
    pub anchor_confidence_threshold: f32,
}

impl IngestConfig {
    pub fn new(
        subject: SubjectId,
        workspace_root: impl Into<PathBuf>,
        docs_root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            subject,
            workspace_root: workspace_root.into(),
            docs_root: docs_root.into(),
            rebuild: true,
            batch_size: 256,
            // 20% was chosen empirically: h2physics's notes file has 16.2%
            // of rows with malformed UUIDs upstream. Use `rex validate` to
            // surface the actual error distribution before tightening this.
            max_skip_pct: 20.0,
            anchor_confidence_threshold: 0.6,
        }
    }
}
