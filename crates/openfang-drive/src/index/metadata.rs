//! DriveFileEntry — metadata record for an indexed file.

use rusqlite::Row;

use crate::{DriveError, DriveResult};

use super::PipelineStatus;

/// Metadata index entry for a file on a drive.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DriveFileEntry {
    pub id: uuid::Uuid,
    pub drive: String,
    pub path: String,
    pub filename: String,
    pub mime_type: String,
    pub size_bytes: u64,
    pub created_at: String,
    pub modified_at: String,
    pub tags: Vec<String>,
    pub classified_by: Option<String>,
    pub ocr_status: PipelineStatus,
    pub content_status: PipelineStatus,
    pub embedding_status: PipelineStatus,
    pub checksum: String,
    pub extracted_text: Option<String>,
}

impl DriveFileEntry {
    /// Parse a DriveFileEntry from a SQLite row.
    /// Column order: id, drive, path, filename, mime_type, size_bytes, created_at,
    /// modified_at, tags, classified_by, ocr_status, ocr_error, content_status,
    /// content_error, embedding_status, embedding_error, checksum, extracted_text
    pub fn from_row(row: &Row<'_>) -> DriveResult<Self> {
        let tags_json: String = row
            .get(8)
            .map_err(|e| DriveError::Database(e.to_string()))?;
        let tags: Vec<String> =
            serde_json::from_str(&tags_json).unwrap_or_default();
        let ocr_status_str: String = row
            .get(10)
            .map_err(|e| DriveError::Database(e.to_string()))?;
        let ocr_error: Option<String> = row
            .get(11)
            .map_err(|e| DriveError::Database(e.to_string()))?;
        let content_status_str: String = row
            .get(12)
            .map_err(|e| DriveError::Database(e.to_string()))?;
        let content_error: Option<String> = row
            .get(13)
            .map_err(|e| DriveError::Database(e.to_string()))?;
        let embedding_status_str: String = row
            .get(14)
            .map_err(|e| DriveError::Database(e.to_string()))?;
        let embedding_error: Option<String> = row
            .get(15)
            .map_err(|e| DriveError::Database(e.to_string()))?;

        Ok(Self {
            id: uuid::Uuid::parse_str(
                &row.get::<_, String>(0)
                    .map_err(|e| DriveError::Database(e.to_string()))?,
            )
            .unwrap_or_default(),
            drive: row
                .get(1)
                .map_err(|e| DriveError::Database(e.to_string()))?,
            path: row
                .get(2)
                .map_err(|e| DriveError::Database(e.to_string()))?,
            filename: row
                .get(3)
                .map_err(|e| DriveError::Database(e.to_string()))?,
            mime_type: row
                .get(4)
                .map_err(|e| DriveError::Database(e.to_string()))?,
            size_bytes: row
                .get::<_, i64>(5)
                .map_err(|e| DriveError::Database(e.to_string()))?
                as u64,
            created_at: row
                .get(6)
                .map_err(|e| DriveError::Database(e.to_string()))?,
            modified_at: row
                .get(7)
                .map_err(|e| DriveError::Database(e.to_string()))?,
            tags,
            classified_by: row
                .get(9)
                .map_err(|e| DriveError::Database(e.to_string()))?,
            ocr_status: PipelineStatus::from_db(&ocr_status_str, ocr_error),
            content_status: PipelineStatus::from_db(&content_status_str, content_error),
            embedding_status: PipelineStatus::from_db(&embedding_status_str, embedding_error),
            checksum: row
                .get(16)
                .map_err(|e| DriveError::Database(e.to_string()))?,
            extracted_text: row
                .get(17)
                .map_err(|e| DriveError::Database(e.to_string()))?,
        })
    }
}
