use crate::checker::{
    ContentExplanation, ContentRuleMatch, MatchStatus, StructureChecker, StructureExplanation,
    StructureRuleMatch, ThresholdChecker, WarnAtSource,
};
use crate::cli::{Cli, ExplainArgs, ExplainFormat};
use crate::error::SlocGuardError;
use crate::{EXIT_CONFIG_ERROR, EXIT_SUCCESS};

use super::context::{load_config, print_preset_info};

#[must_use]
pub fn run_explain(args: &ExplainArgs, cli: &Cli) -> i32 {
    match run_explain_impl(args, cli) {
        Ok(()) => EXIT_SUCCESS,
        Err(e) => {
            crate::output::print_error_full(
                e.error_type(),
                &e.message(),
                e.detail().as_deref(),
                None,
            );
            EXIT_CONFIG_ERROR
        }
    }
}

pub(crate) fn run_explain_impl(args: &ExplainArgs, cli: &Cli) -> crate::Result<()> {
    let load_result = load_config(
        args.config.as_deref(),
        cli.no_config,
        cli.no_extends,
        cli.offline,
    )?;
    let config = load_result.config;

    // Print preset info if a preset was used
    if let Some(ref preset_name) = load_result.preset_used {
        print_preset_info(preset_name);
    }

    let path = &args.path;

    if path.is_file() {
        let checker = ThresholdChecker::new(config)?;
        let explanation = checker.explain(path);
        println!("{}", format_content_explanation(&explanation, args.format)?);
    } else if path.is_dir() {
        match StructureChecker::new(&config.structure) {
            Ok(checker) if checker.is_enabled() => {
                let explanation = checker.explain(path);
                println!(
                    "{}",
                    format_structure_explanation(&explanation, args.format)?
                );
            }
            Ok(_) => {
                println!("Path: {}", path.display());
                println!();
                println!("No structure rules configured.");
                println!("Add [structure] section to your config to enable directory limits.");
            }
            Err(e) => {
                return Err(e);
            }
        }
    } else {
        return Err(SlocGuardError::io_with_path(
            std::io::Error::new(std::io::ErrorKind::NotFound, "Path not found"),
            path.clone(),
        ));
    }

    Ok(())
}

fn format_content_explanation(
    exp: &ContentExplanation,
    format: ExplainFormat,
) -> crate::Result<String> {
    match format {
        ExplainFormat::Text => Ok(format_content_text(exp)),
        ExplainFormat::Json => format_json(exp),
    }
}

fn format_structure_explanation(
    exp: &StructureExplanation,
    format: ExplainFormat,
) -> crate::Result<String> {
    match format {
        ExplainFormat::Text => Ok(format_structure_text(exp)),
        ExplainFormat::Json => format_json(exp),
    }
}

#[allow(clippy::format_push_string)] // Performance is not critical for explanation output
fn format_content_text(exp: &ContentExplanation) -> String {
    let mut output = String::new();

    output.push_str(&format!("Path: {}\n", exp.path.display()));
    output.push('\n');
    output.push_str("Content Rules (SLOC Limits):\n");

    // Show matched rule
    match &exp.matched_rule {
        ContentRuleMatch::Excluded { pattern } => {
            output.push_str(&format!(
                "  Status:  EXCLUDED (matches content.exclude pattern \"{pattern}\")\n"
            ));
            output.push_str(
                "  Note:    This file is excluded from SLOC counting but visible for structure checks.\n",
            );
            return output;
        }
        ContentRuleMatch::Rule {
            index,
            pattern,
            reason,
        } => {
            let reason_str = reason
                .as_ref()
                .map(|r| format!(" (reason: {r})"))
                .unwrap_or_default();
            output.push_str(&format!(
                "  Matched: [[content.rules]] index {index} pattern \"{pattern}\"{reason_str}\n"
            ));
        }
        ContentRuleMatch::Default => {
            output.push_str("  Matched: [content] defaults\n");
        }
    }

    output.push_str(&format!("  Limit:   {} lines\n", exp.effective_limit));

    // Show warn_at with context based on source
    let warn_at_str = match &exp.warn_at_source {
        WarnAtSource::RuleAbsolute { .. } | WarnAtSource::GlobalAbsolute => {
            format!("{} lines (absolute)", exp.effective_warn_at)
        }
        WarnAtSource::RulePercentage { threshold, .. }
        | WarnAtSource::GlobalPercentage { threshold } => {
            format!(
                "{} lines ({:.0}%)",
                exp.effective_warn_at,
                threshold * 100.0
            )
        }
    };
    output.push_str(&format!("  Warn at: {warn_at_str}\n"));

    output.push_str(&format!(
        "  Skip:    comments={}, blank={}\n",
        exp.skip_comments, exp.skip_blank
    ));

    output.push('\n');
    output.push_str("  Rule Chain (evaluated high->low):\n");
    for candidate in &exp.rule_chain {
        let status_char = match candidate.status {
            MatchStatus::Matched => "+",
            MatchStatus::Superseded => "-",
            MatchStatus::NoMatch => " ",
        };
        let pattern_str = candidate
            .pattern
            .as_ref()
            .map_or(String::new(), |p| format!(" \"{p}\""));
        let status_desc = match candidate.status {
            MatchStatus::Matched => "(MATCHED)",
            MatchStatus::Superseded => "(superseded)",
            MatchStatus::NoMatch => "(no match)",
        };
        output.push_str(&format!(
            "    [{status_char}] {}{} -> {} lines {status_desc}\n",
            candidate.source, pattern_str, candidate.limit
        ));
    }

    output
}

