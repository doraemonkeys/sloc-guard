use std::fmt::Write;
use std::path::Path;

use crate::checker::{
    ContentExplanation, ContentRuleMatch, CountExcludeSource, DirInventory, DirInventorySource,
    DirStats, MatchStatus, StructureExplanation, StructureRuleMatch, WarnAtSource,
};
use crate::cli::{Cli, ExplainArgs, ExplainFormat};
use crate::config::FetchPolicy;
use crate::error::SlocGuardError;
use crate::project::ProjectPaths;
use crate::{EXIT_CONFIG_ERROR, EXIT_SUCCESS};

use super::config_notice::{print_config_notice, print_preset_notice};
use super::context::{
    CheckContext, color_choice_to_mode, load_config, resolve_config_root, resolve_project_root,
};
use super::explain_sources::run_explain_sources;

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
    let human_text = matches!(args.format, ExplainFormat::Text);
    let color_mode = color_choice_to_mode(cli.color);
    print_config_notice(
        &load_result.origin,
        cli.quiet,
        cli.verbose,
        human_text,
        color_mode,
    );
    let project_root = resolve_project_root();
    let config_root = resolve_config_root(&load_result, &project_root);
    let project_paths = ProjectPaths::rooted(config_root);
    let config = load_result.config;

    // Print preset info if a preset was used
    if let Some(ref preset_name) = load_result.preset_used {
        print_preset_notice(preset_name, cli.quiet, cli.verbose, human_text, color_mode);
    }

    // INVARIANT: Clap enforces path is required when --sources is not set
    let path = args.path.as_ref().expect("clap enforces path requirement");
    let logical_path = project_paths.logical(path);

    // Explain reports what `check` would enforce, so it builds the same
    // context `check` builds from this configuration (checkers plus the
    // config-driven scanner) instead of a parallel approximation. Check-time
    // CLI flags (`--exclude`, `--no-gitignore`) have no counterpart here.
    let ctx = CheckContext::from_config_with_project_paths(
        &config,
        config.content.warn_threshold,
        config.scanner.exclude.clone(),
        config.scanner.gitignore,
        project_paths,
    )?;

    if path.is_file() {
        let explanation = ctx.threshold_checker.explain(&logical_path);
        println!("{}", format_content_explanation(&explanation, args.format)?);
    } else if path.is_dir() {
        match ctx.structure_checker.as_ref() {
            Some(checker) if checker.is_enabled() => {
                let stats = scan_dir_inventory(&ctx, path, &logical_path)?;
                let inventory = stats.as_ref().map_or(
                    DirInventorySource::ExcludedFromScan,
                    DirInventorySource::ConfiguredScan,
                );
                let explanation = checker.explain(&logical_path, inventory);
                println!(
                    "{}",
                    format_structure_explanation(&explanation, args.format)?
                );
            }
            _ => {
                println!("Path: {}", logical_path.display());
                println!();
                println!("No structure rules configured.");
                println!("Add [structure] section to your config to enable directory limits.");
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

/// Inventory a directory's immediate children via the config-driven scan.
///
/// Runs the same structure-aware scan `check` runs — scanner excludes,
/// gitignore, and no-follow symlink handling included — rooted at the
/// configuration root, so a directory the scan prunes is absent here exactly
/// as it is absent from `check`'s roster. A directory outside the
/// configuration root is scanned as its own root, mirroring `check <dir>`.
/// Returns `None` when the scan never reaches the directory.
fn scan_dir_inventory(
    ctx: &CheckContext,
    path: &Path,
    logical_path: &Path,
) -> crate::Result<Option<DirStats>> {
    let scan_root = if logical_path.is_absolute() || logical_path.starts_with("..") {
        path.to_path_buf()
    } else {
        ctx.project_paths().physical(Path::new("."))
    };
    let mut scan_result = ctx
        .scanner
        .scan_all_with_structure(&[scan_root], ctx.structure_scan_config.as_ref())?;
    scan_result.rebase_logical_paths(ctx.project_paths());
    Ok(scan_result.dir_stats.remove(logical_path))
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

    match &exp.inventory {
        DirInventory::ConfiguredScan { counts } => {
            let _ = writeln!(
                output,
                "  Counts:  files={} raw -> {} effective, dirs={} raw -> {} effective",
                counts.raw_file_count,
                counts.effective_file_count,
                counts.raw_dir_count,
                counts.effective_dir_count
            );
            output.push_str(
                "  Note:    counts reflect the configured scan (scanner excludes + gitignore); check-time CLI flags such as --exclude are not visible here\n",
            );
        }
        DirInventory::ExcludedFromScan => {
            output.push_str(
                "  Counts:  unavailable - the configured scan excludes this directory, so check does not evaluate its limits\n",
            );
        }
        DirInventory::NotScanned => {}
    }

    if let Some(reason) = &exp.override_reason {
        let _ = writeln!(output, "  Reason:  {reason}");
    }

    format_count_exclude_section(exp, &mut output);

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

/// Render the active count-exclusion patterns with provenance and hits.
///
/// Hit lists are rendered only when an inventory was available (the entry
/// carries hits); directories are marked with a trailing `/`.
fn format_count_exclude_section(exp: &StructureExplanation, output: &mut String) {
    if exp.count_exclude.is_empty() {
        return;
    }

    output.push('\n');
    output.push_str("  Count Exclude (global + matched rule):\n");
    for entry in &exp.count_exclude {
        let source_str = match &entry.source {
            CountExcludeSource::Global => "[structure]".to_string(),
            CountExcludeSource::Rule { index, scope } => {
                format!("structure.rules[{index}] \"{scope}\"")
            }
        };
        let hits_str = entry.hits.as_ref().map_or_else(String::new, |hits| {
            let rendered: Vec<String> = hits
                .files
                .iter()
                .cloned()
                .chain(hits.dirs.iter().map(|dir| format!("{dir}/")))
                .collect();
            if rendered.is_empty() {
                " -> excluded: (none)".to_string()
            } else {
                format!(" -> excluded: {}", rendered.join(", "))
            }
        });
        let _ = writeln!(
            output,
            "    \"{}\" (from {source_str}){hits_str}",
            entry.pattern
        );
    }
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

pub(super) fn format_json<T: serde::Serialize>(exp: &T) -> crate::Result<String> {
    Ok(serde_json::to_string_pretty(exp)?)
}

#[cfg(test)]
#[path = "explain_tests/mod.rs"]
mod tests;
