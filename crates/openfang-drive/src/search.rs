//! Unified search — combines metadata search with semantic search results.

use crate::index::{DriveFileEntry, DriveIndex};
use crate::DriveResult;

/// Search result combining metadata and semantic matches.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchResult {
    /// File metadata entry.
    pub entry: DriveFileEntry,
    /// Match type: "metadata" or "semantic".
    pub match_type: String,
    /// Relevance score (0.0 - 1.0). Higher is better.
    pub score: f64,
}

/// Perform a unified search across metadata.
///
/// Semantic search integration will be added when the embedding pipeline
/// is connected to the memory crate's SemanticStore.
pub fn search(
    index: &DriveIndex,
    drive: &str,
    query: &str,
    search_type: &str,
    mime_filter: Option<&str>,
    tag_filter: Option<&str>,
    limit: usize,
) -> DriveResult<Vec<SearchResult>> {
    match search_type {
        "metadata" | "" => {
            let entries = index.search_metadata(drive, query, mime_filter, tag_filter, limit)?;
            Ok(entries
                .into_iter()
                .map(|e| SearchResult {
                    entry: e,
                    match_type: "metadata".to_string(),
                    score: 1.0,
                })
                .collect())
        }
        "semantic" => {
            // Semantic search — placeholder for when embedding pipeline is connected.
            // For now, fall back to metadata text search on extracted_text field.
            let entries = index.search_metadata(drive, query, mime_filter, tag_filter, limit)?;
            Ok(entries
                .into_iter()
                .map(|e| SearchResult {
                    entry: e,
                    match_type: "semantic".to_string(),
                    score: 0.5,
                })
                .collect())
        }
        other => {
            // Default to metadata
            let entries = index.search_metadata(drive, query, mime_filter, tag_filter, limit)?;
            Ok(entries
                .into_iter()
                .map(|e| SearchResult {
                    entry: e,
                    match_type: other.to_string(),
                    score: 1.0,
                })
                .collect())
        }
    }
}
