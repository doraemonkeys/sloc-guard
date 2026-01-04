use std::fmt::Write;

use crate::checker::{
    ContentExplanation, ContentRuleMatch, MatchStatus, StructureChecker, StructureExplanation,
    StructureRuleMatch, ThresholdChecker, WarnAtSource,
};
use crate::cli::{Cli, ExplainArgs, ExplainFormat};
use crate::config::{ConfigLoader, FetchPolicy, FileConfigLoader, SourcedConfig};
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
    // Handle --sources mode: show config inheritance chain
    if args.sources {
        return run_explain_sources(args, cli);
    }

    let load_result = load_config(
        args.config.as_deref(),
        cli.no_config,
        cli.no_extends,
        FetchPolicy::from_cli(cli.extends_policy),
    )?;
    let config = load_result.config;

    // Print preset info if a preset was used
    if let Some(ref preset_name) = load_result.preset_used {
        print_preset_info(preset_name);
    }

    // INVARIANT: Clap enforces path is required when --sources is not set
    let path = args.path.as_ref().expect("clap enforces path requirement");

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

/// Run explain --sources: show config inheritance chain and field sources.
fn run_explain_sources(args: &ExplainArgs, cli: &Cli) -> crate::Result<()> {
    if cli.no_config {
        println!("No configuration loaded (--no-config specified).");
        return Ok(());
    }

    let fetch_policy = FetchPolicy::from_cli(cli.extends_policy);
    let loader = FileConfigLoader::with_options(fetch_policy, None);

    let result = if cli.no_extends {
        // --no-extends: load single file only, don't follow extends chain
        if let Some(ref config_path) = args.config {
            loader.load_from_path_without_extends_with_sources(config_path)?
        } else {
            loader.load_without_extends_with_sources()?
        }
    } else if let Some(ref config_path) = args.config {
        loader.load_from_path_with_sources(config_path)?
    } else {
        loader.load_with_sources()?
    };

    let explanation = ConfigExplanation::from_load_result(&result);
    println!("{}", format_config_explanation(&explanation, args.format)?);

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

fn format_content_text(exp: &ContentExplanation) -> String {
    let mut output = String::new();

    let _ = writeln!(output, "Path: {}", exp.path.display());
    output.push('\n');
    output.push_str("Content Rules (SLOC Limits):\n");

    // Show matched rule
    match &exp.matched_rule {
        ContentRuleMatch::Excluded { pattern } => {
            let _ = writeln!(
                output,
                "  Status:  EXCLUDED (matches content.exclude pattern \"{pattern}\")"
            );
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
            let _ = writeln!(
                output,
                "  Matched: [[content.rules]] index {index} pattern \"{pattern}\"{reason_str}"
            );
        }
        ContentRuleMatch::Default => {
            output.push_str("  Matched: [content] defaults\n");
        }
    }

    let _ = writeln!(output, "  Limit:   {} lines", exp.effective_limit);

    // Show warn_at with context based on source (Rule vs Global, absolute vs percentage)
    let warn_at_str = match &exp.warn_at_source {
        WarnAtSource::RuleAbsolute { index } => {
            format!(
                "{} lines (from content.rules[{index}], absolute)",
                exp.effective_warn_at
            )
        }
        WarnAtSource::RulePercentage { index, threshold } => {
            format!(
                "{} lines (from content.rules[{index}], {:.0}%)",
                exp.effective_warn_at,
                threshold * 100.0
            )
        }
        WarnAtSource::GlobalAbsolute => {
            format!("{} lines (from [content], absolute)", exp.effective_warn_at)
        }
        WarnAtSource::GlobalPercentage { threshold } => {
            format!(
                "{} lines (from [content], {:.0}%)",
                exp.effective_warn_at,
                threshold * 100.0
            )
        }
    };
    let _ = writeln!(output, "  Warn at: {warn_at_str}");

    let _ = writeln!(
        output,
        "  Skip:    comments={}, blank={}",
        exp.skip_comments, exp.skip_blank
    );

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
        let _ = writeln!(
            output,
            "    [{status_char}] {}{} -> {} lines {status_desc}",
            candidate.source, pattern_str, candidate.limit
        );
    }

    output
}

