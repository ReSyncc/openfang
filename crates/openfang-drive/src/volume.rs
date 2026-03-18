//! DriveVolume — a single mounted drive with its backend and metadata index.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use openfang_types::config::DriveConfig;
use tracing::info;

use crate::backend::local::LocalBackend;
use crate::backend::{FileInfo, StorageBackend};
use crate::index::DriveIndex;
use crate::{DriveError, DriveResult, BASE_FOLDERS};

/// A single mounted drive volume.
pub struct DriveVolume {
    name: String,
    backend_type: String,
    backend: Arc<dyn StorageBackend>,
    index: DriveIndex,
    auto_organize: bool,
    read_only: bool,
    /// Root path (only for local backends).
    root_path: Option<PathBuf>,
}

impl DriveVolume {
    /// Expand ~ to home directory in a path string.
    fn expand_path(path: &str) -> PathBuf {
        if path.starts_with("~/") || path == "~" {
            if let Some(home) = dirs::home_dir() {
                home.join(path.strip_prefix("~/").unwrap_or(path))
            } else {
                PathBuf::from(path)
            }
        } else {
            PathBuf::from(path)
        }
    }

    /// Mount a drive from config (synchronous — safe for kernel boot).
    pub fn mount_sync(cfg: &DriveConfig, db_path: &Path) -> DriveResult<Self> {
        let (backend, root_path): (Arc<dyn StorageBackend>, Option<PathBuf>) = match cfg
            .backend
            .as_str()
        {
            "local" => {
                let path = cfg.path.as_deref().ok_or_else(|| {
                    DriveError::Backend("Local backend requires 'path' field".to_string())
                })?;
                let expanded = Self::expand_path(path);
                let lb = LocalBackend::new_sync(expanded.clone())?;
                // Create base folders synchronously
                if cfg.name == "main" && !cfg.read_only {
                    for folder in BASE_FOLDERS {
                        let folder_path = expanded.join(folder);
                        if !folder_path.exists() {
                            let keep = folder_path.join(".keep");
                            std::fs::create_dir_all(&folder_path)?;
                            let _ = std::fs::write(&keep, b"");
                            info!("Created base folder: {folder}");
                        }
                    }
                }
                (Arc::new(lb), Some(expanded))
            }
            other => {
                return Err(DriveError::Backend(format!(
                    "Unsupported backend: {other}. Only 'local' is currently supported."
                )));
            }
        };

        let index_path = db_path.join(format!("drive_{}.db", cfg.name));
        let index = DriveIndex::open(&index_path)?;

        Ok(Self {
            name: cfg.name.clone(),
            backend_type: cfg.backend.clone(),
            backend,
            index,
            auto_organize: cfg.auto_organize,
            read_only: cfg.read_only,
            root_path,
        })
    }

    /// Mount a drive from config (async).
    pub async fn mount(cfg: &DriveConfig, db_path: &Path) -> DriveResult<Self> {
        let (backend, root_path): (Arc<dyn StorageBackend>, Option<PathBuf>) = match cfg
            .backend
            .as_str()
        {
            "local" => {
                let path = cfg.path.as_deref().ok_or_else(|| {
                    DriveError::Backend("Local backend requires 'path' field".to_string())
                })?;
                let expanded = Self::expand_path(path);
                let lb = LocalBackend::new(expanded.clone()).await?;
                (Arc::new(lb), Some(expanded))
            }
            other => {
                return Err(DriveError::Backend(format!(
                    "Unsupported backend: {other}. Only 'local' is currently supported."
                )));
            }
        };

        // Create base folders for the main drive.
        if cfg.name == "main" && !cfg.read_only {
            for folder in BASE_FOLDERS {
                if !backend.exists(folder).await? {
                    backend.write(&format!("{folder}/.keep"), b"").await?;
                    info!("Created base folder: {folder}");
                }
            }
        }

        let index_path = db_path.join(format!("drive_{}.db", cfg.name));
        let index = DriveIndex::open(&index_path)?;

        Ok(Self {
            name: cfg.name.clone(),
            backend_type: cfg.backend.clone(),
            backend,
            index,
            auto_organize: cfg.auto_organize,
            read_only: cfg.read_only,
            root_path,
        })
    }

    // -- Accessors --

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn backend_type(&self) -> &str {
        &self.backend_type
    }

    pub fn auto_organize(&self) -> bool {
        self.auto_organize
    }

    pub fn read_only(&self) -> bool {
        self.read_only
    }

    pub fn root_path(&self) -> Option<&Path> {
        self.root_path.as_deref()
    }

    pub fn index(&self) -> &DriveIndex {
        &self.index
    }

    pub fn backend(&self) -> &Arc<dyn StorageBackend> {
        &self.backend
    }

    // -- Delegated operations with read-only enforcement --

    pub async fn list(&self, path: &str) -> DriveResult<Vec<FileInfo>> {
        self.backend.list(path).await
    }

    pub async fn read(&self, path: &str) -> DriveResult<Vec<u8>> {
        self.backend.read(path).await
    }

    pub async fn write(&self, path: &str, data: &[u8]) -> DriveResult<()> {
        if self.read_only {
            return Err(DriveError::ReadOnly(self.name.clone()));
        }
        self.backend.write(path, data).await
    }

    pub async fn delete(&self, path: &str) -> DriveResult<()> {
        if self.read_only {
            return Err(DriveError::ReadOnly(self.name.clone()));
        }
        self.backend.delete(path).await?;
        // Remove from index
        self.index.remove_by_path(&self.name, path)?;
        Ok(())
    }

    pub async fn rename(&self, from: &str, to: &str) -> DriveResult<()> {
        if self.read_only {
            return Err(DriveError::ReadOnly(self.name.clone()));
        }
        self.backend.rename(from, to).await
    }

    pub async fn copy_file(&self, from: &str, to: &str) -> DriveResult<()> {
        if self.read_only {
            return Err(DriveError::ReadOnly(self.name.clone()));
        }
        self.backend.copy_file(from, to).await
    }

    pub async fn stat(&self, path: &str) -> DriveResult<FileInfo> {
        self.backend.stat(path).await
    }

    pub async fn exists(&self, path: &str) -> DriveResult<bool> {
        self.backend.exists(path).await
    }
}
