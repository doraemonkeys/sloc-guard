use serde::Serialize;

use crate::analyzer::SplitSuggestion;
use crate::checker::CheckResult;
use crate::error::Result;

use super::OutputFormatter;

/// SARIF 2.1.0 output formatter for GitHub Code Scanning and other CI/CD tools.
pub struct SarifFormatter {
    show_suggestions: bool,
}

impl SarifFormatter {
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

impl Default for SarifFormatter {
    fn default() -> Self {
        Self::new()
    }
}

const SARIF_SCHEMA: &str = "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json";
const SARIF_VERSION: &str = "2.1.0";
const TOOL_NAME: &str = "sloc-guard";
const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");
const TOOL_INFO_URI: &str = "https://github.com/doraemonkeys/sloc-guard";

const RULE_LINE_LIMIT_EXCEEDED: &str = "sloc-guard/line-limit-exceeded";
const RULE_LINE_LIMIT_WARNING: &str = "sloc-guard/line-limit-warning";

#[derive(Serialize)]
struct SarifLog {
    #[serde(rename = "$schema")]
    schema: &'static str,
    version: &'static str,
    runs: Vec<Run>,
}

#[derive(Serialize)]
struct Run {
    tool: Tool,
    results: Vec<SarifResult>,
}

#[derive(Serialize)]
struct Tool {
    driver: ToolDriver,
}

#[derive(Serialize)]
struct ToolDriver {
    name: &'static str,
    version: &'static str,
    #[serde(rename = "informationUri")]
    information_uri: &'static str,
    rules: Vec<ReportingDescriptor>,
}

#[derive(Serialize)]
struct ReportingDescriptor {
    id: &'static str,
    name: &'static str,
    #[serde(rename = "shortDescription")]
    short_description: MultiformatMessageString,
    #[serde(rename = "fullDescription")]
    full_description: MultiformatMessageString,
    #[serde(rename = "defaultConfiguration")]
    default_configuration: ReportingConfiguration,
}

#[derive(Serialize)]
struct ReportingConfiguration {
    level: &'static str,
}

#[derive(Serialize)]
struct MultiformatMessageString {
    text: &'static str,
}

#[derive(Serialize)]
struct SarifResult {
    #[serde(rename = "ruleId")]
    rule_id: &'static str,
    #[serde(rename = "ruleIndex")]
    rule_index: usize,
    level: &'static str,
    message: Message,
    locations: Vec<Location>,
    #[serde(skip_serializing_if = "Option::is_none")]
    suppressions: Option<Vec<Suppression>>,
    properties: ResultProperties,
}

#[derive(Serialize)]
struct Message {
    text: String,
}

#[derive(Serialize)]
struct Location {
    #[serde(rename = "physicalLocation")]
    physical_location: PhysicalLocation,
}

#[derive(Serialize)]
struct PhysicalLocation {
    #[serde(rename = "artifactLocation")]
    artifact_location: ArtifactLocation,
}

#[derive(Serialize)]
struct ArtifactLocation {
    uri: String,
    #[serde(rename = "uriBaseId")]
    uri_base_id: &'static str,
}

#[derive(Serialize)]
struct Suppression {
    kind: &'static str,
    justification: &'static str,
}

#[derive(Serialize)]
struct ResultProperties {
    sloc: usize,
    limit: usize,
    #[serde(rename = "usagePercent")]
    usage_percent: f64,
    stats: StatsProperties,
    #[serde(rename = "overrideReason", skip_serializing_if = "Option::is_none")]
    override_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    suggestions: Option<SplitSuggestion>,
}

#[derive(Serialize)]
struct StatsProperties {
    total: usize,
    code: usize,
    comment: usize,
    blank: usize,
}

impl SarifFormatter {
    fn build_rules() -> Vec<ReportingDescriptor> {
        vec![
            ReportingDescriptor {
                id: RULE_LINE_LIMIT_EXCEEDED,
                name: "LineLimitExceeded",
                short_description: MultiformatMessageString {
                    text: "File exceeds SLOC limit",
                },
                full_description: MultiformatMessageString {
                    text: "The source lines of code (SLOC) in this file exceeds the configured maximum limit.",
                },
                default_configuration: ReportingConfiguration { level: "error" },
            },
            ReportingDescriptor {
                id: RULE_LINE_LIMIT_WARNING,
                name: "LineLimitWarning",
                short_description: MultiformatMessageString {
                    text: "File approaching SLOC limit",
                },
                full_description: MultiformatMessageString {
                    text: "The source lines of code (SLOC) in this file is approaching the configured maximum limit.",
                },
                default_configuration: ReportingConfiguration { level: "warning" },
            },
        ]
    }

