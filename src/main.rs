use std::fs::{self, File};
use std::io::BufReader;
use std::path::Path;
use std::sync::Mutex;

use clap::Parser;
use rayon::prelude::*;

use sloc_guard::baseline::{compute_file_hash, Baseline};
use sloc_guard::cache::{compute_config_hash, Cache};
use sloc_guard::checker::{Checker, ThresholdChecker};
use sloc_guard::cli::{
    BaselineAction, BaselineArgs, BaselineUpdateArgs, CheckArgs, Cli, ColorChoice, Commands,
    GroupBy, StatsArgs,
};
use sloc_guard::commands::{run_config, run_init};
use sloc_guard::config::{Config, ConfigLoader, FileConfigLoader};
use sloc_guard::counter::{CountResult, LineStats, SlocCounter};
use sloc_guard::git::{ChangedFiles, GitDiff};
use sloc_guard::language::LanguageRegistry;
use sloc_guard::output::{
    ColorMode, FileStatistics, JsonFormatter, OutputFormat, OutputFormatter, ProjectStatistics,
    SarifFormatter, ScanProgress, StatsFormatter, StatsJsonFormatter, StatsTextFormatter,
    TextFormatter,
};
use sloc_guard::scanner::{DirectoryScanner, FileScanner, GlobFilter};
use sloc_guard::{EXIT_CONFIG_ERROR, EXIT_SUCCESS, EXIT_THRESHOLD_EXCEEDED};

/// File size threshold for streaming reads (10 MB)
const LARGE_FILE_THRESHOLD: u64 = 10 * 1024 * 1024;

/// Default cache file path
const DEFAULT_CACHE_PATH: &str = ".sloc-guard-cache.json";

const fn color_choice_to_mode(choice: ColorChoice) -> ColorMode {
    match choice {
        ColorChoice::Auto => ColorMode::Auto,
        ColorChoice::Always => ColorMode::Always,
        ColorChoice::Never => ColorMode::Never,
    }
}

fn main() {
    let cli = Cli::parse();

    let exit_code = match &cli.command {
        Commands::Check(args) => run_check(args, &cli),
        Commands::Stats(args) => run_stats(args, &cli),
        Commands::Init(args) => run_init(args),
        Commands::Config(args) => run_config(args),
        Commands::Baseline(args) => run_baseline(args, &cli),
    };

    std::process::exit(exit_code);
}

fn run_check(args: &CheckArgs, cli: &Cli) -> i32 {
    match run_check_impl(args, cli) {
        Ok(exit_code) => exit_code,
        Err(e) => {
            eprintln!("Error: {e}");
            EXIT_CONFIG_ERROR
        }
    }
}

fn run_check_impl(args: &CheckArgs, cli: &Cli) -> sloc_guard::Result<i32> {
    // 1. Load configuration
    let mut config = load_config(args.config.as_deref(), cli.no_config)?;

    // 2. Apply CLI argument overrides
    apply_cli_overrides(&mut config, args);

    // 3. Load baseline if specified
    let baseline = load_baseline(args.baseline.as_deref())?;

    // 3.1 Load cache if not disabled
    let config_hash = compute_config_hash(&config);
    let cache = if args.no_cache {
        None
    } else {
        load_cache(&config_hash)
    };
    let cache = Mutex::new(cache.unwrap_or_else(|| Cache::new(config_hash)));

    // 4. Create GlobFilter
    let extensions = args
        .ext
        .clone()
        .unwrap_or_else(|| config.default.extensions.clone());
    let mut exclude_patterns = config.exclude.patterns.clone();
    exclude_patterns.extend(args.exclude.clone());
    let filter = GlobFilter::new(extensions, &exclude_patterns)?;

    // 5. Determine paths to scan
    let paths_to_scan = get_scan_paths(args, &config);

    // 6. Scan directories
    let scanner = DirectoryScanner::new(filter);
    let mut all_files = Vec::new();
    for path in &paths_to_scan {
        let files = scanner.scan(path)?;
        all_files.extend(files);
    }

    // 6.1 Filter by git diff if --diff is specified
    let all_files = filter_by_git_diff(all_files, args.diff.as_deref())?;

    // 7. Process each file (parallel with rayon)
    let registry = LanguageRegistry::default();
    let warn_threshold = args.warn_threshold.unwrap_or(config.default.warn_threshold);
    let checker = ThresholdChecker::new(config.clone()).with_warning_threshold(warn_threshold);

    let skip_comments = config.default.skip_comments && !args.no_skip_comments;
    let skip_blank = config.default.skip_blank && !args.no_skip_blank;

    let progress = ScanProgress::new(all_files.len() as u64, cli.quiet);
    let mut results: Vec<_> = all_files
        .par_iter()
        .filter_map(|file_path| {
            let result = process_file_cached(
                file_path,
                &registry,
                &checker,
                skip_comments,
                skip_blank,
                &cache,
            );
            progress.inc();
            result
        })
        .collect();
    progress.finish();

    // 7.1 Save cache if not disabled
    #[allow(clippy::collapsible_if)]
    if !args.no_cache {
        if let Ok(cache_guard) = cache.lock() {
            save_cache(&cache_guard);
        }
    }

    // 8. Apply baseline comparison: mark failures as grandfathered if in baseline
    if let Some(ref baseline) = baseline {
        apply_baseline_comparison(&mut results, baseline);
    }

    // 9. Format output
    let color_mode = color_choice_to_mode(cli.color);
    let output = format_output(args.format, &results, color_mode, cli.verbose)?;

    // 10. Write output
    write_output(args.output.as_deref(), &output, cli.quiet)?;

    // 11. Determine exit code
    let has_failures = results
        .iter()
        .any(sloc_guard::checker::CheckResult::is_failed);
    let has_warnings = results
        .iter()
        .any(sloc_guard::checker::CheckResult::is_warning);

    if args.warn_only {
        return Ok(EXIT_SUCCESS);
    }

    // Strict mode: CLI flag takes precedence, otherwise use config
    let strict = args.strict || config.default.strict;

    if has_failures || (strict && has_warnings) {
        Ok(EXIT_THRESHOLD_EXCEEDED)
    } else {
        Ok(EXIT_SUCCESS)
    }
}

