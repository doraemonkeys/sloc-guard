use std::fs;
use std::path::Path;

use clap::Parser;

use sloc_guard::checker::{Checker, ThresholdChecker};
use sloc_guard::cli::{CheckArgs, Cli, Commands, ConfigAction};
use sloc_guard::config::{Config, ConfigLoader, FileConfigLoader};
use sloc_guard::counter::{LineStats, SlocCounter};
use sloc_guard::language::LanguageRegistry;
use sloc_guard::output::{JsonFormatter, OutputFormat, OutputFormatter, TextFormatter};
use sloc_guard::scanner::{DirectoryScanner, FileScanner, GlobFilter};
use sloc_guard::{EXIT_CONFIG_ERROR, EXIT_SUCCESS, EXIT_THRESHOLD_EXCEEDED};

fn main() {
    let cli = Cli::parse();

    let exit_code = match &cli.command {
        Commands::Check(args) => run_check(args, &cli),
        Commands::Stats(args) => run_stats(args, &cli),
        Commands::Init(args) => run_init(args),
        Commands::Config(args) => run_config(args),
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

    // 3. Create GlobFilter
    let extensions = args
        .ext
        .clone()
        .unwrap_or_else(|| config.default.extensions.clone());
    let mut exclude_patterns = config.exclude.patterns.clone();
    exclude_patterns.extend(args.exclude.clone());
    let filter = GlobFilter::new(extensions, &exclude_patterns)?;

    // 4. Determine paths to scan
    let paths_to_scan = get_scan_paths(args, &config);

    // 5. Scan directories
    let scanner = DirectoryScanner::new(filter);
    let mut all_files = Vec::new();
    for path in &paths_to_scan {
        let files = scanner.scan(path)?;
        all_files.extend(files);
    }

    // 6. Process each file
    let registry = LanguageRegistry::default();
    let warn_threshold = args
        .warn_threshold
        .unwrap_or(config.default.warn_threshold);
    let checker = ThresholdChecker::new(config.clone()).with_warning_threshold(warn_threshold);

    let skip_comments = config.default.skip_comments && !args.no_skip_comments;
    let skip_blank = config.default.skip_blank && !args.no_skip_blank;

    let mut results = Vec::new();
    for file_path in &all_files {
        if let Some(result) =
            process_file(file_path, &registry, &checker, skip_comments, skip_blank)
        {
            results.push(result);
        }
    }

    // 7. Format output
    let output = format_output(args.format, &results)?;

    // 8. Write output
    write_output(args.output.as_deref(), &output, cli.quiet)?;

    // 9. Determine exit code
    let has_failures = results
        .iter()
        .any(sloc_guard::checker::CheckResult::is_failed);
    if args.warn_only || !has_failures {
        Ok(EXIT_SUCCESS)
    } else {
        Ok(EXIT_THRESHOLD_EXCEEDED)
    }
}

fn load_config(config_path: Option<&Path>, no_config: bool) -> sloc_guard::Result<Config> {
    if no_config {
        return Ok(Config::default());
    }

    let loader = FileConfigLoader::new();
    config_path.map_or_else(|| loader.load(), |path| loader.load_from_path(path))
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

fn process_file(
    file_path: &Path,
    registry: &LanguageRegistry,
    checker: &ThresholdChecker,
    skip_comments: bool,
    skip_blank: bool,
) -> Option<sloc_guard::checker::CheckResult> {
    let ext = file_path.extension()?.to_str()?;
    let language = registry.get_by_extension(ext)?;

    let content = fs::read_to_string(file_path).ok()?;
    let counter = SlocCounter::new(&language.comment_syntax);
    let stats = counter.count(&content);

    // Compute effective stats based on skip_comments and skip_blank
    let effective_stats = compute_effective_stats(&stats, skip_comments, skip_blank);

    Some(checker.check(file_path, &effective_stats))
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
) -> sloc_guard::Result<String> {
    match format {
        OutputFormat::Text => TextFormatter.format(results),
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

fn run_stats(_args: &sloc_guard::cli::StatsArgs, _cli: &Cli) -> i32 {
    // TODO: Implement stats command
    println!("Stats command not yet implemented");
    EXIT_SUCCESS
}

fn run_init(_args: &sloc_guard::cli::InitArgs) -> i32 {
    // TODO: Implement init command
    println!("Init command not yet implemented");
    EXIT_SUCCESS
}

fn run_config(args: &sloc_guard::cli::ConfigArgs) -> i32 {
    match &args.action {
        ConfigAction::Validate { config } => {
            // TODO: Implement config validation
            println!("Validating config: {}", config.display());
            EXIT_SUCCESS
        }
        ConfigAction::Show { config, format } => {
            // TODO: Implement config show
            println!(
                "Showing config: {} (format: {})",
                config
                    .as_ref()
                    .map_or_else(|| "default".to_string(), |p| p.display().to_string()),
                format
            );
            EXIT_SUCCESS
        }
    }
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
