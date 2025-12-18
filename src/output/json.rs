use serde::Serialize;

use crate::checker::{CheckResult, CheckStatus};
use crate::error::Result;

use super::OutputFormatter;

pub struct JsonFormatter;

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
                .fold((0, 0, 0, 0), |(p, w, f, g), r| match r.status {
                    CheckStatus::Passed => (p + 1, w, f, g),
                    CheckStatus::Warning => (p, w + 1, f, g),
                    CheckStatus::Failed => (p, w, f + 1, g),
                    CheckStatus::Grandfathered => (p, w, f, g + 1),
                });

        let output = JsonOutput {
            summary: Summary {
                total_files: results.len(),
                passed,
                warnings,
                failed,
                grandfathered,
            },
            results: results.iter().map(convert_result).collect(),
        };

        Ok(serde_json::to_string_pretty(&output)?)
    }
}

fn convert_result(result: &CheckResult) -> FileResult {
    FileResult {
        path: result.path.display().to_string(),
        status: match result.status {
            CheckStatus::Passed => "passed".to_string(),
            CheckStatus::Warning => "warning".to_string(),
            CheckStatus::Failed => "failed".to_string(),
            CheckStatus::Grandfathered => "grandfathered".to_string(),
        },
        sloc: result.stats.sloc(),
        limit: result.limit,
        stats: FileStats {
            total: result.stats.total,
            code: result.stats.code,
            comment: result.stats.comment,
            blank: result.stats.blank,
        },
        override_reason: result.override_reason.clone(),
    }
}

#[cfg(test)]
#[path = "json_tests.rs"]
mod tests;
