use crate::checker::{
    ContentExplanation, ContentRuleMatch, MatchStatus, StructureChecker, StructureExplanation,
    StructureRuleMatch, ThresholdChecker,
};
use crate::cli::{Cli, ExplainArgs, ExplainFormat};
use crate::error::SlocGuardError;
use crate::{EXIT_CONFIG_ERROR, EXIT_SUCCESS};

use super::context::load_config;

#[must_use]
pub fn run_explain(args: &ExplainArgs, cli: &Cli) -> i32 {
    match run_explain_impl(args, cli) {
        Ok(()) => EXIT_SUCCESS,
        Err(e) => {
            eprintln!("Error: {e}");
            EXIT_CONFIG_ERROR
        }
    }
}

pub(crate) fn run_explain_impl(args: &ExplainArgs, cli: &Cli) -> crate::Result<()> {
    let config = load_config(
        args.config.as_deref(),
        cli.no_config,
        cli.no_extends,
        cli.offline,
    )?;

    let path = &args.path;

    if path.is_file() {
        let checker = ThresholdChecker::new(config);
        let explanation = checker.explain(path);
        println!("{}", format_content_explanation(&explanation, args.format));
    } else if path.is_dir() {
        match StructureChecker::new(&config.structure) {
            Ok(checker) if checker.is_enabled() => {
                let explanation = checker.explain(path);
                println!(
                    "{}",
                    format_structure_explanation(&explanation, args.format)
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
        return Err(SlocGuardError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Path not found: {}", path.display()),
        )));
    }

    Ok(())
}

fn format_content_explanation(exp: &ContentExplanation, format: ExplainFormat) -> String {
    match format {
        ExplainFormat::Text => format_content_text(exp),
        ExplainFormat::Json => format_json(exp),
    }
}

fn format_structure_explanation(exp: &StructureExplanation, format: ExplainFormat) -> String {
    match format {
        ExplainFormat::Text => format_structure_text(exp),
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
        ContentRuleMatch::Override { index, reason } => {
            output.push_str(&format!(
                "  Matched: [[content.overrides]] index {index} (reason: {reason})\n"
            ));
        }
        ContentRuleMatch::Rule { index, pattern } => {
            output.push_str(&format!(
                "  Matched: [[content.rules]] index {index} pattern \"{pattern}\"\n"
            ));
        }
        ContentRuleMatch::Default => {
            output.push_str("  Matched: [content] defaults\n");
        }
    }

    output.push_str(&format!("  Limit:   {} lines\n", exp.effective_limit));
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )] // Precision loss is acceptable for display purposes
    let warn_at = (exp.effective_limit as f64 * exp.warn_threshold) as usize;
    output.push_str(&format!(
        "  Warn at: {} lines ({:.0}%)\n",
        warn_at,
        exp.warn_threshold * 100.0
    ));
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
        StructureRuleMatch::Override { index, reason } => {
            output.push_str(&format!(
                "  Matched: [[structure.overrides]] index {index} (reason: {reason})\n"
            ));
        }
        StructureRuleMatch::Rule { index, pattern } => {
            output.push_str(&format!(
                "  Matched: [[structure.rules]] index {index} pattern \"{pattern}\"\n"
            ));
        }
        StructureRuleMatch::Default => {
            output.push_str("  Matched: [structure] defaults\n");
        }
    }

    let max_files_str = exp.effective_max_files.map_or_else(
        || "none".to_string(),
        |v| {
            if v == -1 {
                "unlimited".to_string()
            } else {
                v.to_string()
            }
        },
    );
    let max_dirs_str = exp.effective_max_dirs.map_or_else(
        || "none".to_string(),
        |v| {
            if v == -1 {
                "unlimited".to_string()
            } else {
                v.to_string()
            }
        },
    );
    let max_depth_str = exp.effective_max_depth.map_or_else(
        || "none".to_string(),
        |v| {
            if v == -1 {
                "unlimited".to_string()
            } else {
                v.to_string()
            }
        },
    );

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

fn format_json<T: serde::Serialize>(exp: &T) -> String {
    serde_json::to_string_pretty(exp).unwrap_or_else(|e| format!("Error serializing to JSON: {e}"))
}

#[cfg(test)]
#[path = "explain_tests.rs"]
mod tests;
