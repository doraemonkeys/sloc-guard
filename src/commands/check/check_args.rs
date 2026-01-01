use std::path::PathBuf;

use crate::cli::CheckArgs;

/// Validate structure params require explicit path and return resolved paths.
///
/// - If `--max-files`, `--max-dirs`, or `--max-depth` is specified, paths must be explicitly provided
/// - If no paths are provided and no structure params, defaults to current directory
pub fn validate_and_resolve_paths(args: &CheckArgs) -> crate::Result<Vec<PathBuf>> {
    let has_structure_params =
        args.max_files.is_some() || args.max_dirs.is_some() || args.max_depth.is_some();

    if args.paths.is_empty() {
        if has_structure_params {
            return Err(crate::SlocGuardError::Config(
                "--max-files/--max-dirs/--max-depth require a target <PATH>".to_string(),
            ));
        }
        // Default to current directory when no paths and no structure params
        Ok(vec![PathBuf::from(".")])
    } else {
        Ok(args.paths.clone())
    }
}

/// Apply CLI argument overrides to configuration.
///
/// CLI flags take precedence over config file values.
pub const fn apply_cli_overrides(config: &mut crate::config::Config, args: &CheckArgs) {
    if let Some(max_lines) = args.max_lines {
        config.content.max_lines = max_lines;
    }

    if args.count_comments {
        config.content.skip_comments = false;
    }

    if args.count_blank {
        config.content.skip_blank = false;
    }

    if let Some(warn_threshold) = args.warn_threshold {
        config.content.warn_threshold = warn_threshold;
    }

    // Apply CLI structure overrides (override defaults, not rules)
    if let Some(max_files) = args.max_files {
        config.structure.max_files = Some(max_files);
    }

    if let Some(max_dirs) = args.max_dirs {
        config.structure.max_dirs = Some(max_dirs);
    }

    if let Some(max_depth) = args.max_depth {
        config.structure.max_depth = Some(max_depth);
    }
}