fn load_config(config_path: Option<&Path>, no_config: bool) -> sloc_guard::Result<Config> {
    if no_config {
        return Ok(Config::default());
    }

    let loader = FileConfigLoader::new();
    config_path.map_or_else(|| loader.load(), |path| loader.load_from_path(path))
}

fn load_baseline(baseline_path: Option<&Path>) -> sloc_guard::Result<Option<Baseline>> {
    let Some(path) = baseline_path else {
        return Ok(None);
    };

    if !path.exists() {
        return Err(sloc_guard::SlocGuardError::Config(format!(
            "Baseline file not found: {}",
            path.display()
        )));
    }

    Ok(Some(Baseline::load(path)?))
}

fn load_cache(config_hash: &str) -> Option<Cache> {
    let cache_path = Path::new(DEFAULT_CACHE_PATH);
    if !cache_path.exists() {
        return None;
    }

    Cache::load(cache_path)
        .ok()
        .filter(|cache| cache.is_valid(config_hash))
}

fn save_cache(cache: &Cache) {
    let cache_path = Path::new(DEFAULT_CACHE_PATH);
    // Silently ignore errors when saving cache
    let _ = cache.save(cache_path);
}

fn apply_baseline_comparison(results: &mut [sloc_guard::checker::CheckResult], baseline: &Baseline) {
    for result in results.iter_mut() {
        if !result.is_failed() {
            continue;
        }

        let path_str = result.path.to_string_lossy().replace('\\', "/");
        if baseline.contains(&path_str) {
            result.set_grandfathered();
        }
    }
}

const fn apply_cli_overrides(config: &mut Config, args: &CheckArgs) {
    if let Some(max_lines) = args.max_lines {
        config.default.max_lines = max_lines;
    }

    if args.no_skip_comments {
        config.default.skip_comments = false;
    }

    if args.no_skip_blank {
        config.default.skip_blank = false;
    }

    if let Some(warn_threshold) = args.warn_threshold {
        config.default.warn_threshold = warn_threshold;
    }
}

fn get_scan_paths(args: &CheckArgs, config: &Config) -> Vec<std::path::PathBuf> {
    // CLI --include overrides config include_paths
    if !args.include.is_empty() {
        return args.include.iter().map(std::path::PathBuf::from).collect();
    }

    // If CLI paths provided (other than default "."), use them
    let default_path = std::path::PathBuf::from(".");
    if args.paths.len() != 1 || args.paths[0] != default_path {
        return args.paths.clone();
    }

    // Use config include_paths if available
    if !config.default.include_paths.is_empty() {
        return config
            .default
            .include_paths
            .iter()
            .map(std::path::PathBuf::from)
            .collect();
    }

    // Default to current directory
    args.paths.clone()
}