#[allow(clippy::format_push_string)] // Performance is not critical for explanation output
fn format_structure_text(exp: &StructureExplanation) -> String {
    let mut output = String::new();

    output.push_str(&format!("Path: {}\n", exp.path.display()));
    output.push('\n');
    output.push_str("Structure Rules (Directory Limits):\n");

    // Show matched rule
    match &exp.matched_rule {
        StructureRuleMatch::Rule {
            index,
            pattern,
            reason,
        } => {
            let reason_str = reason
                .as_ref()
                .map(|r| format!(" (reason: {r})"))
                .unwrap_or_default();
            output.push_str(&format!(
                "  Matched: [[structure.rules]] index {index} pattern \"{pattern}\"{reason_str}\n"
            ));
        }
        StructureRuleMatch::Default => {
            output.push_str("  Matched: [structure] defaults\n");
        }
    }

    let max_files_str = format_limit(exp.effective_max_files);
    let max_dirs_str = format_limit(exp.effective_max_dirs);
    let max_depth_str = format_limit(exp.effective_max_depth);

    output.push_str(&format!(
        "  Limits:  max_files={max_files_str}, max_dirs={max_dirs_str}, max_depth={max_depth_str}\n"
    ));
    output.push_str(&format!("  Warn at: {:.0}%\n", exp.warn_threshold * 100.0));

    if let Some(reason) = &exp.override_reason {
        output.push_str(&format!("  Reason:  {reason}\n"));
    }

    output.push('\n');
    output.push_str("  Rule Chain (evaluated high->low):\n");
    for candidate in &exp.rule_chain {
        let status_char = match candidate.status {
            MatchStatus::Matched => "+",
            MatchStatus::Superseded => "-",
            MatchStatus::NoMatch => " ",
        };
        let pattern_str = candidate
            .pattern
            .as_ref()
            .map_or(String::new(), |p| format!(" \"{p}\""));
        let status_desc = match candidate.status {
            MatchStatus::Matched => "(MATCHED)",
            MatchStatus::Superseded => "(superseded)",
            MatchStatus::NoMatch => "(no match)",
        };
        let files_str = candidate
            .max_files
            .map_or_else(|| "-".to_string(), |v| v.to_string());
        let dirs_str = candidate
            .max_dirs
            .map_or_else(|| "-".to_string(), |v| v.to_string());
        let depth_str = candidate
            .max_depth
            .map_or_else(|| "-".to_string(), |v| v.to_string());
        output.push_str(&format!(
            "    [{status_char}] {}{} -> files={files_str}, dirs={dirs_str}, depth={depth_str} {status_desc}\n",
            candidate.source, pattern_str
        ));
    }

    output
}

/// Format an optional limit value for display.
/// - `None` → "none" (no limit configured)
/// - `Some(-1)` → "unlimited" (explicitly unlimited)
/// - `Some(n)` → numeric string
fn format_limit(value: Option<i64>) -> String {
    match value {
        None => "none".to_string(),
        Some(-1) => "unlimited".to_string(),
        Some(v) => v.to_string(),
    }
}

fn format_json<T: serde::Serialize>(exp: &T) -> crate::Result<String> {
    Ok(serde_json::to_string_pretty(exp)?)
}

#[cfg(test)]
#[path = "explain_tests.rs"]
mod tests;
