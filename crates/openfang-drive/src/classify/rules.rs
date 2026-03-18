//! Rule engine — parse, match, CRUD, and persistence for classification rules.

use chrono::Datelike;
use openfang_types::config::DriveRuleConfig;

/// A compiled classification rule.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClassificationRule {
    pub name: String,
    pub drive: String,
    pub destination: String,
    pub tags: Vec<String>,
    pub mime: Option<String>,
    pub mime_prefix: Option<String>,
    pub filename_glob: Option<String>,
    pub content_contains: Vec<String>,
}

impl ClassificationRule {
    /// Build from config.
    pub fn from_config(cfg: &DriveRuleConfig) -> Self {
        Self {
            name: cfg.name.clone(),
            drive: cfg.drive.clone(),
            destination: cfg.destination.clone(),
            tags: cfg.tags.clone(),
            mime: cfg.mime.clone(),
            mime_prefix: cfg.mime_prefix.clone(),
            filename_glob: cfg.filename_glob.clone(),
            content_contains: cfg.content_contains.clone(),
        }
    }

    /// Check if this rule matches the given file metadata and content.
    pub fn matches(&self, filename: &str, mime_type: &str, text_content: Option<&str>) -> bool {
        // MIME exact match
        if let Some(ref mime) = self.mime {
            if mime_type != mime {
                return false;
            }
        }

        // MIME prefix match
        if let Some(ref prefix) = self.mime_prefix {
            if !mime_type.starts_with(prefix) {
                return false;
            }
        }

        // Filename glob
        if let Some(ref glob) = self.filename_glob {
            if !simple_glob(glob, filename) {
                return false;
            }
        }

        // Content contains (all must match, case-insensitive)
        if !self.content_contains.is_empty() {
            let text = match text_content {
                Some(t) => t.to_lowercase(),
                None => return false, // Can't match content without text
            };
            for needle in &self.content_contains {
                if !text.contains(&needle.to_lowercase()) {
                    return false;
                }
            }
        }

        true
    }

    /// Resolve template variables in the destination path.
    pub fn resolve_destination(&self, file_date: Option<&chrono::NaiveDate>) -> String {
        let now = chrono::Utc::now().naive_utc().date();
        let date = file_date.unwrap_or(&now);

        self.destination
            .replace("{year}", &date.year().to_string())
            .replace("{month}", &format!("{:02}", date.month()))
            .replace("{day}", &format!("{:02}", date.day()))
    }
}

/// Simple glob pattern matching (supports * as wildcard).
fn simple_glob(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if pattern == value {
        return true;
    }
    if let Some(suffix) = pattern.strip_prefix('*') {
        return value.ends_with(suffix);
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return value.starts_with(prefix);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use openfang_types::config::DriveRuleConfig;

    #[test]
    fn test_rule_matches_mime_and_content() {
        let cfg = DriveRuleConfig {
            name: "tax-w2".to_string(),
            drive: "main".to_string(),
            destination: "/Documents/Tax/{year}/W2s/".to_string(),
            tags: vec!["tax".to_string(), "w2".to_string()],
            mime: Some("application/pdf".to_string()),
            mime_prefix: None,
            filename_glob: None,
            content_contains: vec!["W-2".to_string(), "Wage and Tax".to_string()],
        };
        let rule = ClassificationRule::from_config(&cfg);

        // Match
        assert!(rule.matches(
            "w2_2025.pdf",
            "application/pdf",
            Some("This is a W-2 Wage and Tax Statement for 2025")
        ));

        // Wrong MIME
        assert!(!rule.matches(
            "w2.pdf",
            "image/png",
            Some("W-2 Wage and Tax")
        ));

        // Missing content keyword
        assert!(!rule.matches(
            "w2.pdf",
            "application/pdf",
            Some("W-2 but no wage info")
        ));

        // No text content
        assert!(!rule.matches("w2.pdf", "application/pdf", None));
    }

    #[test]
    fn test_rule_matches_mime_prefix() {
        let cfg = DriveRuleConfig {
            name: "photos".to_string(),
            drive: "main".to_string(),
            destination: "/Photos/{year}/{month}/".to_string(),
            tags: vec!["photo".to_string()],
            mime: None,
            mime_prefix: Some("image/".to_string()),
            filename_glob: None,
            content_contains: vec![],
        };
        let rule = ClassificationRule::from_config(&cfg);

        assert!(rule.matches("photo.jpg", "image/jpeg", None));
        assert!(rule.matches("shot.png", "image/png", None));
        assert!(!rule.matches("doc.pdf", "application/pdf", None));
    }

    #[test]
    fn test_destination_template() {
        let rule = ClassificationRule {
            name: "test".to_string(),
            drive: "main".to_string(),
            destination: "/Tax/{year}/{month}/".to_string(),
            tags: vec![],
            mime: None,
            mime_prefix: None,
            filename_glob: None,
            content_contains: vec![],
        };
        let date = chrono::NaiveDate::from_ymd_opt(2025, 3, 15).unwrap();
        assert_eq!(rule.resolve_destination(Some(&date)), "/Tax/2025/03/");
    }
}