fn filter_by_git_diff(
    files: Vec<std::path::PathBuf>,
    diff_ref: Option<&str>,
) -> sloc_guard::Result<Vec<std::path::PathBuf>> {
    let Some(base_ref) = diff_ref else {
        return Ok(files);
    };

    // Discover git repository from current directory
    let git_diff = GitDiff::discover(Path::new("."))?;
    let changed_files = git_diff.get_changed_files(base_ref)?;

    // Canonicalize paths for comparison
    let changed_canonical: std::collections::HashSet<_> = changed_files
        .iter()
        .filter_map(|p| p.canonicalize().ok())
        .collect();

    // Filter to only include changed files
    let filtered: Vec<_> = files
        .into_iter()
        .filter(|f| {
            f.canonicalize()
                .ok()
                .is_some_and(|canon| changed_canonical.contains(&canon))
        })
        .collect();

    Ok(filtered)
}

fn process_file(
    file_path: &Path,
    registry: &LanguageRegistry,
    checker: &ThresholdChecker,
    skip_comments: bool,
    skip_blank: bool,
) -> Option<sloc_guard::checker::CheckResult> {
    let ext = file_path.extension()?.to_str()?;
    let language = registry.get_by_extension(ext)?;

    let counter = SlocCounter::new(&language.comment_syntax);
    let stats = count_file_lines(file_path, &counter)?;

    let effective_stats = compute_effective_stats(&stats, skip_comments, skip_blank);

    Some(checker.check(file_path, &effective_stats))
}

fn process_file_cached(
    file_path: &Path,
    registry: &LanguageRegistry,
    checker: &ThresholdChecker,
    skip_comments: bool,
    skip_blank: bool,
    cache: &Mutex<Cache>,
) -> Option<sloc_guard::checker::CheckResult> {
    let ext = file_path.extension()?.to_str()?;
    let language = registry.get_by_extension(ext)?;

    let path_key = file_path.to_string_lossy().replace('\\', "/");
    let file_hash = compute_file_hash(file_path).ok()?;

    // Try to get stats from cache
    let stats = {
        let cache_guard = cache.lock().ok()?;
        cache_guard
            .get_if_valid(&path_key, &file_hash)
            .map(|entry| LineStats::from(&entry.stats))
    };

    let stats = if let Some(cached_stats) = stats {
        cached_stats
    } else {
        // Cache miss: count lines and update cache
        let counter = SlocCounter::new(&language.comment_syntax);
        let stats = count_file_lines(file_path, &counter)?;

        // Update cache
        if let Ok(mut cache_guard) = cache.lock() {
            cache_guard.set(&path_key, file_hash, &stats);
        }

        stats
    };

    let effective_stats = compute_effective_stats(&stats, skip_comments, skip_blank);

    Some(checker.check(file_path, &effective_stats))
}

fn count_file_lines(file_path: &Path, counter: &SlocCounter) -> Option<LineStats> {
    let metadata = fs::metadata(file_path).ok()?;

    let result = if metadata.len() >= LARGE_FILE_THRESHOLD {
        let file = File::open(file_path).ok()?;
        let reader = BufReader::new(file);
        counter.count_reader(reader).ok()?
    } else {
        let content = fs::read_to_string(file_path).ok()?;
        counter.count(&content)
    };

    match result {
        CountResult::Stats(stats) => Some(stats),
        CountResult::IgnoredFile => None,
    }
}

fn compute_effective_stats(stats: &LineStats, skip_comments: bool, skip_blank: bool) -> LineStats {
    let mut effective = stats.clone();

    // If not skipping comments, add them to code count
    if !skip_comments {
        effective.code += effective.comment;
        effective.comment = 0;
    }

    // If not skipping blanks, add them to code count
    if !skip_blank {
        effective.code += effective.blank;
        effective.blank = 0;
    }

    effective
}

fn format_output(
    format: OutputFormat,
    results: &[sloc_guard::checker::CheckResult],
    color_mode: ColorMode,
    verbose: u8,
) -> sloc_guard::Result<String> {
    match format {
        OutputFormat::Text => TextFormatter::with_verbose(color_mode, verbose).format(results),
        OutputFormat::Json => JsonFormatter.format(results),
        OutputFormat::Sarif => SarifFormatter.format(results),
        OutputFormat::Markdown => Err(sloc_guard::SlocGuardError::Config(
            "Markdown output format is not yet implemented".to_string(),
        )),
    }
}

