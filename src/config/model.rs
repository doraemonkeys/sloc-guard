use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Config {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extends: Option<String>,

    #[serde(default)]
    pub default: DefaultConfig,

    #[serde(default)]
    pub rules: std::collections::HashMap<String, RuleConfig>,

    #[serde(default)]
    pub path_rules: Vec<PathRule>,

    #[serde(default)]
    pub exclude: ExcludeConfig,

    #[serde(default, rename = "override")]
    pub overrides: Vec<FileOverride>,

    #[serde(default)]
    pub languages: std::collections::HashMap<String, CustomLanguageConfig>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CustomLanguageConfig {
    #[serde(default)]
    pub extensions: Vec<String>,

    #[serde(default)]
    pub single_line_comments: Vec<String>,

    #[serde(default)]
    pub multi_line_comments: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct DefaultConfig {
    #[serde(default = "default_max_lines")]
    pub max_lines: usize,

    #[serde(default = "default_extensions")]
    pub extensions: Vec<String>,

    #[serde(default)]
    pub include_paths: Vec<String>,

    #[serde(default = "default_true")]
    pub skip_comments: bool,

    #[serde(default = "default_true")]
    pub skip_blank: bool,

    #[serde(default = "default_warn_threshold")]
    pub warn_threshold: f64,

    #[serde(default)]
    pub strict: bool,

    #[serde(default = "default_true")]
    pub gitignore: bool,
}

impl Default for DefaultConfig {
    fn default() -> Self {
        Self {
            max_lines: default_max_lines(),
            extensions: default_extensions(),
            include_paths: Vec::new(),
            skip_comments: true,
            skip_blank: true,
            warn_threshold: default_warn_threshold(),
            strict: false,
            gitignore: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuleConfig {
    #[serde(default)]
    pub extensions: Vec<String>,

    pub max_lines: Option<usize>,

    #[serde(default)]
    pub skip_comments: Option<bool>,

    #[serde(default)]
    pub skip_blank: Option<bool>,

    #[serde(default)]
    pub warn_threshold: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExcludeConfig {
    #[serde(default)]
    pub patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileOverride {
    pub path: String,
    pub max_lines: usize,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PathRule {
    pub pattern: String,
    pub max_lines: usize,
    #[serde(default)]
    pub warn_threshold: Option<f64>,
}

const fn default_max_lines() -> usize {
    500
}

fn default_extensions() -> Vec<String> {
    vec![
        "rs".to_string(),
        "go".to_string(),
        "py".to_string(),
        "js".to_string(),
        "ts".to_string(),
        "c".to_string(),
        "cpp".to_string(),
    ]
}

const fn default_true() -> bool {
    true
}

const fn default_warn_threshold() -> f64 {
    0.9
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
