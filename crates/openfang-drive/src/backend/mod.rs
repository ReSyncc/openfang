//! Storage backend trait and implementations.

pub mod local;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::DriveResult;

/// Metadata about a file or directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    /// File or directory name (leaf).
    pub name: String,
    /// Full path relative to the drive root.
    pub path: String,
    /// Whether this is a directory.
    pub is_dir: bool,
    /// File size in bytes (0 for directories).
    pub size_bytes: u64,
    /// MIME type (empty for directories).
    pub mime_type: String,
    /// Created timestamp (ISO 8601).
    pub created_at: String,
    /// Modified timestamp (ISO 8601).
    pub modified_at: String,
}

/// A pluggable storage backend for drive volumes.
///
/// Only `LocalBackend` is implemented now. The trait is designed so that
/// `GoogleDriveBackend`, `S3Backend`, `SmbBackend`, etc. can be added later.
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// List files and directories at the given path.
    async fn list(&self, path: &str) -> DriveResult<Vec<FileInfo>>;
    /// Read file contents.
    async fn read(&self, path: &str) -> DriveResult<Vec<u8>>;
    /// Write file contents (creates parent directories as needed).
    async fn write(&self, path: &str, data: &[u8]) -> DriveResult<()>;
    /// Delete a file or empty directory.
    async fn delete(&self, path: &str) -> DriveResult<()>;
    /// Rename or move a file/directory.
    async fn rename(&self, from: &str, to: &str) -> DriveResult<()>;
    /// Copy a file.
    async fn copy_file(&self, from: &str, to: &str) -> DriveResult<()>;
    /// Get file metadata.
    async fn stat(&self, path: &str) -> DriveResult<FileInfo>;
    /// Check if a path exists.
    async fn exists(&self, path: &str) -> DriveResult<bool>;
}
