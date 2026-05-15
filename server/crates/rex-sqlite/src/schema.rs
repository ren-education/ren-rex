//! SQL DDL and schema version checks.

pub const SCHEMA_VERSION: u32 = 1;

pub const DDL: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS subjects (
        id         TEXT PRIMARY KEY,
        created_at INTEGER NOT NULL,
        item_count INTEGER NOT NULL DEFAULT 0
    )",
    "CREATE TABLE IF NOT EXISTS documents (
        id                   TEXT PRIMARY KEY,
        subject_id           TEXT NOT NULL REFERENCES subjects(id) ON DELETE CASCADE,
        kind                 TEXT NOT NULL CHECK (kind IN ('Question','Note')),
        parent_id            TEXT REFERENCES documents(id),
        depends_on_json      TEXT NOT NULL DEFAULT '[]',
        number               TEXT,
        source_path          TEXT NOT NULL,
        context              TEXT,
        question             TEXT,
        answer               TEXT,
        notes                TEXT,
        mark                 INTEGER,
        options_json         TEXT,
        keywords_json        TEXT NOT NULL DEFAULT '[]',
        pdf_path             TEXT,
        pdf_page             INTEGER,
        pdf_bbox_json        TEXT,
        pdf_confidence       REAL,
        pdf_fallback_reason  TEXT CHECK (pdf_fallback_reason IS NULL
                                       OR pdf_fallback_reason IN ('LowConfidence','PdfReadFailed','PdfNotFound')),
        created_at           INTEGER NOT NULL
    )",
    "CREATE INDEX IF NOT EXISTS idx_documents_subject ON documents(subject_id)",
    "CREATE INDEX IF NOT EXISTS idx_documents_kind    ON documents(subject_id, kind)",
    "CREATE INDEX IF NOT EXISTS idx_documents_parent  ON documents(parent_id)",
    "CREATE TABLE IF NOT EXISTS document_tags (
        document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
        field       TEXT NOT NULL,
        value       TEXT NOT NULL,
        PRIMARY KEY (document_id, field, value)
    )",
    "CREATE INDEX IF NOT EXISTS idx_tags_field_value ON document_tags(field, value)",
    "CREATE INDEX IF NOT EXISTS idx_tags_doc_field   ON document_tags(document_id, field)",
    "CREATE VIRTUAL TABLE IF NOT EXISTS document_fts USING fts5(
        document_id UNINDEXED,
        search_text,
        tokenize='porter unicode61 remove_diacritics 1'
    )",
    "CREATE TABLE IF NOT EXISTS document_vec (
        document_id TEXT PRIMARY KEY REFERENCES documents(id) ON DELETE CASCADE,
        subject_id  TEXT NOT NULL,
        embedding   BLOB NOT NULL
    )",
    "CREATE INDEX IF NOT EXISTS idx_document_vec_subject ON document_vec(subject_id)",
    "CREATE TABLE IF NOT EXISTS ingest_log (
        id          INTEGER PRIMARY KEY AUTOINCREMENT,
        subject_id  TEXT NOT NULL,
        started_at  INTEGER NOT NULL,
        finished_at INTEGER,
        items_added INTEGER NOT NULL DEFAULT 0,
        items_total INTEGER NOT NULL DEFAULT 0,
        pdfs_seen   INTEGER NOT NULL DEFAULT 0,
        pdfs_failed INTEGER NOT NULL DEFAULT 0,
        status      TEXT NOT NULL CHECK (status IN ('running','ok','failed')),
        error       TEXT
    )",
    "CREATE TABLE IF NOT EXISTS schema_meta (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    )",
];

pub const PRAGMAS: &[&str] = &[
    "PRAGMA journal_mode = WAL",
    "PRAGMA synchronous = NORMAL",
    "PRAGMA temp_store = MEMORY",
    "PRAGMA mmap_size = 268435456",
    "PRAGMA foreign_keys = ON",
];
