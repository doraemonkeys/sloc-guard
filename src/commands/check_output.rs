use crate::checker::{CheckResult, StructureViolation, ViolationType};
use crate::counter::LineStats;
use crate::output::{
    HtmlFormatter, JsonFormatter, MarkdownFormatter, OutputFormat, OutputFormatter,
    SarifFormatter, TextFormatter,
};

pub(crate) fn format_output(
    format: OutputFormat,
    results: &[CheckResult],
    color_mode: crate::output::ColorMode,
    verbose: u8,
    show_suggestions: bool,
) -> crate::Result<String> {
    match format {
        OutputFormat::Text => TextFormatter::with_verbose(color_mode, verbose)
            .with_suggestions(show_suggestions)
            .format(results),
        OutputFormat::Json => JsonFormatter::new()
            .with_suggestions(show_suggestions)
            .format(results),
        OutputFormat::Sarif => SarifFormatter::new()
            .with_suggestions(show_suggestions)
            .format(results),
        OutputFormat::Markdown => MarkdownFormatter::new()
            .with_suggestions(show_suggestions)
            .format(results),
        OutputFormat::Html => HtmlFormatter::new()
            .with_suggestions(show_suggestions)
            .format(results),
    }
}

/// Convert a structure violation to a check result for unified output.
pub(crate) fn structure_violation_to_check_result(violation: &StructureViolation) -> CheckResult {
    // Create synthetic LineStats representing the violation
    // We use 'code' to represent the actual count for display purposes
    let stats = LineStats {
        total: violation.actual,
        code: violation.actual,
        comment: 0,
        blank: 0,
        ignored: 0,
    };

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
            limit: violation.limit,
            override_reason,
            suggestions: None,
        }
    } else {
        CheckResult::Failed {
            path: violation.path.clone(),
            stats,
            limit: violation.limit,
            override_reason,
            suggestions: None,
        }
    }
}

#[cfg(test)]
#[path = "check_output_tests.rs"]
mod tests;

