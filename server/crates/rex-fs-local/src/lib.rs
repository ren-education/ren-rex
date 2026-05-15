//! `LocalFsBlobStore` — a `BlobStore` backed by the local filesystem.
//!
//! Paths are joined under a configured root. Traversal outside the root
//! (via `..` segments) is rejected as `BadInput` to avoid surprise reads.

use std::path::{Component, Path, PathBuf};

use async_trait::async_trait;
use bytes::Bytes;
use rex_domain::{BlobStore, Error, Result};

pub struct LocalFsBlobStore {
    root: PathBuf,
}

impl LocalFsBlobStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn safe_join(&self, rel: &Path) -> Result<PathBuf> {
        for comp in rel.components() {
            if matches!(comp, Component::ParentDir) {
                return Err(Error::bad_input(format!(
                    "path traversal rejected: {}",
                    rel.display()
                )));
            }
        }
        Ok(self.root.join(rel))
    }
}

#[async_trait]
impl BlobStore for LocalFsBlobStore {
    async fn get(&self, path: &Path) -> Result<Bytes> {
        let full = self.safe_join(path)?;
        let bytes = tokio::fs::read(&full).await.map_err(|e| Error::Storage {
            source: Box::new(e),
        })?;
        Ok(Bytes::from(bytes))
    }

    async fn exists(&self, path: &Path) -> Result<bool> {
        let full = self.safe_join(path)?;
        Ok(tokio::fs::try_exists(&full)
            .await
            .map_err(|e| Error::Storage {
                source: Box::new(e),
            })?)
    }

    async fn list(&self, prefix: &Path) -> Result<Vec<PathBuf>> {
        let full = self.safe_join(prefix)?;
        let mut out = Vec::new();
        let mut walk = vec![full.clone()];
        while let Some(dir) = walk.pop() {
            let meta = match tokio::fs::metadata(&dir).await {
                Ok(m) => m,
                Err(_) => continue,
            };
            if !meta.is_dir() {
                if meta.is_file() {
                    if let Ok(rel) = dir.strip_prefix(&self.root) {
                        out.push(rel.to_path_buf());
                    }
                }
                continue;
            }
            let mut rd = tokio::fs::read_dir(&dir).await.map_err(|e| Error::Storage {
                source: Box::new(e),
            })?;
            while let Some(entry) = rd.next_entry().await.map_err(|e| Error::Storage {
                source: Box::new(e),
            })? {
                let p = entry.path();
                if p.is_dir() {
                    walk.push(p);
                } else if p.is_file() {
                    if let Ok(rel) = p.strip_prefix(&self.root) {
                        out.push(rel.to_path_buf());
                    }
                }
            }
        }
        out.sort();
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn get_reads_file() {
        let dir = tempfile::tempdir().unwrap();
        tokio::fs::write(dir.path().join("hello.txt"), b"hi").await.unwrap();
        let bs = LocalFsBlobStore::new(dir.path());
        let bytes = bs.get(Path::new("hello.txt")).await.unwrap();
        assert_eq!(&bytes[..], b"hi");
    }

    #[tokio::test]
    async fn exists_returns_correctly() {
        let dir = tempfile::tempdir().unwrap();
        tokio::fs::write(dir.path().join("x"), b"").await.unwrap();
        let bs = LocalFsBlobStore::new(dir.path());
        assert!(bs.exists(Path::new("x")).await.unwrap());
        assert!(!bs.exists(Path::new("y")).await.unwrap());
    }

    #[tokio::test]
    async fn list_returns_files_recursively() {
        let dir = tempfile::tempdir().unwrap();
        tokio::fs::create_dir(dir.path().join("sub")).await.unwrap();
        tokio::fs::write(dir.path().join("a.txt"), b"").await.unwrap();
        tokio::fs::write(dir.path().join("sub/b.txt"), b"").await.unwrap();
        let bs = LocalFsBlobStore::new(dir.path());
        let mut files = bs.list(Path::new("")).await.unwrap();
        files.sort();
        assert_eq!(files.len(), 2);
    }

    #[tokio::test]
    async fn path_traversal_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let bs = LocalFsBlobStore::new(dir.path());
        let err = bs.get(Path::new("../etc/passwd")).await.unwrap_err();
        assert!(matches!(err, Error::BadInput { .. }));
    }
}