    fn convert_result(result: &CheckResult, show_suggestions: bool) -> Option<SarifResult> {
        if result.is_passed() {
            return None;
        }

        let (rule_id, rule_index, level) = match result {
            CheckResult::Failed { .. } => (RULE_LINE_LIMIT_EXCEEDED, 0, "error"),
            CheckResult::Warning { .. } => (RULE_LINE_LIMIT_WARNING, 1, "warning"),
            CheckResult::Grandfathered { .. } => (RULE_LINE_LIMIT_EXCEEDED, 0, "note"),
            CheckResult::Passed { .. } => unreachable!(),
        };

        let suppressions = if result.is_grandfathered() {
            Some(vec![Suppression {
                kind: "external",
                justification: "File is in baseline (grandfathered)",
            }])
        } else {
            None
        };

        let message_text = match result {
            CheckResult::Failed { .. } => format!(
                "File has {} SLOC, exceeding limit of {} by {} lines",
                result.stats().sloc(),
                result.limit(),
                result.stats().sloc() - result.limit()
            ),
            CheckResult::Warning { .. } => format!(
                "File has {} SLOC ({:.1}% of {} limit)",
                result.stats().sloc(),
                result.usage_percent(),
                result.limit()
            ),
            CheckResult::Grandfathered { .. } => format!(
                "File has {} SLOC, exceeding limit of {} (grandfathered)",
                result.stats().sloc(),
                result.limit()
            ),
            CheckResult::Passed { .. } => unreachable!(),
        };

        // Convert path to URI format (forward slashes)
        let uri = result.path().display().to_string().replace('\\', "/");

        let suggestions = if show_suggestions {
            result.suggestions().cloned()
        } else {
            None
        };

        Some(SarifResult {
            rule_id,
            rule_index,
            level,
            message: Message { text: message_text },
            locations: vec![Location {
                physical_location: PhysicalLocation {
                    artifact_location: ArtifactLocation {
                        uri,
                        uri_base_id: "%SRCROOT%",
                    },
                },
            }],
            suppressions,
            properties: ResultProperties {
                sloc: result.stats().sloc(),
                limit: result.limit(),
                usage_percent: result.usage_percent(),
                stats: StatsProperties {
                    total: result.stats().total,
                    code: result.stats().code,
                    comment: result.stats().comment,
                    blank: result.stats().blank,
                },
                override_reason: result.override_reason().map(String::from),
                suggestions,
            },
        })
    }
}

impl OutputFormatter for SarifFormatter {
    fn format(&self, results: &[CheckResult]) -> Result<String> {
        let sarif_results: Vec<SarifResult> = results
            .iter()
            .filter_map(|r| Self::convert_result(r, self.show_suggestions))
            .collect();

        let log = SarifLog {
            schema: SARIF_SCHEMA,
            version: SARIF_VERSION,
            runs: vec![Run {
                tool: Tool {
                    driver: ToolDriver {
                        name: TOOL_NAME,
                        version: TOOL_VERSION,
                        information_uri: TOOL_INFO_URI,
                        rules: Self::build_rules(),
                    },
                },
                results: sarif_results,
            }],
        };

        Ok(serde_json::to_string_pretty(&log)?)
    }
}

#[cfg(test)]
#[path = "sarif_tests.rs"]
mod tests;
