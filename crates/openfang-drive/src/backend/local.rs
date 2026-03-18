//! Local filesystem storage backend.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::fs;

use super::{FileInfo, StorageBackend};
use crate::{DriveError, DriveResult};

/// Local filesystem backend — files stored directly on disk.
pub struct LocalBackend {
    root: PathBuf,
}

impl LocalBackend {
    /// Create a new local backend rooted at the given directory (async).
    pub async fn new(root: PathBuf) -> DriveResult<Self> {
        fs::create_dir_all(&root).await?;
        Ok(Self { root })
    }

    /// Create a new local backend synchronously (for kernel boot).
    pub fn new_sync(root: PathBuf) -> DriveResult<Self> {
        std::fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    /// Resolve a drive-relative path to an absolute filesystem path.
    fn resolve(&self, path: &str) -> PathBuf {
        let clean = path.trim_start_matches('/');
        self.root.join(clean)
    }

    /// Get the root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

fn system_time_to_iso(st: std::time::SystemTime) -> String {
    let dt: DateTime<Utc> = st.into();
    dt.to_rfc3339()
}

fn mime_from_path(path: &Path) -> String {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "pdf" => "application/pdf",
        "json" => "application/json",
        "toml" => "application/toml",
        "yaml" | "yml" => "application/yaml",
        "xml" => "application/xml",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" | "mjs" => "application/javascript",
        "ts" | "tsx" => "application/typescript",
        "rs" => "text/x-rust",
        "py" => "text/x-python",
        "go" => "text/x-go",
        "c" | "h" => "text/x-c",
        "cpp" | "hpp" | "cc" => "text/x-c++",
        "java" => "text/x-java",
        "md" => "text/markdown",
        "txt" | "log" | "csv" => "text/plain",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "zip" => "application/zip",
        "tar" => "application/x-tar",
        "gz" | "tgz" => "application/gzip",
        "doc" | "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" | "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" | "pptx" => {
            "application/vnd.openxmlformats-officedocument.presentationml.presentation"
        }
        _ => "application/octet-stream",
    }
    .to_string()
}

fn file_info_from_metadata(
    name: String,
    rel_path: String,
    metadata: &std::fs::Metadata,
    full_path: &Path,
) -> FileInfo {
    let is_dir = metadata.is_dir();
    FileInfo {
        name,
        path: rel_path,
        is_dir,
        size_bytes: if is_dir { 0 } else { metadata.len() },
        mime_type: if is_dir {
            String::new()
        } else {
            mime_from_path(full_path)
        },
        created_at: metadata
            .created()
            .map(system_time_to_iso)
            .unwrap_or_default(),
        modified_at: metadata
            .modified()
            .map(system_time_to_iso)
            .unwrap_or_default(),
    }
}

#[async_trait]
impl StorageBackend for LocalBackend {
    async fn list(&self, path: &str) -> DriveResult<Vec<FileInfo>> {
        let abs = self.resolve(path);
        if !abs.exists() {
            return Err(DriveError::PathNotFound(path.to_string()));
        }
        let mut entries = Vec::new();
        let mut read_dir = fs::read_dir(&abs).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let meta = entry.metadata().await?;
            let name = entry.file_name().to_string_lossy().to_string();
            let rel = format!(
                "{}/{}",
                path.trim_end_matches('/'),
                name
            );
            entries.push(file_info_from_metadata(name, rel, &meta, &entry.path()));
        }
        entries.sort_by(|a, b| {
            // Directories first, then alphabetical
            b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name))
        });
        Ok(entries)
    }

    async fn read(&self, path: &str) -> DriveResult<Vec<u8>> {
        let abs = self.resolve(path);
        if !abs.exists() {
            return Err(DriveError::PathNotFound(path.to_string()));
        }
        Ok(fs::read(&abs).await?)
    }

    async fn write(&self, path: &str, data: &[u8]) -> DriveResult<()> {
        let abs = self.resolve(path);
        if let Some(parent) = abs.parent() {
            fs::create_dir_all(parent).await?;
        }
        Ok(fs::write(&abs, data).await?)
    }

    async fn delete(&self, path: &str) -> DriveResult<()> {
        let abs = self.resolve(path);
        if !abs.exists() {
            return Err(DriveError::PathNotFound(path.to_string()));
        }
        if abs.is_dir() {
            fs::remove_dir(&abs).await?;
        } else {
            fs::remove_file(&abs).await?;
        }
        Ok(())
    }

    async fn rename(&self, from: &str, to: &str) -> DriveResult<()> {
        let abs_from = self.resolve(from);
        let abs_to = self.resolve(to);
        if !abs_from.exists() {
            return Err(DriveError::PathNotFound(from.to_string()));
        }
        if let Some(parent) = abs_to.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::rename(&abs_from, &abs_to).await?;
        Ok(())
    }

    async fn copy_file(&self, from: &str, to: &str) -> DriveResult<()> {
        let abs_from = self.resolve(from);
        let abs_to = self.resolve(to);
        if !abs_from.exists() {
            return Err(DriveError::PathNotFound(from.to_string()));
        }
        if let Some(parent) = abs_to.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::copy(&abs_from, &abs_to).await?;
        Ok(())
    }

    async fn stat(&self, path: &str) -> DriveResult<FileInfo> {
        let abs = self.resolve(path);
        if !abs.exists() {
            return Err(DriveError::PathNotFound(path.to_string()));
        }
        let meta = fs::metadata(&abs).await?;
        let name = abs
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        Ok(file_info_from_metadata(
            name,
            path.to_string(),
            &meta,
            &abs,
        ))
    }

    async fn exists(&self, path: &str) -> DriveResult<bool> {
        Ok(self.resolve(path).exists())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_local_backend_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let backend = LocalBackend::new(tmp.path().to_path_buf()).await.unwrap();

        // Write
        backend
            .write("/test/hello.txt", b"Hello, Drive!")
            .await
            .unwrap();

        // Exists
        assert!(backend.exists("/test/hello.txt").await.unwrap());
        assert!(!backend.exists("/test/nope.txt").await.unwrap());

        // Read
        let data = backend.read("/test/hello.txt").await.unwrap();
        assert_eq!(data, b"Hello, Drive!");

        // Stat
        let info = backend.stat("/test/hello.txt").await.unwrap();
        assert_eq!(info.name, "hello.txt");
        assert_eq!(info.size_bytes, 13);
        assert_eq!(info.mime_type, "text/plain");
        assert!(!info.is_dir);

        // List
        let entries = backend.list("/test").await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "hello.txt");

        // Copy
        backend
            .copy_file("/test/hello.txt", "/test/copy.txt")
            .await
            .unwrap();
        assert!(backend.exists("/test/copy.txt").await.unwrap());

        // Rename
        backend
            .rename("/test/copy.txt", "/test/moved.txt")
            .await
            .unwrap();
        assert!(!backend.exists("/test/copy.txt").await.unwrap());
        assert!(backend.exists("/test/moved.txt").await.unwrap());

        // Delete
        backend.delete("/test/moved.txt").await.unwrap();
        assert!(!backend.exists("/test/moved.txt").await.unwrap());
    }
}