fn write_output(output_path: Option<&Path>, content: &str, quiet: bool) -> sloc_guard::Result<()> {
    if let Some(path) = output_path {
        fs::write(path, content)?;
    } else if !quiet {
        print!("{content}");
    }
    Ok(())
}

fn run_stats(args: &StatsArgs, cli: &Cli) -> i32 {
    match run_stats_impl(args, cli) {
        Ok(exit_code) => exit_code,
        Err(e) => {
            eprintln!("Error: {e}");
            EXIT_CONFIG_ERROR
        }
    }
}

fn run_stats_impl(args: &StatsArgs, cli: &Cli) -> sloc_guard::Result<i32> {
    // 1. Load configuration (for exclude patterns)
    let config = load_config(args.config.as_deref(), cli.no_config)?;

    // 1.1 Load cache if not disabled
    let config_hash = compute_config_hash(&config);
    let cache = if args.no_cache {
        None
    } else {
        load_cache(&config_hash)
    };
    let cache = Mutex::new(cache.unwrap_or_else(|| Cache::new(config_hash)));

    // 2. Create GlobFilter
    let extensions = args
        .ext
        .clone()
        .unwrap_or_else(|| config.default.extensions.clone());
    let mut exclude_patterns = config.exclude.patterns.clone();
    exclude_patterns.extend(args.exclude.clone());
    let filter = GlobFilter::new(extensions, &exclude_patterns)?;

    // 3. Determine paths to scan
    let paths_to_scan = get_stats_scan_paths(args, &config);

    // 4. Scan directories
    let scanner = DirectoryScanner::new(filter);
    let mut all_files = Vec::new();
    for path in &paths_to_scan {
        let files = scanner.scan(path)?;
        all_files.extend(files);
    }

    // 5. Process each file and collect statistics (parallel with rayon)
    let registry = LanguageRegistry::default();

    let progress = ScanProgress::new(all_files.len() as u64, cli.quiet);
    let file_stats: Vec<_> = all_files
        .par_iter()
        .filter_map(|file_path| {
            let result = collect_file_stats_cached(file_path, &registry, &cache);
            progress.inc();
            result
        })
        .collect();
    progress.finish();

    // 5.1 Save cache if not disabled
    #[allow(clippy::collapsible_if)]
    if !args.no_cache {
        if let Ok(cache_guard) = cache.lock() {
            save_cache(&cache_guard);
        }
    }

    let project_stats = ProjectStatistics::new(file_stats);
    let project_stats = match args.group_by {
        GroupBy::Lang => project_stats.with_language_breakdown(),
        GroupBy::None => project_stats,
    };

    // 6. Format output
    let output = format_stats_output(args.format, &project_stats)?;

    // 7. Write output
    write_output(args.output.as_deref(), &output, cli.quiet)?;

    Ok(EXIT_SUCCESS)
}

fn get_stats_scan_paths(args: &StatsArgs, config: &Config) -> Vec<std::path::PathBuf> {
    // CLI --include overrides config include_paths
    if !args.include.is_empty() {
        return args.include.iter().map(std::path::PathBuf::from).collect();
    }

    // If CLI paths provided (other than default "."), use them
    let default_path = std::path::PathBuf::from(".");
    if args.paths.len() != 1 || args.paths[0] != default_path {
        return args.paths.clone();
    }

    // Use config include_paths if available
    if !config.default.include_paths.is_empty() {
        return config
            .default
            .include_paths
            .iter()
            .map(std::path::PathBuf::from)
            .collect();
    }

    // Default to current directory
    args.paths.clone()
}

#[allow(dead_code)]
fn collect_file_stats(file_path: &Path, registry: &LanguageRegistry) -> Option<FileStatistics> {
    let ext = file_path.extension()?.to_str()?;
    let language = registry.get_by_extension(ext)?;

    let counter = SlocCounter::new(&language.comment_syntax);
    let stats = count_file_lines(file_path, &counter)?;

    Some(FileStatistics {
        path: file_path.to_path_buf(),
        stats,
        language: language.name.to_string(),
    })
}

