use std::fs::{self, File};
use std::io::BufReader;
use std::path::Path;

use clap::Parser;
use rayon::prelude::*;

use sloc_guard::baseline::{compute_file_hash, Baseline};
use sloc_guard::checker::{Checker, ThresholdChecker};
use sloc_guard::cli::{
    BaselineAction, BaselineArgs, BaselineUpdateArgs, CheckArgs, Cli, ColorChoice, Commands,
    ConfigAction, StatsArgs,
};
use sloc_guard::config::{Config, ConfigLoader, FileConfigLoader};
use sloc_guard::counter::{CountResult, LineStats, SlocCounter};
use sloc_guard::git::{ChangedFiles, GitDiff};
use sloc_guard::language::LanguageRegistry;
use sloc_guard::output::{
    ColorMode, FileStatistics, JsonFormatter, OutputFormat, OutputFormatter, ProjectStatistics,
    StatsFormatter, StatsJsonFormatter, StatsTextFormatter, TextFormatter,
};
use sloc_guard::scanner::{DirectoryScanner, FileScanner, GlobFilter};
use sloc_guard::{EXIT_CONFIG_ERROR, EXIT_SUCCESS, EXIT_THRESHOLD_EXCEEDED};

/// File size threshold for streaming reads (10 MB)
const LARGE_FILE_THRESHOLD: u64 = 10 * 1024 * 1024;

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

    let mut results: Vec<_> = all_files
        .par_iter()
        .filter_map(|file_path| {
            process_file(file_path, &registry, &checker, skip_comments, skip_blank)
        })
        .collect();

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
        OutputFormat::Sarif => Err(sloc_guard::SlocGuardError::Config(
            "SARIF output format is not yet implemented".to_string(),
        )),
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

    let file_stats: Vec<_> = all_files
        .par_iter()
        .filter_map(|file_path| collect_file_stats(file_path, &registry))
        .collect();

    let project_stats = ProjectStatistics::new(file_stats);

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

fn collect_file_stats(file_path: &Path, registry: &LanguageRegistry) -> Option<FileStatistics> {
    let ext = file_path.extension()?.to_str()?;
    let language = registry.get_by_extension(ext)?;

    let counter = SlocCounter::new(&language.comment_syntax);
    let stats = count_file_lines(file_path, &counter)?;

    Some(FileStatistics {
        path: file_path.to_path_buf(),
        stats,
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
            "SARIF output format is not yet implemented".to_string(),
        )),
        OutputFormat::Markdown => Err(sloc_guard::SlocGuardError::Config(
            "Markdown output format is not yet implemented".to_string(),
        )),
    }
}

fn run_init(args: &sloc_guard::cli::InitArgs) -> i32 {
    match run_init_impl(args) {
        Ok(()) => EXIT_SUCCESS,
        Err(e) => {
            eprintln!("Error: {e}");
            EXIT_CONFIG_ERROR
        }
    }
}

fn run_init_impl(args: &sloc_guard::cli::InitArgs) -> sloc_guard::Result<()> {
    let output_path = &args.output;

    // Check if file already exists
    if output_path.exists() && !args.force {
        return Err(sloc_guard::SlocGuardError::Config(format!(
            "Configuration file already exists: {}. Use --force to overwrite.",
            output_path.display()
        )));
    }

    // Generate template config
    let template = generate_config_template();

    // Write to file
    fs::write(output_path, template)?;

    println!("Created configuration file: {}", output_path.display());
    Ok(())
}

fn generate_config_template() -> String {
    r#"# sloc-guard configuration file
# See: https://github.com/user/sloc-guard for documentation

[default]
# Maximum lines of code per file (default: 500)
max_lines = 500

# File extensions to check
extensions = ["rs", "go", "py", "js", "ts", "c", "cpp"]

# Directories to include (empty = scan from current directory)
# include_paths = ["src", "lib"]

# Skip comment lines when counting (default: true)
skip_comments = true

# Skip blank lines when counting (default: true)
skip_blank = true

# Warning threshold as ratio of max_lines (default: 0.9)
# Files exceeding this ratio but under max_lines will show warnings
warn_threshold = 0.9

# Strict mode: treat warnings as failures (default: false)
# strict = true

# Extension-based rules (override defaults for specific languages)
# [rules.rust]
# extensions = ["rs"]
# max_lines = 300

# [rules.python]
# extensions = ["py"]
# max_lines = 400

# Path-based rules (higher priority than extension rules)
# [[path_rules]]
# pattern = "src/generated/**"
# max_lines = 1000
# warn_threshold = 1.0  # Disable warnings for generated code

# Exclude patterns (glob syntax)
[exclude]
patterns = [
    "**/target/**",
    "**/node_modules/**",
    "**/.git/**",
    "**/vendor/**",
]

# Per-file overrides (highest priority)
# [[override]]
# path = "src/legacy/parser.rs"
# max_lines = 800
# reason = "Legacy code, scheduled for refactor"
"#
    .to_string()
}

fn run_config(args: &sloc_guard::cli::ConfigArgs) -> i32 {
    match &args.action {
        ConfigAction::Validate { config } => run_config_validate(config),
        ConfigAction::Show { config, format } => run_config_show(config.as_deref(), format),
    }
}

fn run_config_validate(config_path: &Path) -> i32 {
    match run_config_validate_impl(config_path) {
        Ok(()) => {
            println!("Configuration is valid: {}", config_path.display());
            EXIT_SUCCESS
        }
        Err(e) => {
            eprintln!("Configuration error: {e}");
            EXIT_CONFIG_ERROR
        }
    }
}

