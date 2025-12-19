use std::path::Path;
use std::sync::Mutex;

use rayon::prelude::*;

use crate::baseline::{Baseline, compute_file_hash};
use crate::cache::{Cache, compute_config_hash};
use crate::checker::{CheckResult, Checker, ThresholdChecker};
use crate::cli::{BaselineAction, BaselineArgs, BaselineUpdateArgs, Cli};
use crate::language::LanguageRegistry;
use crate::output::ScanProgress;
use crate::scanner::scan_files;
use crate::{EXIT_CONFIG_ERROR, EXIT_SUCCESS};

use super::check::compute_effective_stats;
use super::context::{
    FileReader, RealFileReader, load_config, process_file_with_cache, resolve_scan_paths,
};

#[must_use]
pub fn run_baseline(args: &BaselineArgs, cli: &Cli) -> i32 {
    match &args.action {
        BaselineAction::Update(update_args) => run_baseline_update(update_args, cli),
    }
}

fn run_baseline_update(args: &BaselineUpdateArgs, cli: &Cli) -> i32 {
    // Deprecation warning
    eprintln!("Warning: `baseline update` is deprecated. Use `check --update-baseline` instead.");

    match run_baseline_update_impl(args, cli) {
        Ok(count) => {
            if !cli.quiet {
                println!(
                    "Baseline created with {} file(s): {}",
                    count,
                    args.output.display()
                );
            }
            EXIT_SUCCESS
        }
        Err(e) => {
            eprintln!("Error: {e}");
            EXIT_CONFIG_ERROR
        }
    }
}

pub(crate) fn run_baseline_update_impl(
    args: &BaselineUpdateArgs,
    cli: &Cli,
) -> crate::Result<usize> {
    // 1. Load configuration
    let mut config = load_config(args.config.as_deref(), cli.no_config, cli.no_extends)?;

    // 2. Prepare exclude patterns from scanner config
    let mut exclude_patterns = config.scanner.exclude.clone();
    exclude_patterns.extend(args.exclude.clone());

    // Apply CLI extensions override if provided
    if let Some(ref cli_extensions) = args.ext {
        config.content.extensions.clone_from(cli_extensions);
    }

    // 3. Determine paths to scan
    let paths_to_scan = resolve_scan_paths(&args.paths, &args.include, &config);

    // 4. Scan directories (respecting .gitignore if enabled)
    // Scanner returns ALL files, extension filtering is done by ThresholdChecker
    let use_gitignore = config.scanner.gitignore && !args.no_gitignore;
    let all_files = scan_files(&paths_to_scan, &exclude_patterns, use_gitignore)?;

    // 5. Process each file and find violations
    let registry = LanguageRegistry::with_custom_languages(&config.languages);
    let checker =
        ThresholdChecker::new(config.clone()).with_warning_threshold(config.content.warn_threshold);
    let cache = Mutex::new(Cache::new(compute_config_hash(&config)));
    let reader = RealFileReader;

    let progress = ScanProgress::new(all_files.len() as u64, cli.quiet);
    let violations: Vec<_> = all_files
        .par_iter()
        .filter(|file_path| checker.should_process(file_path)) // Filter by extension
        .filter_map(|file_path| {
            let result = process_file_for_baseline(file_path, &registry, &checker, &cache, &reader);
            progress.inc();
            result
                .filter(CheckResult::is_failed)
                .map(|r| (file_path.clone(), r.stats().code))
        })
        .collect();
    progress.finish();

    // 6. Create baseline from violations
    let mut baseline = Baseline::new();
    for (path, lines) in &violations {
        let path_str = path.to_string_lossy().replace('\\', "/");
        let hash = compute_file_hash(path)?;
        baseline.set_content(&path_str, *lines, hash);
    }

    // 7. Save baseline to file
    baseline.save(&args.output)?;

    Ok(violations.len())
}

fn process_file_for_baseline(
    file_path: &Path,
    registry: &LanguageRegistry,
    checker: &ThresholdChecker,
    cache: &Mutex<Cache>,
    reader: &dyn FileReader,
) -> Option<CheckResult> {
    let (stats, _language) = process_file_with_cache(file_path, registry, cache, reader)?;
    let (skip_comments, skip_blank) = checker.get_skip_settings_for_path(file_path);
    let effective_stats = compute_effective_stats(&stats, skip_comments, skip_blank);
    Some(checker.check(file_path, &effective_stats))
}

#[cfg(test)]
#[path = "baseline_tests.rs"]
mod tests;
