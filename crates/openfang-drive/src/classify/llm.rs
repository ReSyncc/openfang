//! LLM fallback classifier — used when no rule matches.
//!
//! Queues unclassified files for LLM-based classification as a low-priority
//! background task. The LLM examines filename + text snippet and returns
//! a suggested destination and tags.

/// Request for LLM classification (queued for background processing).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LlmClassifyRequest {
    pub drive: String,
    pub path: String,
    pub filename: String,
    pub mime_type: String,
    /// First ~500 chars of extracted text.
    pub text_snippet: Option<String>,
}

/// LLM classification result.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LlmClassifyResult {
    pub destination: String,
    pub tags: Vec<String>,
    pub confidence: f64,
}
