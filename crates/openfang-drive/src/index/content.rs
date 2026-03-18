//! Content extraction pipeline stage.
//!
//! Extracts text content from files for indexing and semantic search.
//! Runs as a background job after OCR (if needed).

use sha2::{Digest, Sha256};

/// Compute SHA-256 checksum of data.
pub fn compute_checksum(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Extract text content from file data based on MIME type.
///
/// Currently supports plain text and code files. PDF extraction and
/// office document extraction will be added later (requires external libs).
pub fn extract_text(data: &[u8], mime_type: &str) -> Option<String> {
    match mime_type {
        // Text-based files: return as-is
        m if m.starts_with("text/") => String::from_utf8(data.to_vec()).ok(),
        "application/json" | "application/toml" | "application/yaml" | "application/javascript"
        | "application/typescript" => String::from_utf8(data.to_vec()).ok(),
        // Binary formats — text extraction not yet implemented
        // "application/pdf" => extract_pdf_text(data),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checksum() {
        let sum = compute_checksum(b"hello");
        assert_eq!(sum.len(), 64); // SHA-256 hex is 64 chars
    }

    #[test]
    fn test_extract_text() {
        assert_eq!(
            extract_text(b"Hello world", "text/plain"),
            Some("Hello world".to_string())
        );
        assert!(extract_text(b"\xff\xfe", "image/png").is_none());
    }
}
