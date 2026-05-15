//! rex-sqlite: SQLite + FTS5 adapter implementing ItemStore, FtsIndex, VectorStore.
//!
//! ## Vector storage note (deviation from spec)
//!
//! The spec describes using the `sqlite-vec` extension for KNN. For v1, we
//! store vectors as `BLOB` columns and compute cosine similarity in Rust over
//! the filtered candidate set. Performance is fine for corpora up to ~50K
//! items (the size we expect across all `ren-subjects` collections). The
//! `VectorStore` trait surface is identical, so swapping to `sqlite-vec`
//! later is a single-crate change.

mod conn;
mod schema;
mod sql;
mod store;

pub use conn::{open_db, SqliteOpenError};
pub use store::SqliteStore;
