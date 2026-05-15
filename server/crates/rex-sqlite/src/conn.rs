//! SQLite connection setup: PRAGMAs, schema, version stamps.

use std::path::Path;

use rusqlite::Connection;
use thiserror::Error;

use crate::schema::{DDL, PRAGMAS, SCHEMA_VERSION};

#[derive(Debug, Error)]
pub enum SqliteOpenError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("schema mismatch: db version {db_version} != binary version {bin_version}")]
    SchemaMismatch {
        db_version: u32,
        bin_version: u32,
    },
    #[error("vector dimension mismatch: db {db} != embedder {embedder}")]
    DimensionMismatch { db: usize, embedder: usize },
}

pub fn open_db(path: &Path) -> Result<Connection, SqliteOpenError> {
    let conn = Connection::open(path)?;
    for pragma in PRAGMAS {
        // PRAGMA results are ignored; configuration may also return rows.
        let _ = conn.pragma_update(None, &pragma_key(pragma), pragma_value(pragma));
    }
    // Set journal_mode via execute_batch since pragma_update doesn't always apply WAL.
    conn.execute_batch("PRAGMA journal_mode = WAL")?;
    for sql in DDL {
        conn.execute(sql, [])?;
    }
    // Stamp version if missing.
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM schema_meta WHERE key='version'",
        [],
        |r| r.get(0),
    )?;
    if count == 0 {
        conn.execute(
            "INSERT INTO schema_meta (key, value) VALUES ('version', ?)",
            [SCHEMA_VERSION.to_string()],
        )?;
    } else {
        let v: String = conn.query_row(
            "SELECT value FROM schema_meta WHERE key='version'",
            [],
            |r| r.get(0),
        )?;
        let dbv: u32 = v.parse().unwrap_or(0);
        if dbv != SCHEMA_VERSION {
            return Err(SqliteOpenError::SchemaMismatch {
                db_version: dbv,
                bin_version: SCHEMA_VERSION,
            });
        }
    }
    Ok(conn)
}

fn pragma_key(p: &str) -> String {
    // "PRAGMA journal_mode = WAL" -> "journal_mode"
    p.trim_start_matches("PRAGMA ")
        .split('=')
        .next()
        .unwrap_or("")
        .trim()
        .to_string()
}

fn pragma_value(p: &str) -> String {
    p.split('=').nth(1).unwrap_or("").trim().to_string()
}
