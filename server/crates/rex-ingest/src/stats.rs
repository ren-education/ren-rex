#[derive(Debug, Clone, Default)]
pub struct IngestStats {
    pub rows_questions: u64,
    pub rows_notes: u64,
    pub rows_skipped: u64,
    pub pdfs_seen: u64,
    pub pdfs_anchored: u64,
    pub pdfs_low_confidence: u64,
    pub pdfs_read_failed: u64,
    pub pdfs_not_found: u64,
    pub took_ms: u64,
}

impl IngestStats {
    pub fn total_rows(&self) -> u64 {
        self.rows_questions + self.rows_notes + self.rows_skipped
    }
}
