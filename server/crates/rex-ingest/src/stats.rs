#[derive(Debug, Clone, Default)]
pub struct IngestStats {
    pub rows_questions: u64,
    pub rows_notes: u64,
    pub rows_skipped: u64,
    /// Rows whose `parent_id` referenced an id that wasn't ingested (usually
    /// because the parent row was itself skipped due to a malformed UUID).
    /// The reference is set to NULL at write time so the row still indexes;
    /// the parent→child link is the only thing lost.
    pub dangling_parent_nulled: u64,
    /// Same idea for `depends_on`: dangling sibling-ids are pruned from the
    /// list rather than failing the whole row.
    pub dangling_depends_pruned: u64,
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