fn format_structure_text(exp: &StructureExplanation) -> String {
    let mut output = String::new();

    let _ = writeln!(output, "Path: {}", exp.path.display());
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
            let _ = writeln!(
                output,
                "  Matched: [[structure.rules]] index {index} pattern \"{pattern}\"{reason_str}"
            );
        }
        StructureRuleMatch::Default => {
            output.push_str("  Matched: [structure] defaults\n");
        }
    }

    let max_files_str = format_limit(exp.effective_max_files);
    let max_dirs_str = format_limit(exp.effective_max_dirs);
    let max_depth_str = format_limit(exp.effective_max_depth);

    let _ = writeln!(
        output,
        "  Limits:  max_files={max_files_str}, max_dirs={max_dirs_str}, max_depth={max_depth_str}"
    );
    let _ = writeln!(output, "  Warn at: {:.0}%", exp.warn_threshold * 100.0);

    if let Some(reason) = &exp.override_reason {
        let _ = writeln!(output, "  Reason:  {reason}");
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
        let _ = writeln!(
            output,
            "    [{status_char}] {}{} -> files={files_str}, dirs={dirs_str}, depth={depth_str} {status_desc}",
            candidate.source, pattern_str
        );
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

// ============================================================================
// Config Source Chain Explanation
// ============================================================================

/// Key configuration fields tracked for `explain --sources` output.
///
/// Each entry is (`display_path`, `toml_path_parts`) where:
/// - `display_path`: Human-readable field path (e.g., `content.max_lines`)
/// - `toml_path_parts`: Path segments for TOML value lookup
///
/// # Curated Subset
///
/// This is an intentionally curated subset of Config fields most useful for
/// understanding inheritance behavior in `--sources` output. It does **not**
/// include every Config field—only those commonly overridden or queried.
///
/// # Maintenance
///
/// When fields are renamed/removed, the `key_fields_match_config_schema` test
/// will fail. However, adding new fields to Config won't cause test failures;
/// update this list manually if new fields warrant inclusion in `--sources`.
pub(crate) const KEY_FIELDS: &[(&str, &[&str])] = &[
    // Content settings (ContentConfig)
    ("content.max_lines", &["content", "max_lines"]),
    ("content.extensions", &["content", "extensions"]),
    ("content.warn_threshold", &["content", "warn_threshold"]),
    ("content.skip_comments", &["content", "skip_comments"]),
    ("content.skip_blank", &["content", "skip_blank"]),
    // Structure settings (StructureConfig)
    ("structure.max_files", &["structure", "max_files"]),
    ("structure.max_dirs", &["structure", "max_dirs"]),
    ("structure.max_depth", &["structure", "max_depth"]),
    ("structure.warn_threshold", &["structure", "warn_threshold"]),
    // Scanner settings (ScannerConfig)
    ("scanner.gitignore", &["scanner", "gitignore"]),
    ("scanner.exclude", &["scanner", "exclude"]),
    // Check settings (CheckConfig)
    ("check.warnings_as_errors", &["check", "warnings_as_errors"]),
    ("check.fail_fast", &["check", "fail_fast"]),
];

/// Explanation of configuration inheritance chain.
///
/// Shows which config sources were loaded and which fields came from where.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConfigExplanation {
    /// The inheritance chain from base to child (first = deepest base, last = local).
    pub chain: Vec<String>,
    /// Key fields with their effective values and originating sources.
    pub fields: Vec<FieldWithSource>,
}

/// A configuration field with its value and originating source.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct FieldWithSource {
    /// Field path (e.g., `content.max_lines`).
    pub field: String,
    /// Effective value as string.
    pub value: String,
    /// Which source provided this value (source name or path).
    pub source: String,
}

impl ConfigExplanation {
    /// Build a `ConfigExplanation` from a load result with sources.
    #[must_use]
    pub fn from_load_result(result: &crate::config::LoadResultWithSources) -> Self {
        let chain: Vec<String> = result
            .source_chain
            .iter()
            .map(|s| s.source.to_string())
            .collect();

        // Compute field sources for key configuration fields
        let fields = Self::compute_field_sources(&result.source_chain);

        Self { chain, fields }
    }

    /// Compute which source contributed each key field.
    ///
    /// For each field, walks the source chain from child to base (reverse order)
    /// and finds the first source that defines the field.
    fn compute_field_sources(source_chain: &[SourcedConfig]) -> Vec<FieldWithSource> {
        let mut fields = Vec::new();

        for (field_path, path_parts) in KEY_FIELDS {
            // Walk from child to base (reverse) to find the "winning" source
            for sourced in source_chain.iter().rev() {
                if let Some(value) = get_nested_value(&sourced.value, path_parts) {
                    fields.push(FieldWithSource {
                        field: (*field_path).to_string(),
                        value: format_toml_value(value),
                        source: sourced.source.to_string(),
                    });
                    break;
                }
            }
        }

        fields
    }
}

/// Get a nested value from a TOML value by path.
fn get_nested_value<'a>(value: &'a toml::Value, path: &[&str]) -> Option<&'a toml::Value> {
    let mut current = value;
    for &key in path {
        current = current.get(key)?;
    }
    Some(current)
}

/// Format a TOML value for display.
fn format_toml_value(value: &toml::Value) -> String {
    match value {
        toml::Value::String(s) => format!("\"{s}\""),
        toml::Value::Integer(n) => n.to_string(),
        toml::Value::Float(f) => f.to_string(),
        toml::Value::Boolean(b) => b.to_string(),
        toml::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(format_toml_value).collect();
            format!("[{}]", items.join(", "))
        }
        toml::Value::Table(_) => "{...}".to_string(),
        toml::Value::Datetime(dt) => dt.to_string(),
    }
}

fn format_config_explanation(
    exp: &ConfigExplanation,
    format: ExplainFormat,
) -> crate::Result<String> {
    match format {
        ExplainFormat::Text => Ok(format_config_text(exp)),
        ExplainFormat::Json => format_json(exp),
    }
}

fn format_config_text(exp: &ConfigExplanation) -> String {
    let mut output = String::new();

    output.push_str("Configuration Source Chain\n");
    output.push_str("==========================\n\n");

    if exp.chain.is_empty() {
        output.push_str("No configuration file found. Using defaults.\n");
        return output;
    }

    output.push_str("Inheritance Chain (base → child):\n");
    for (i, source) in exp.chain.iter().enumerate() {
        let prefix = if i == 0 { "  " } else { "  ↓ " };
        let _ = writeln!(output, "{prefix}{source}");
    }

    output.push('\n');
    output.push_str("Field Sources:\n");
    output.push_str("--------------\n");

    if exp.fields.is_empty() {
        output.push_str("  (no fields configured)\n");
    } else {
        // Group fields by section for better readability
        let mut current_section = "";
        for field in &exp.fields {
            let section = field.field.split('.').next().unwrap_or("");
            if section != current_section {
                if !current_section.is_empty() {
                    output.push('\n');
                }
                current_section = section;
                let _ = writeln!(output, "  [{section}]");
            }
            let field_name = field.field.split('.').nth(1).unwrap_or(&field.field);
            let _ = writeln!(
                output,
                "    {field_name} = {} (from {})",
                field.value, field.source
            );
        }
    }

    output
}

#[cfg(test)]
#[path = "explain_tests.rs"]
mod tests;
