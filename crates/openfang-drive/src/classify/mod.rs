//! Classification pipeline — rule-based and LLM-fallback file organization.

pub mod llm;
pub mod rules;

use openfang_types::config::DriveRuleConfig;

use self::rules::ClassificationRule;

/// Orchestrates classification: rules first, then LLM fallback.
pub struct ClassificationPipeline {
    rules: Vec<ClassificationRule>,
}

impl ClassificationPipeline {
    /// Create a new pipeline from config rules.
    pub fn new(rule_configs: &[DriveRuleConfig]) -> Self {
        let rules = rule_configs
            .iter()
            .map(ClassificationRule::from_config)
            .collect();
        Self { rules }
    }

    /// Classify a file. Returns (destination_path, tags, classified_by) if matched.
    pub fn classify(
        &self,
        filename: &str,
        mime_type: &str,
        text_content: Option<&str>,
        file_date: Option<&chrono::NaiveDate>,
    ) -> Option<ClassificationResult> {
        for rule in &self.rules {
            if rule.matches(filename, mime_type, text_content) {
                let dest = rule.resolve_destination(file_date);
                return Some(ClassificationResult {
                    destination: dest,
                    tags: rule.tags.clone(),
                    classified_by: format!("rule:{}", rule.name),
                });
            }
        }
        None
    }

    /// Get all rules.
    pub fn rules(&self) -> &[ClassificationRule] {
        &self.rules
    }

    /// Add a new rule at runtime.
    pub fn add_rule(&mut self, rule: ClassificationRule) {
        self.rules.push(rule);
    }
}

/// Result of classifying a file.
#[derive(Debug, Clone)]
pub struct ClassificationResult {
    pub destination: String,
    pub tags: Vec<String>,
    pub classified_by: String,
}
