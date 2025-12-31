use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::analyzer::SplitSuggestion;
use crate::checker::{CheckResult, ViolationCategory, ViolationType};
use crate::error::Result;

use super::OutputFormatter;
use super::path::display_path;

/// SARIF 2.1.0 output formatter for GitHub Code Scanning and other CI/CD tools.
pub struct SarifFormatter {
    show_suggestions: bool,
    project_root: Option<PathBuf>,
}

impl SarifFormatter {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            show_suggestions: false,
            project_root: None,
        }
    }

    #[must_use]
    pub const fn with_suggestions(mut self, show: bool) -> Self {
        self.show_suggestions = show;
        self
    }

    #[must_use]
    pub fn with_project_root(mut self, root: Option<PathBuf>) -> Self {
        self.project_root = root;
        self
    }

    fn display_path(&self, path: &Path) -> String {
        display_path(path, self.project_root.as_deref())
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

// Content (SLOC) rule IDs
const RULE_LINE_LIMIT_EXCEEDED: &str = "sloc-guard/line-limit-exceeded";
const RULE_LINE_LIMIT_WARNING: &str = "sloc-guard/line-limit-warning";

// Structure rule IDs
const RULE_STRUCTURE_FILE_COUNT: &str = "sloc-guard/structure-file-count";
const RULE_STRUCTURE_DIR_COUNT: &str = "sloc-guard/structure-dir-count";
const RULE_STRUCTURE_MAX_DEPTH: &str = "sloc-guard/structure-max-depth";
const RULE_STRUCTURE_DISALLOWED_FILE: &str = "sloc-guard/structure-disallowed-file";
const RULE_STRUCTURE_DISALLOWED_DIR: &str = "sloc-guard/structure-disallowed-dir";
const RULE_STRUCTURE_DENIED: &str = "sloc-guard/structure-denied";
const RULE_STRUCTURE_NAMING: &str = "sloc-guard/structure-naming";
const RULE_STRUCTURE_SIBLING: &str = "sloc-guard/structure-sibling";

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
            // Content (SLOC) rules - indices 0-1
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
            // Structure rules - indices 2-9
            ReportingDescriptor {
                id: RULE_STRUCTURE_FILE_COUNT,
                name: "StructureFileCount",
                short_description: MultiformatMessageString {
                    text: "Directory exceeds file count limit",
                },
                full_description: MultiformatMessageString {
                    text: "The number of files in this directory exceeds the configured maximum limit.",
                },
                default_configuration: ReportingConfiguration { level: "error" },
            },
            ReportingDescriptor {
                id: RULE_STRUCTURE_DIR_COUNT,
                name: "StructureDirCount",
                short_description: MultiformatMessageString {
                    text: "Directory exceeds subdirectory count limit",
                },
                full_description: MultiformatMessageString {
                    text: "The number of subdirectories in this directory exceeds the configured maximum limit.",
                },
                default_configuration: ReportingConfiguration { level: "error" },
            },
            ReportingDescriptor {
                id: RULE_STRUCTURE_MAX_DEPTH,
                name: "StructureMaxDepth",
                short_description: MultiformatMessageString {
                    text: "Directory exceeds maximum depth",
                },
                full_description: MultiformatMessageString {
                    text: "The directory nesting depth exceeds the configured maximum limit.",
                },
                default_configuration: ReportingConfiguration { level: "error" },
            },
            ReportingDescriptor {
                id: RULE_STRUCTURE_DISALLOWED_FILE,
                name: "StructureDisallowedFile",
                short_description: MultiformatMessageString {
                    text: "File type not allowed",
                },
                full_description: MultiformatMessageString {
                    text: "This file type is not in the allowlist for this directory.",
                },
                default_configuration: ReportingConfiguration { level: "error" },
            },
            ReportingDescriptor {
                id: RULE_STRUCTURE_DISALLOWED_DIR,
                name: "StructureDisallowedDir",
                short_description: MultiformatMessageString {
                    text: "Directory not allowed",
                },
                full_description: MultiformatMessageString {
                    text: "This directory is not in the allowlist.",
                },
                default_configuration: ReportingConfiguration { level: "error" },
            },
            ReportingDescriptor {
                id: RULE_STRUCTURE_DENIED,
                name: "StructureDenied",
                short_description: MultiformatMessageString {
                    text: "File or directory denied",
                },
                full_description: MultiformatMessageString {
                    text: "This file or directory matches a deny pattern and is not allowed.",
                },
                default_configuration: ReportingConfiguration { level: "error" },
            },
            ReportingDescriptor {
                id: RULE_STRUCTURE_NAMING,
                name: "StructureNaming",
                short_description: MultiformatMessageString {
                    text: "File naming convention violated",
                },
                full_description: MultiformatMessageString {
                    text: "The file name does not match the required naming pattern for this directory.",
                },
                default_configuration: ReportingConfiguration { level: "error" },
            },
            ReportingDescriptor {
                id: RULE_STRUCTURE_SIBLING,
                name: "StructureSibling",
                short_description: MultiformatMessageString {
                    text: "Required sibling file missing",
                },
                full_description: MultiformatMessageString {
                    text: "A required sibling file is missing for this file.",
                },
                default_configuration: ReportingConfiguration { level: "error" },
            },
        ]
    }

    /// Get rule ID and index based on violation category and result type.
    fn get_rule_info(result: &CheckResult) -> (&'static str, usize, &'static str) {
        let is_warning = result.is_warning();
        let is_grandfathered = result.is_grandfathered();

        match result.violation_category() {
            Some(ViolationCategory::Structure { violation_type, .. }) => {
                let (rule_id, rule_index) = match violation_type {
                    ViolationType::FileCount => (RULE_STRUCTURE_FILE_COUNT, 2),
                    ViolationType::DirCount => (RULE_STRUCTURE_DIR_COUNT, 3),
                    ViolationType::MaxDepth => (RULE_STRUCTURE_MAX_DEPTH, 4),
                    ViolationType::DisallowedFile => (RULE_STRUCTURE_DISALLOWED_FILE, 5),
                    ViolationType::DisallowedDirectory => (RULE_STRUCTURE_DISALLOWED_DIR, 6),
                    ViolationType::DeniedFile { .. } | ViolationType::DeniedDirectory { .. } => {
                        (RULE_STRUCTURE_DENIED, 7)
                    }
                    ViolationType::NamingConvention { .. } => (RULE_STRUCTURE_NAMING, 8),
                    ViolationType::MissingSibling { .. }
                    | ViolationType::GroupIncomplete { .. } => (RULE_STRUCTURE_SIBLING, 9),
                };
                let level = if is_grandfathered {
                    "note"
                } else if is_warning {
                    "warning"
                } else {
                    "error"
                };
                (rule_id, rule_index, level)
            }
            Some(ViolationCategory::Content) | None => {
                // Content (SLOC) violation
                if is_grandfathered {
                    (RULE_LINE_LIMIT_EXCEEDED, 0, "note")
                } else if is_warning {
                    (RULE_LINE_LIMIT_WARNING, 1, "warning")
                } else {
                    (RULE_LINE_LIMIT_EXCEEDED, 0, "error")
                }
            }
        }
    }

    /// Generate message text based on violation category.
    fn get_message_text(result: &CheckResult) -> String {
        match result.violation_category() {
            Some(ViolationCategory::Structure { violation_type, .. }) => {
                Self::format_structure_message(result, violation_type)
            }
            Some(ViolationCategory::Content) | None => Self::format_content_message(result),
        }
    }

    fn format_content_message(result: &CheckResult) -> String {
        let sloc = result.stats().sloc();
        let limit = result.limit();

        if result.is_grandfathered() {
            format!("File has {sloc} SLOC, exceeding limit of {limit} (grandfathered)")
        } else if result.is_warning() {
            format!(
                "File has {sloc} SLOC ({:.1}% of {limit} limit)",
                result.usage_percent()
            )
        } else {
            format!(
                "File has {sloc} SLOC, exceeding limit of {limit} by {} lines",
                sloc.saturating_sub(limit)
            )
        }
    }

    fn format_structure_message(result: &CheckResult, violation_type: &ViolationType) -> String {
        let actual = result.stats().sloc();
        let limit = result.limit();
        let grandfathered_suffix = if result.is_grandfathered() {
            " (grandfathered)"
        } else {
            ""
        };

        match violation_type {
            ViolationType::FileCount => {
                format!(
                    "Directory has {actual} files, exceeding limit of {limit}{grandfathered_suffix}"
                )
            }
            ViolationType::DirCount => {
                format!(
                    "Directory has {actual} subdirectories, exceeding limit of {limit}{grandfathered_suffix}"
                )
            }
            ViolationType::MaxDepth => {
                format!(
                    "Directory depth is {actual}, exceeding limit of {limit}{grandfathered_suffix}"
                )
            }
            ViolationType::DisallowedFile => {
                format!("File type not allowed in this directory{grandfathered_suffix}")
            }
            ViolationType::DisallowedDirectory => {
                format!("Directory not allowed{grandfathered_suffix}")
            }
            ViolationType::DeniedFile {
                pattern_or_extension,
            } => {
                format!("File matches deny pattern '{pattern_or_extension}'{grandfathered_suffix}")
            }
            ViolationType::DeniedDirectory { pattern } => {
                format!("Directory matches deny pattern '{pattern}'{grandfathered_suffix}")
            }
            ViolationType::NamingConvention { expected_pattern } => {
                format!(
                    "File name does not match required pattern '{expected_pattern}'{grandfathered_suffix}"
                )
            }
            ViolationType::MissingSibling {
                expected_sibling_pattern,
            } => {
                format!(
                    "Missing required sibling file matching '{expected_sibling_pattern}'{grandfathered_suffix}"
                )
            }
            ViolationType::GroupIncomplete {
                missing_patterns, ..
            } => {
                let missing = missing_patterns.join(", ");
                format!("Incomplete file group, missing: {missing}{grandfathered_suffix}")
            }
        }
    }

    fn convert_result(&self, result: &CheckResult) -> Option<SarifResult> {
        if result.is_passed() {
            return None;
        }

        let (rule_id, rule_index, level) = Self::get_rule_info(result);
        let message_text = Self::get_message_text(result);

        let suppressions = if result.is_grandfathered() {
            Some(vec![Suppression {
                kind: "external",
                justification: "File is in baseline (grandfathered)",
            }])
        } else {
            None
        };

        // Convert path to URI format (already uses forward slashes from display_path)
        let uri = self.display_path(result.path());

        let suggestions = if self.show_suggestions {
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
                // Use raw_stats for display (before skip_comments/skip_blank adjustments)
                stats: {
                    let raw = result.raw_stats();
                    StatsProperties {
                        total: raw.total,
                        code: raw.code,
                        comment: raw.comment,
                        blank: raw.blank,
                    }
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
            .filter_map(|r| self.convert_result(r))
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
