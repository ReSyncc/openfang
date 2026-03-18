//! Drive index — coordinates metadata storage and content extraction queues.

pub mod content;
pub mod metadata;

use std::path::Path;

use rusqlite::Connection;

use crate::{DriveError, DriveResult};
pub use metadata::DriveFileEntry;

/// Pipeline processing status for a file entry.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "status", content = "error")]
pub enum PipelineStatus {
    NotNeeded,
    Pending,
    Processing,
    Complete,
    Failed(String),
}

impl PipelineStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::NotNeeded => "not_needed",
            Self::Pending => "pending",
            Self::Processing => "processing",
            Self::Complete => "complete",
            Self::Failed(_) => "failed",
        }
    }

    pub fn from_db(status: &str, error: Option<String>) -> Self {
        match status {
            "not_needed" => Self::NotNeeded,
            "pending" => Self::Pending,
            "processing" => Self::Processing,
            "complete" => Self::Complete,
            "failed" => Self::Failed(error.unwrap_or_default()),
            _ => Self::NotNeeded,
        }
    }
}

/// The drive index manages SQLite metadata for indexed files.
pub struct DriveIndex {
    conn: std::sync::Mutex<Connection>,
}

impl DriveIndex {
    /// Open or create the index database at the given path.
    pub fn open(path: &Path) -> DriveResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| DriveError::Database(format!("Cannot create DB dir: {e}")))?;
        }
        let conn = Connection::open(path)
            .map_err(|e| DriveError::Database(format!("Cannot open DB: {e}")))?;

        // Create tables
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS drive_files (
                id TEXT PRIMARY KEY,
                drive TEXT NOT NULL,
                path TEXT NOT NULL,
                filename TEXT NOT NULL,
                mime_type TEXT NOT NULL DEFAULT '',
                size_bytes INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                modified_at TEXT NOT NULL,
                tags TEXT NOT NULL DEFAULT '[]',
                classified_by TEXT,
                ocr_status TEXT NOT NULL DEFAULT 'not_needed',
                ocr_error TEXT,
                content_status TEXT NOT NULL DEFAULT 'not_needed',
                content_error TEXT,
                embedding_status TEXT NOT NULL DEFAULT 'not_needed',
                embedding_error TEXT,
                checksum TEXT NOT NULL DEFAULT '',
                extracted_text TEXT
            );
            CREATE UNIQUE INDEX IF NOT EXISTS idx_drive_files_drive_path ON drive_files(drive, path);
            CREATE INDEX IF NOT EXISTS idx_drive_files_mime ON drive_files(mime_type);
            CREATE INDEX IF NOT EXISTS idx_drive_files_tags ON drive_files(tags);

            CREATE TABLE IF NOT EXISTS drive_repos (
                id TEXT PRIMARY KEY,
                drive TEXT NOT NULL,
                path TEXT NOT NULL,
                remote_url TEXT,
                default_branch TEXT NOT NULL DEFAULT 'main',
                registered_by TEXT,
                created_at TEXT NOT NULL
            );
            CREATE UNIQUE INDEX IF NOT EXISTS idx_drive_repos_drive_path ON drive_repos(drive, path);
            ",
        )
        .map_err(|e| DriveError::Database(format!("Cannot create tables: {e}")))?;

        Ok(Self {
            conn: std::sync::Mutex::new(conn),
        })
    }

    /// Insert or update a file entry.
    pub fn upsert_file(&self, entry: &DriveFileEntry) -> DriveResult<()> {
        let conn = self.conn.lock().map_err(|e| DriveError::Database(e.to_string()))?;
        let tags_json = serde_json::to_string(&entry.tags).unwrap_or_else(|_| "[]".to_string());
        conn.execute(
            "INSERT INTO drive_files (id, drive, path, filename, mime_type, size_bytes, created_at, modified_at, tags, classified_by, ocr_status, ocr_error, content_status, content_error, embedding_status, embedding_error, checksum, extracted_text)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
             ON CONFLICT(drive, path) DO UPDATE SET
               filename=excluded.filename, mime_type=excluded.mime_type, size_bytes=excluded.size_bytes,
               modified_at=excluded.modified_at, tags=excluded.tags, classified_by=excluded.classified_by,
               ocr_status=excluded.ocr_status, ocr_error=excluded.ocr_error,
               content_status=excluded.content_status, content_error=excluded.content_error,
               embedding_status=excluded.embedding_status, embedding_error=excluded.embedding_error,
               checksum=excluded.checksum, extracted_text=excluded.extracted_text",
            rusqlite::params![
                entry.id.to_string(),
                entry.drive,
                entry.path,
                entry.filename,
                entry.mime_type,
                entry.size_bytes,
                entry.created_at,
                entry.modified_at,
                tags_json,
                entry.classified_by,
                entry.ocr_status.as_str(),
                match &entry.ocr_status { PipelineStatus::Failed(e) => Some(e.as_str()), _ => None },
                entry.content_status.as_str(),
                match &entry.content_status { PipelineStatus::Failed(e) => Some(e.as_str()), _ => None },
                entry.embedding_status.as_str(),
                match &entry.embedding_status { PipelineStatus::Failed(e) => Some(e.as_str()), _ => None },
                entry.checksum,
                entry.extracted_text,
            ],
        )
        .map_err(|e| DriveError::Database(format!("upsert_file: {e}")))?;
        Ok(())
    }

    /// Get a file entry by drive and path.
    pub fn get_file(&self, drive: &str, path: &str) -> DriveResult<Option<DriveFileEntry>> {
        let conn = self.conn.lock().map_err(|e| DriveError::Database(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT id, drive, path, filename, mime_type, size_bytes, created_at, modified_at, tags, classified_by, ocr_status, ocr_error, content_status, content_error, embedding_status, embedding_error, checksum, extracted_text FROM drive_files WHERE drive = ?1 AND path = ?2")
            .map_err(|e| DriveError::Database(e.to_string()))?;
        let mut rows = stmt
            .query(rusqlite::params![drive, path])
            .map_err(|e| DriveError::Database(e.to_string()))?;
        if let Some(row) = rows.next().map_err(|e| DriveError::Database(e.to_string()))? {
            Ok(Some(DriveFileEntry::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    /// Remove a file entry by drive and path.
    pub fn remove_by_path(&self, drive: &str, path: &str) -> DriveResult<()> {
        let conn = self.conn.lock().map_err(|e| DriveError::Database(e.to_string()))?;
        conn.execute(
            "DELETE FROM drive_files WHERE drive = ?1 AND path = ?2",
            rusqlite::params![drive, path],
        )
        .map_err(|e| DriveError::Database(format!("remove: {e}")))?;
        Ok(())
    }

    /// Update tags on a file entry.
    pub fn set_tags(&self, drive: &str, path: &str, tags: &[String]) -> DriveResult<()> {
        let conn = self.conn.lock().map_err(|e| DriveError::Database(e.to_string()))?;
        let tags_json = serde_json::to_string(tags).unwrap_or_else(|_| "[]".to_string());
        conn.execute(
            "UPDATE drive_files SET tags = ?1 WHERE drive = ?2 AND path = ?3",
            rusqlite::params![tags_json, drive, path],
        )
        .map_err(|e| DriveError::Database(format!("set_tags: {e}")))?;
        Ok(())
    }

    /// Search files by metadata criteria.
    pub fn search_metadata(
        &self,
        drive: &str,
        query: &str,
        mime_filter: Option<&str>,
        tag_filter: Option<&str>,
        limit: usize,
    ) -> DriveResult<Vec<DriveFileEntry>> {
        let conn = self.conn.lock().map_err(|e| DriveError::Database(e.to_string()))?;
        let mut sql = String::from(
            "SELECT id, drive, path, filename, mime_type, size_bytes, created_at, modified_at, tags, classified_by, ocr_status, ocr_error, content_status, content_error, embedding_status, embedding_error, checksum, extracted_text FROM drive_files WHERE drive = ?1",
        );
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(drive.to_string())];
        let mut idx = 2;

        if !query.is_empty() {
            sql.push_str(&format!(
                " AND (filename LIKE ?{idx} OR path LIKE ?{idx} OR extracted_text LIKE ?{idx})"
            ));
            params.push(Box::new(format!("%{query}%")));
            idx += 1;
        }

        if let Some(mime) = mime_filter {
            sql.push_str(&format!(" AND mime_type = ?{idx}"));
            params.push(Box::new(mime.to_string()));
            idx += 1;
        }

        if let Some(tag) = tag_filter {
            sql.push_str(&format!(" AND tags LIKE ?{idx}"));
            params.push(Box::new(format!("%\"{tag}\"%")));
            let _ = idx;
        }

        sql.push_str(&format!(" ORDER BY modified_at DESC LIMIT {limit}"));

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| DriveError::Database(e.to_string()))?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt
            .query_map(param_refs.as_slice(), |row| {
                Ok(DriveFileEntry::from_row(row).unwrap())
            })
            .map_err(|e| DriveError::Database(e.to_string()))?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| DriveError::Database(e.to_string()))?);
        }
        Ok(results)
    }

    /// Get pipeline queue status counts.
    pub fn pipeline_status(&self) -> DriveResult<PipelineQueueStatus> {
        let conn = self.conn.lock().map_err(|e| DriveError::Database(e.to_string()))?;
        let total: u64 = conn
            .query_row("SELECT COUNT(*) FROM drive_files", [], |r| r.get(0))
            .map_err(|e| DriveError::Database(e.to_string()))?;
        let ocr_pending: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM drive_files WHERE ocr_status = 'pending'",
                [],
                |r| r.get(0),
            )
            .map_err(|e| DriveError::Database(e.to_string()))?;
        let content_pending: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM drive_files WHERE content_status = 'pending'",
                [],
                |r| r.get(0),
            )
            .map_err(|e| DriveError::Database(e.to_string()))?;
        let embedding_pending: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM drive_files WHERE embedding_status = 'pending'",
                [],
                |r| r.get(0),
            )
            .map_err(|e| DriveError::Database(e.to_string()))?;
        Ok(PipelineQueueStatus {
            total_files: total,
            ocr_pending,
            content_pending,
            embedding_pending,
        })
    }

    /// Get the most recently modified files.
    pub fn recent_files(&self, drive: &str, limit: usize) -> DriveResult<Vec<DriveFileEntry>> {
        let conn = self.conn.lock().map_err(|e| DriveError::Database(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT id, drive, path, filename, mime_type, size_bytes, created_at, modified_at, tags, classified_by, ocr_status, ocr_error, content_status, content_error, embedding_status, embedding_error, checksum, extracted_text FROM drive_files WHERE drive = ?1 ORDER BY modified_at DESC LIMIT ?2")
            .map_err(|e| DriveError::Database(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params![drive, limit as i64], |row| {
                Ok(DriveFileEntry::from_row(row).unwrap())
            })
            .map_err(|e| DriveError::Database(e.to_string()))?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| DriveError::Database(e.to_string()))?);
        }
        Ok(results)
    }

    /// Get files with pipeline errors.
    pub fn pipeline_errors(&self, drive: &str, limit: usize) -> DriveResult<Vec<DriveFileEntry>> {
        let conn = self.conn.lock().map_err(|e| DriveError::Database(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT id, drive, path, filename, mime_type, size_bytes, created_at, modified_at, tags, classified_by, ocr_status, ocr_error, content_status, content_error, embedding_status, embedding_error, checksum, extracted_text FROM drive_files WHERE drive = ?1 AND (ocr_status = 'failed' OR content_status = 'failed' OR embedding_status = 'failed') ORDER BY modified_at DESC LIMIT ?2")
            .map_err(|e| DriveError::Database(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params![drive, limit as i64], |row| {
                Ok(DriveFileEntry::from_row(row).unwrap())
            })
            .map_err(|e| DriveError::Database(e.to_string()))?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| DriveError::Database(e.to_string()))?);
        }
        Ok(results)
    }

    // -- Repo registry --

    /// Register a git repo.
    pub fn register_repo(&self, repo: &RepoEntry) -> DriveResult<()> {
        let conn = self.conn.lock().map_err(|e| DriveError::Database(e.to_string()))?;
        conn.execute(
            "INSERT INTO drive_repos (id, drive, path, remote_url, default_branch, registered_by, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(drive, path) DO UPDATE SET remote_url=excluded.remote_url, default_branch=excluded.default_branch",
            rusqlite::params![
                repo.id.to_string(),
                repo.drive,
                repo.path,
                repo.remote_url,
                repo.default_branch,
                repo.registered_by,
                repo.created_at,
            ],
        )
        .map_err(|e| DriveError::Database(format!("register_repo: {e}")))?;
        Ok(())
    }

    /// List all repos for a drive.
    pub fn list_repos(&self, drive: &str) -> DriveResult<Vec<RepoEntry>> {
        let conn = self.conn.lock().map_err(|e| DriveError::Database(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT id, drive, path, remote_url, default_branch, registered_by, created_at FROM drive_repos WHERE drive = ?1 ORDER BY path")
            .map_err(|e| DriveError::Database(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params![drive], |row| {
                Ok(RepoEntry {
                    id: uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
                    drive: row.get(1)?,
                    path: row.get(2)?,
                    remote_url: row.get(3)?,
                    default_branch: row.get(4)?,
                    registered_by: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })
            .map_err(|e| DriveError::Database(e.to_string()))?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| DriveError::Database(e.to_string()))?);
        }
        Ok(results)
    }

    /// Get a repo by ID.
    pub fn get_repo(&self, id: &str) -> DriveResult<Option<RepoEntry>> {
        let conn = self.conn.lock().map_err(|e| DriveError::Database(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT id, drive, path, remote_url, default_branch, registered_by, created_at FROM drive_repos WHERE id = ?1")
            .map_err(|e| DriveError::Database(e.to_string()))?;
        let mut rows = stmt
            .query(rusqlite::params![id])
            .map_err(|e| DriveError::Database(e.to_string()))?;
        if let Some(row) = rows.next().map_err(|e| DriveError::Database(e.to_string()))? {
            Ok(Some(RepoEntry {
                id: uuid::Uuid::parse_str(&row.get::<_, String>(0).map_err(|e| DriveError::Database(e.to_string()))?).unwrap_or_default(),
                drive: row.get(1).map_err(|e| DriveError::Database(e.to_string()))?,
                path: row.get(2).map_err(|e| DriveError::Database(e.to_string()))?,
                remote_url: row.get(3).map_err(|e| DriveError::Database(e.to_string()))?,
                default_branch: row.get(4).map_err(|e| DriveError::Database(e.to_string()))?,
                registered_by: row.get(5).map_err(|e| DriveError::Database(e.to_string()))?,
                created_at: row.get(6).map_err(|e| DriveError::Database(e.to_string()))?,
            }))
        } else {
            Ok(None)
        }
    }
}

/// Pipeline queue status summary.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PipelineQueueStatus {
    pub total_files: u64,
    pub ocr_pending: u64,
    pub content_pending: u64,
    pub embedding_pending: u64,
}

/// A registered git repository on a drive.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RepoEntry {
    pub id: uuid::Uuid,
    pub drive: String,
    pub path: String,
    pub remote_url: Option<String>,
    pub default_branch: String,
    pub registered_by: Option<String>,
    pub created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drive_index_crud() {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("test_index.db");
        let index = DriveIndex::open(&db_path).unwrap();

        let entry = DriveFileEntry {
            id: uuid::Uuid::new_v4(),
            drive: "main".to_string(),
            path: "/Documents/test.pdf".to_string(),
            filename: "test.pdf".to_string(),
            mime_type: "application/pdf".to_string(),
            size_bytes: 1024,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            modified_at: "2026-01-01T00:00:00Z".to_string(),
            tags: vec!["test".to_string()],
            classified_by: None,
            ocr_status: PipelineStatus::NotNeeded,
            content_status: PipelineStatus::Pending,
            embedding_status: PipelineStatus::NotNeeded,
            checksum: "abc123".to_string(),
            extracted_text: None,
        };

        index.upsert_file(&entry).unwrap();

        let found = index.get_file("main", "/Documents/test.pdf").unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.filename, "test.pdf");
        assert_eq!(found.tags, vec!["test"]);

        // Search
        let results = index
            .search_metadata("main", "test", None, None, 10)
            .unwrap();
        assert_eq!(results.len(), 1);

        // Tags
        index
            .set_tags("main", "/Documents/test.pdf", &["tax".to_string(), "w2".to_string()])
            .unwrap();
        let updated = index
            .get_file("main", "/Documents/test.pdf")
            .unwrap()
            .unwrap();
        assert_eq!(updated.tags, vec!["tax", "w2"]);

        // Remove
        index.remove_by_path("main", "/Documents/test.pdf").unwrap();
        assert!(index
            .get_file("main", "/Documents/test.pdf")
            .unwrap()
            .is_none());
    }
}
