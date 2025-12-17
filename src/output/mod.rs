mod json;
mod text;

pub use json::JsonFormatter;
pub use text::TextFormatter;

use crate::checker::CheckResult;
use crate::error::Result;

/// Trait for formatting check results into various output formats.
pub trait OutputFormatter {
    /// Format the check results into a string.
    ///
    /// # Errors
    /// Returns an error if the formatting fails.
    fn format(&self, results: &[CheckResult]) -> Result<String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
    Sarif,
    Markdown,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            "sarif" => Ok(Self::Sarif),
            "markdown" | "md" => Ok(Self::Markdown),
            _ => Err(format!("Unknown output format: {s}")),
        }
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
