use std::path::PathBuf;

use crate::checker::{CheckResult, StructureViolation, ViolationCategory, ViolationType};
use crate::counter::LineStats;
use crate::output::{
    HtmlFormatter, JsonFormatter, MarkdownFormatter, OutputFormat, OutputFormatter,
    ProjectStatistics, SarifFormatter, TextFormatter,
};

pub fn format_output(
    format: OutputFormat,
    results: &[CheckResult],
    color_mode: crate::output::ColorMode,
    verbose: u8,
    show_suggestions: bool,
    project_stats: Option<ProjectStatistics>,
    project_root: Option<PathBuf>,
) -> crate::Result<String> {
    match format {
        OutputFormat::Text => TextFormatter::with_verbose(color_mode, verbose)
            .with_suggestions(show_suggestions)
            .with_project_root(project_root)
            .format(results),
        OutputFormat::Json => JsonFormatter::new()
            .with_suggestions(show_suggestions)
            .with_project_root(project_root)
            .format(results),
        OutputFormat::Sarif => SarifFormatter::new()
            .with_suggestions(show_suggestions)
            .with_project_root(project_root)
            .format(results),
        OutputFormat::Markdown => MarkdownFormatter::new()
            .with_suggestions(show_suggestions)
            .with_project_root(project_root)
            .format(results),
        OutputFormat::Html => {
            let mut formatter = HtmlFormatter::new()
                .with_suggestions(show_suggestions)
                .with_project_root(project_root);
            if let Some(stats) = project_stats {
                formatter = formatter.with_stats(stats);
            }
            formatter.format(results)
        }
    }
}

/// Convert a structure violation to a check result for unified output.
pub fn structure_violation_to_check_result(violation: &StructureViolation) -> CheckResult {
    // Create synthetic LineStats representing the violation
    // We use 'code' to represent the actual count for display purposes
    let stats = LineStats {
        total: violation.actual,
        code: violation.actual,
        comment: 0,
        blank: 0,
        ignored: 0,
    };

    // Build the violation category with structured type information
    let violation_category = Some(ViolationCategory::Structure {
        violation_type: violation.violation_type.clone(),
        triggering_rule: violation.triggering_rule_pattern.clone(),
    });

    // Build human-readable description for override_reason (for backwards compatibility)
    let override_reason = match &violation.violation_type {
        ViolationType::FileCount => Some("structure: files count exceeded".to_string()),
        ViolationType::DirCount => Some("structure: subdirs count exceeded".to_string()),
        ViolationType::MaxDepth => Some("structure: depth count exceeded".to_string()),
        ViolationType::DisallowedFile => {
            let rule = violation
                .triggering_rule_pattern
                .as_deref()
                .unwrap_or("unknown");
            Some(format!("structure: disallowed file (rule: {rule})"))
        }
        ViolationType::DisallowedDirectory => {
            let rule = violation
                .triggering_rule_pattern
                .as_deref()
                .unwrap_or("unknown");
            Some(format!("structure: disallowed directory (rule: {rule})"))
        }
        ViolationType::DeniedFile {
            pattern_or_extension,
        } => {
            let rule = violation
                .triggering_rule_pattern
                .as_deref()
                .unwrap_or("global");
            Some(format!(
                "structure: denied file (matched: {pattern_or_extension}, rule: {rule})"
            ))
        }
        ViolationType::NamingConvention { expected_pattern } => {
            let rule = violation
                .triggering_rule_pattern
                .as_deref()
                .unwrap_or("unknown");
            Some(format!(
                "structure: naming convention violation (expected: {expected_pattern}, rule: {rule})"
            ))
        }
        ViolationType::MissingSibling {
            expected_sibling_pattern,
        } => {
            let rule = violation
                .triggering_rule_pattern
                .as_deref()
                .unwrap_or("unknown");
            Some(format!(
                "structure: missing sibling (expected: {expected_sibling_pattern}, rule: {rule})"
            ))
        }
        ViolationType::GroupIncomplete {
            missing_patterns, ..
        } => {
            let rule = violation
                .triggering_rule_pattern
                .as_deref()
                .unwrap_or("unknown");
            let missing = missing_patterns.join(", ");
            Some(format!(
                "structure: group incomplete (missing: {missing}, rule: {rule})"
            ))
        }
        ViolationType::DeniedDirectory { pattern } => {
            let rule = violation
                .triggering_rule_pattern
                .as_deref()
                .unwrap_or("global");
            Some(format!(
                "structure: denied directory (matched: {pattern}, rule: {rule})"
            ))
        }
    };

    if violation.is_warning {
        CheckResult::Warning {
            path: violation.path.clone(),
            stats,
            raw_stats: None, // Structure violations don't have raw vs effective stats
            limit: violation.limit,
            override_reason,
            suggestions: None,
            violation_category,
        }
    } else {
        CheckResult::Failed {
            path: violation.path.clone(),
            stats,
            raw_stats: None, // Structure violations don't have raw vs effective stats
            limit: violation.limit,
            override_reason,
            suggestions: None,
            violation_category,
        }
    }
}