fn run_config_validate_impl(config_path: &Path) -> sloc_guard::Result<()> {
    // 1. Check if file exists
    if !config_path.exists() {
        return Err(sloc_guard::SlocGuardError::Config(format!(
            "Configuration file not found: {}",
            config_path.display()
        )));
    }

    // 2. Read and parse TOML
    let content = fs::read_to_string(config_path)?;
    let config: Config = toml::from_str(&content)?;

    // 3. Validate semantic correctness
    validate_config_semantics(&config)?;

    Ok(())
}

fn validate_config_semantics(config: &Config) -> sloc_guard::Result<()> {
    // Validate warn_threshold is in range [0.0, 1.0]
    if !(0.0..=1.0).contains(&config.default.warn_threshold) {
        return Err(sloc_guard::SlocGuardError::Config(format!(
            "warn_threshold must be between 0.0 and 1.0, got {}",
            config.default.warn_threshold
        )));
    }

    // Validate exclude patterns are valid globs
    for pattern in &config.exclude.patterns {
        globset::Glob::new(pattern).map_err(|e| sloc_guard::SlocGuardError::InvalidPattern {
            pattern: pattern.clone(),
            source: e,
        })?;
    }

    // Validate override paths are not empty
    for (i, override_cfg) in config.overrides.iter().enumerate() {
        if override_cfg.path.is_empty() {
            return Err(sloc_guard::SlocGuardError::Config(format!(
                "override[{i}].path cannot be empty"
            )));
        }
    }

    // Validate rules have either extensions or max_lines
    for (name, rule) in &config.rules {
        if rule.extensions.is_empty() && rule.max_lines.is_none() {
            return Err(sloc_guard::SlocGuardError::Config(format!(
                "rules.{name}: must specify at least extensions or max_lines"
            )));
        }
    }

    Ok(())
}

fn run_config_show(config_path: Option<&Path>, format: &str) -> i32 {
    match run_config_show_impl(config_path, format) {
        Ok(output) => {
            print!("{output}");
            EXIT_SUCCESS
        }
        Err(e) => {
            eprintln!("Error: {e}");
            EXIT_CONFIG_ERROR
        }
    }
}

fn run_config_show_impl(config_path: Option<&Path>, format: &str) -> sloc_guard::Result<String> {
    // Load configuration (from file or defaults)
    let config = load_config(config_path, false)?;

    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&config)?;
            Ok(format!("{json}\n"))
        }
        _ => Ok(format_config_text(&config)),
    }
}

fn format_config_text(config: &Config) -> String {
    use std::fmt::Write;

    let mut output = String::new();

    output.push_str("=== Effective Configuration ===\n\n");

    // Default section
    output.push_str("[default]\n");
    let _ = writeln!(output, "  max_lines = {}", config.default.max_lines);
    let _ = writeln!(output, "  extensions = {:?}", config.default.extensions);
    if !config.default.include_paths.is_empty() {
        let _ = writeln!(
            output,
            "  include_paths = {:?}",
            config.default.include_paths
        );
    }
    let _ = writeln!(output, "  skip_comments = {}", config.default.skip_comments);
    let _ = writeln!(output, "  skip_blank = {}", config.default.skip_blank);
    let _ = writeln!(
        output,
        "  warn_threshold = {}",
        config.default.warn_threshold
    );
    let _ = writeln!(output, "  strict = {}", config.default.strict);

    // Rules section
    if !config.rules.is_empty() {
        output.push('\n');
        let mut rule_names: Vec<_> = config.rules.keys().collect();
        rule_names.sort();
        for name in rule_names {
            let rule = &config.rules[name];
            let _ = writeln!(output, "[rules.{name}]");
            if !rule.extensions.is_empty() {
                let _ = writeln!(output, "  extensions = {:?}", rule.extensions);
            }
            if let Some(max_lines) = rule.max_lines {
                let _ = writeln!(output, "  max_lines = {max_lines}");
            }
            if let Some(skip_comments) = rule.skip_comments {
                let _ = writeln!(output, "  skip_comments = {skip_comments}");
            }
            if let Some(skip_blank) = rule.skip_blank {
                let _ = writeln!(output, "  skip_blank = {skip_blank}");
            }
        }
    }

    // Exclude section
    if !config.exclude.patterns.is_empty() {
        output.push_str("\n[exclude]\n");
        output.push_str("  patterns = [\n");
        for pattern in &config.exclude.patterns {
            let _ = writeln!(output, "    \"{pattern}\",");
        }
        output.push_str("  ]\n");
    }

    // Override section
    if !config.overrides.is_empty() {
        output.push('\n');
        for override_cfg in &config.overrides {
            output.push_str("[[override]]\n");
            let _ = writeln!(output, "  path = \"{}\"", override_cfg.path);
            let _ = writeln!(output, "  max_lines = {}", override_cfg.max_lines);
            if let Some(reason) = &override_cfg.reason {
                let _ = writeln!(output, "  reason = \"{reason}\"");
            }
        }
    }

    output
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

    let violations: Vec<_> = all_files
        .par_iter()
        .filter_map(|file_path| {
            let result = process_file(file_path, &registry, &checker, skip_comments, skip_blank)?;
            if result.is_failed() {
                Some((file_path.clone(), result.stats.code))
            } else {
                None
            }
        })
        .collect();

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