fn collect_file_stats_cached(
    file_path: &Path,
    registry: &LanguageRegistry,
    cache: &Mutex<Cache>,
) -> Option<FileStatistics> {
    let ext = file_path.extension()?.to_str()?;
    let language = registry.get_by_extension(ext)?;

    let path_key = file_path.to_string_lossy().replace('\\', "/");
    let file_hash = compute_file_hash(file_path).ok()?;

    // Try to get stats from cache
    let stats = {
        let cache_guard = cache.lock().ok()?;
        cache_guard
            .get_if_valid(&path_key, &file_hash)
            .map(|entry| LineStats::from(&entry.stats))
    };

    let stats = if let Some(cached_stats) = stats {
        cached_stats
    } else {
        // Cache miss: count lines and update cache
        let counter = SlocCounter::new(&language.comment_syntax);
        let stats = count_file_lines(file_path, &counter)?;

        // Update cache
        if let Ok(mut cache_guard) = cache.lock() {
            cache_guard.set(&path_key, file_hash, &stats);
        }

        stats
    };

    Some(FileStatistics {
        path: file_path.to_path_buf(),
        stats,
        language: language.name.to_string(),
    })
}

fn format_stats_output(
    format: OutputFormat,
    stats: &ProjectStatistics,
) -> sloc_guard::Result<String> {
    match format {
        OutputFormat::Text => StatsTextFormatter.format(stats),
        OutputFormat::Json => StatsJsonFormatter.format(stats),
        OutputFormat::Sarif => Err(sloc_guard::SlocGuardError::Config(
            "SARIF output format is not supported for stats command".to_string(),
        )),
        OutputFormat::Markdown => Err(sloc_guard::SlocGuardError::Config(
            "Markdown output format is not yet implemented".to_string(),
        )),
    }
}

fn run_baseline(args: &BaselineArgs, cli: &Cli) -> i32 {
    match &args.action {
        BaselineAction::Update(update_args) => run_baseline_update(update_args, cli),
    }
}

fn run_baseline_update(args: &BaselineUpdateArgs, cli: &Cli) -> i32 {
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

fn run_baseline_update_impl(args: &BaselineUpdateArgs, cli: &Cli) -> sloc_guard::Result<usize> {
    // 1. Load configuration
    let config = load_config(args.config.as_deref(), cli.no_config)?;

    // 2. Create GlobFilter
    let extensions = args
        .ext
        .clone()
        .unwrap_or_else(|| config.default.extensions.clone());
    let mut exclude_patterns = config.exclude.patterns.clone();
    exclude_patterns.extend(args.exclude.clone());
    let filter = GlobFilter::new(extensions, &exclude_patterns)?;

    // 3. Determine paths to scan
    let paths_to_scan = get_baseline_scan_paths(args, &config);

    // 4. Scan directories
    let scanner = DirectoryScanner::new(filter);
    let mut all_files = Vec::new();
    for path in &paths_to_scan {
        let files = scanner.scan(path)?;
        all_files.extend(files);
    }

    // 5. Process each file and find violations
    let registry = LanguageRegistry::default();
    let warn_threshold = config.default.warn_threshold;
    let checker = ThresholdChecker::new(config.clone()).with_warning_threshold(warn_threshold);

    let skip_comments = config.default.skip_comments;
    let skip_blank = config.default.skip_blank;

    let progress = ScanProgress::new(all_files.len() as u64, cli.quiet);
    let violations: Vec<_> = all_files
        .par_iter()
        .filter_map(|file_path| {
            let result = process_file(file_path, &registry, &checker, skip_comments, skip_blank);
            progress.inc();
            let result = result?;
            if result.is_failed() {
                Some((file_path.clone(), result.stats.code))
            } else {
                None
            }
        })
        .collect();
    progress.finish();

    // 6. Create baseline from violations
    let mut baseline = Baseline::new();
    for (path, lines) in &violations {
        let path_str = path.to_string_lossy().replace('\\', "/");
        let hash = compute_file_hash(path)?;
        baseline.set(&path_str, *lines, hash);
    }

    // 7. Save baseline to file
    baseline.save(&args.output)?;

    Ok(violations.len())
}

fn get_baseline_scan_paths(
    args: &BaselineUpdateArgs,
    config: &Config,
) -> Vec<std::path::PathBuf> {
    // CLI --include overrides config include_paths
    if !args.include.is_empty() {
        return args.include.iter().map(std::path::PathBuf::from).collect();
    }

    // If CLI paths provided (other than default "."), use them
    let default_path = std::path::PathBuf::from(".");
    if args.paths.len() != 1 || args.paths[0] != default_path {
        return args.paths.clone();
    }

    // Use config include_paths if available
    if !config.default.include_paths.is_empty() {
        return config
            .default
            .include_paths
            .iter()
            .map(std::path::PathBuf::from)
            .collect();
    }

    // Default to current directory
    args.paths.clone()
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
