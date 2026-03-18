//! OCR pipeline stage.
//!
//! Detects if a PDF/image needs OCR (no extractable text layer) and
//! produces text output. Uses Tesseract (local) or a configurable cloud
//! OCR provider.
//!
//! This module defines the pipeline interface. Actual OCR engine integration
//! will be added when Tesseract bindings or cloud provider SDKs are wired.

/// OCR processing request.
#[derive(Debug, Clone)]
pub struct OcrRequest {
    pub drive: String,
    pub path: String,
    pub mime_type: String,
    pub data: Vec<u8>,
}

/// OCR processing result.
#[derive(Debug, Clone)]
pub struct OcrResult {
    pub text: String,
    pub confidence: f64,
}

/// Check whether a file likely needs OCR.
pub fn needs_ocr(mime_type: &str, has_text: bool) -> bool {
    if has_text {
        return false;
    }
    matches!(
        mime_type,
        "application/pdf" | "image/png" | "image/jpeg" | "image/tiff" | "image/bmp"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_needs_ocr() {
        assert!(needs_ocr("application/pdf", false));
        assert!(!needs_ocr("application/pdf", true));
        assert!(needs_ocr("image/png", false));
        assert!(!needs_ocr("text/plain", false));
    }
}
