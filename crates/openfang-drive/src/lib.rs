//! OpenFang Drive — virtual filesystem with intelligent auto-organization.
//!
//! Provides mountable drive volumes backed by pluggable storage backends,
//! background content indexing, classification rules, git repo management,
//! and unified search (metadata + semantic).

pub mod backend;
pub mod classify;
pub mod index;
pub mod ocr;
pub mod repos;
pub mod search;
pub mod volume;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use openfang_types::config::{DriveConfig, DriveRuleConfig};
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::info;

use crate::classify::ClassificationPipeline;
use crate::volume::DriveVolume;

/// Errors originating from the drive subsystem.
#[derive(Debug, Error)]
pub enum DriveError {
    #[error("Drive not found: {0}")]
    DriveNotFound(String),
    #[error("Path not found: {0}")]
    PathNotFound(String),
    #[error("Path denied: {0}")]
    PathDenied(String),
    #[error("Backend error: {0}")]
    Backend(String),
    #[error("Index error: {0}")]
    Index(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Database error: {0}")]
    Database(String),
    #[error("Drive is read-only: {0}")]
    ReadOnly(String),
    #[error("Invalid path: {0}")]
    InvalidPath(String),
}

pub type DriveResult<T> = Result<T, DriveError>;

/// Base folders created on a new main drive.
pub const BASE_FOLDERS: &[&str] = &[
    "Desktop",
    "Documents",
    "Downloads",
    "Music",
    "Photos",
    "Videos",
    "Repos",
    "Data",
];

/// The DriveManager coordinates all mounted drive volumes.
///
/// Lives on the kernel and resolves `drive:path` references to the correct backend.
pub struct DriveManager {
    /// Mounted volumes keyed by drive name.
    volumes: RwLock<HashMap<String, Arc<DriveVolume>>>,
    /// Classification pipeline for auto-organization.
    classification: Arc<ClassificationPipeline>,
    /// Home directory for default drive path resolution.
    home_dir: PathBuf,
}

impl DriveManager {
    /// Default drive configs when none are provided.
    fn default_configs(home_dir: &std::path::Path) -> Vec<DriveConfig> {
        vec![DriveConfig {
            name: "main".to_string(),
            backend: "local".to_string(),
            path: Some(home_dir.join("drive").to_string_lossy().to_string()),
            auto_organize: true,
            read_only: false,
        }]
    }

    /// Boot the drive manager synchronously (for kernel boot).
    pub fn new_sync(
        home_dir: PathBuf,
        drive_configs: &[DriveConfig],
        rule_configs: &[DriveRuleConfig],
        db_path: PathBuf,
    ) -> DriveResult<Self> {
        let classification = Arc::new(ClassificationPipeline::new(rule_configs));
        let mut volumes = HashMap::new();

        let configs = if drive_configs.is_empty() {
            Self::default_configs(&home_dir)
        } else {
            drive_configs.to_vec()
        };

        for cfg in &configs {
            let vol = DriveVolume::mount_sync(cfg, &db_path)?;
            info!("Mounted drive '{}' (backend={})", cfg.name, cfg.backend);
            volumes.insert(cfg.name.clone(), Arc::new(vol));
        }

        Ok(Self {
            volumes: RwLock::new(volumes),
            classification,
            home_dir,
        })
    }

    /// Boot the drive manager from kernel config (async).
    pub async fn new(
        home_dir: PathBuf,
        drive_configs: &[DriveConfig],
        rule_configs: &[DriveRuleConfig],
        db_path: PathBuf,
    ) -> DriveResult<Self> {
        let classification = Arc::new(ClassificationPipeline::new(rule_configs));
        let mut volumes = HashMap::new();

        let configs = if drive_configs.is_empty() {
            Self::default_configs(&home_dir)
        } else {
            drive_configs.to_vec()
        };

        for cfg in &configs {
            let vol = DriveVolume::mount(cfg, &db_path).await?;
            info!("Mounted drive '{}' (backend={})", cfg.name, cfg.backend);
            volumes.insert(cfg.name.clone(), Arc::new(vol));
        }

        Ok(Self {
            volumes: RwLock::new(volumes),
            classification,
            home_dir,
        })
    }

    /// Get a mounted drive volume by name.
    pub async fn get_volume(&self, name: &str) -> DriveResult<Arc<DriveVolume>> {
        self.volumes
            .read()
            .await
            .get(name)
            .cloned()
            .ok_or_else(|| DriveError::DriveNotFound(name.to_string()))
    }

    /// List all mounted drives.
    pub async fn list_drives(&self) -> Vec<DriveInfo> {
        let vols = self.volumes.read().await;
        vols.values()
            .map(|v| DriveInfo {
                name: v.name().to_string(),
                backend: v.backend_type().to_string(),
                path: v.root_path().map(|p| p.to_string_lossy().to_string()),
                auto_organize: v.auto_organize(),
                read_only: v.read_only(),
            })
            .collect()
    }

    /// Create and mount a new drive at runtime.
    pub async fn create_drive(&self, cfg: &DriveConfig, db_path: &std::path::Path) -> DriveResult<()> {
        let vol = DriveVolume::mount(cfg, db_path).await?;
        info!("Created and mounted drive '{}'", cfg.name);
        self.volumes
            .write()
            .await
            .insert(cfg.name.clone(), Arc::new(vol));
        Ok(())
    }

    /// Unmount and remove a drive.
    pub async fn remove_drive(&self, name: &str) -> DriveResult<()> {
        self.volumes
            .write()
            .await
            .remove(name)
            .ok_or_else(|| DriveError::DriveNotFound(name.to_string()))?;
        info!("Removed drive '{name}'");
        Ok(())
    }

    /// Get the classification pipeline.
    pub fn classification(&self) -> &Arc<ClassificationPipeline> {
        &self.classification
    }

    /// Home directory for path resolution.
    pub fn home_dir(&self) -> &PathBuf {
        &self.home_dir
    }

    /// Resolve `drive:path` to (drive_name, path_within_drive).
    pub fn parse_drive_path(input: &str) -> Option<(&str, &str)> {
        input.split_once(':')
    }

    /// Validate a path component — reject `..` traversal attempts.
    pub fn validate_path(path: &str) -> DriveResult<()> {
        if path.contains("..") {
            return Err(DriveError::InvalidPath(
                "Path traversal (..) is not allowed".to_string(),
            ));
        }
        Ok(())
    }
}

/// Summary info about a mounted drive.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DriveInfo {
    pub name: String,
    pub backend: String,
    pub path: Option<String>,
    pub auto_organize: bool,
    pub read_only: bool,
}
