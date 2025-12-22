use serde::Serialize;

use crate::analyzer::SplitSuggestion;
use crate::checker::{CheckResult, ViolationCategory};
use crate::error::Result;

use super::OutputFormatter;

pub struct JsonFormatter {
    show_suggestions: bool,
}

impl JsonFormatter {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            show_suggestions: false,
        }
    }

    #[must_use]
    pub const fn with_suggestions(mut self, show: bool) -> Self {
        self.show_suggestions = show;
        self
    }
}

impl Default for JsonFormatter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize)]
struct JsonOutput {
    summary: Summary,
    results: Vec<FileResult>,
}

#[derive(Serialize)]
struct Summary {
    total_files: usize,
    passed: usize,
    warnings: usize,
    failed: usize,
    grandfathered: usize,
}

#[derive(Serialize)]
struct FileResult {
    path: String,
    status: String,
    sloc: usize,
    limit: usize,
    stats: FileStats,
    #[serde(skip_serializing_if = "Option::is_none")]
    override_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    violation_category: Option<ViolationCategory>,
    #[serde(skip_serializing_if = "Option::is_none")]
    suggestions: Option<SplitSuggestion>,
}

#[derive(Serialize)]
struct FileStats {
    total: usize,
    code: usize,
    comment: usize,
    blank: usize,
}

impl OutputFormatter for JsonFormatter {
    fn format(&self, results: &[CheckResult]) -> Result<String> {
        let (passed, warnings, failed, grandfathered) =
            results
                .iter()
                .fold((0, 0, 0, 0), |(p, w, f, g), r| match r {
                    CheckResult::Passed { .. } => (p + 1, w, f, g),
                    CheckResult::Warning { .. } => (p, w + 1, f, g),
                    CheckResult::Failed { .. } => (p, w, f + 1, g),
                    CheckResult::Grandfathered { .. } => (p, w, f, g + 1),
                });

        let output = JsonOutput {
            summary: Summary {
                total_files: results.len(),
                passed,
                warnings,
                failed,
                grandfathered,
            },
            results: results
                .iter()
                .map(|r| convert_result(r, self.show_suggestions))
                .collect(),
        };

        Ok(serde_json::to_string_pretty(&output)?)
    }
}

fn convert_result(result: &CheckResult, show_suggestions: bool) -> FileResult {
    let suggestions = if show_suggestions {
        result.suggestions().cloned()
    } else {
        None
    };

    FileResult {
        path: result.path().display().to_string(),
        status: match result {
            CheckResult::Passed { .. } => "passed".to_string(),
            CheckResult::Warning { .. } => "warning".to_string(),
            CheckResult::Failed { .. } => "failed".to_string(),
            CheckResult::Grandfathered { .. } => "grandfathered".to_string(),
        },
        sloc: result.stats().sloc(),
        limit: result.limit(),
        stats: FileStats {
            total: result.stats().total,
            code: result.stats().code,
            comment: result.stats().comment,
            blank: result.stats().blank,
        },
        override_reason: result.override_reason().map(String::from),
        violation_category: result.violation_category().cloned(),
        suggestions,
    }
}

#[cfg(test)]
#[path = "json_tests.rs"]
mod tests;
