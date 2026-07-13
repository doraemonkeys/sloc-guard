//! Configuration source chain explanation (`explain --sources`).
//!
//! Shows the extends/preset inheritance chain and which source contributed
//! each key configuration field.

use std::fmt::Write;

use crate::cli::{Cli, ExplainArgs, ExplainFormat};
use crate::config::{ConfigLoader, FetchPolicy, FileConfigLoader, SourcedConfig};

use super::context::resolve_project_root;
use super::explain::format_json;

/// Run explain --sources: show config inheritance chain and field sources.
pub(crate) fn run_explain_sources(args: &ExplainArgs, cli: &Cli) -> crate::Result<()> {
    if cli.no_config {
        println!("No configuration loaded (--no-config specified).");
        return Ok(());
    }

    let fetch_policy = FetchPolicy::from_cli(cli.extends_policy);
    let loader = FileConfigLoader::with_options(fetch_policy, Some(resolve_project_root()));

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
#[path = "explain_tests/config_sources_tests.rs"]
mod tests;
