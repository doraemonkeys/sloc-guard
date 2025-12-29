use std::path::PathBuf;

use super::*;

#[test]
fn cli_check_default_path() {
    let cli = Cli::parse_from(["sloc-guard", "check"]);
    match cli.command {
        Commands::Check(args) => {
            // paths is empty by default; validate_and_resolve_paths() applies the "." default
            assert!(args.paths.is_empty());
        }
        _ => panic!("Expected Check command"),
    }
}

#[test]
fn cli_check_with_paths() {
    let cli = Cli::parse_from(["sloc-guard", "check", "src", "tests"]);
    match cli.command {
        Commands::Check(args) => {
            assert_eq!(
                args.paths,
                vec![PathBuf::from("src"), PathBuf::from("tests")]
            );
        }
        _ => panic!("Expected Check command"),
    }
}

#[test]
fn cli_check_with_config() {
    let cli = Cli::parse_from(["sloc-guard", "check", "--config", "custom.toml"]);
    match cli.command {
        Commands::Check(args) => {
            assert_eq!(args.config, Some(PathBuf::from("custom.toml")));
        }
        _ => panic!("Expected Check command"),
    }
}

#[test]
fn cli_check_with_max_lines() {
    let cli = Cli::parse_from(["sloc-guard", "check", "--max-lines", "300"]);
    match cli.command {
        Commands::Check(args) => {
            assert_eq!(args.max_lines, Some(300));
        }
        _ => panic!("Expected Check command"),
    }
}

#[test]
fn cli_check_with_extensions() {
    let cli = Cli::parse_from(["sloc-guard", "check", "--ext", "rs,go,py"]);
    match cli.command {
        Commands::Check(args) => {
            assert_eq!(
                args.ext,
                Some(vec!["rs".to_string(), "go".to_string(), "py".to_string()])
            );
        }
        _ => panic!("Expected Check command"),
    }
}

#[test]
fn cli_check_with_format() {
    let cli = Cli::parse_from(["sloc-guard", "check", "--format", "json"]);
    match cli.command {
        Commands::Check(args) => {
            assert_eq!(args.format, OutputFormat::Json);
        }
        _ => panic!("Expected Check command"),
    }
}

#[test]
fn cli_check_warn_only() {
    let cli = Cli::parse_from(["sloc-guard", "check", "--warn-only"]);
    match cli.command {
        Commands::Check(args) => {
            assert!(args.warn_only);
        }
        _ => panic!("Expected Check command"),
    }
}

#[test]
fn cli_check_with_diff() {
    let cli = Cli::parse_from(["sloc-guard", "check", "--diff", "origin/main"]);
    match cli.command {
        Commands::Check(args) => {
            assert_eq!(args.diff, Some("origin/main".to_string()));
        }
        _ => panic!("Expected Check command"),
    }
}

#[test]
fn cli_check_with_diff_no_value() {
    // When --diff is provided without a value, it should default to "HEAD"
    let cli = Cli::parse_from(["sloc-guard", "check", "--diff"]);
    match cli.command {
        Commands::Check(args) => {
            assert_eq!(args.diff, Some("HEAD".to_string()));
        }
        _ => panic!("Expected Check command"),
    }
}

#[test]
fn cli_check_with_staged() {
    let cli = Cli::parse_from(["sloc-guard", "check", "--staged"]);
    match cli.command {
        Commands::Check(args) => {
            assert!(args.staged);
            assert!(args.diff.is_none());
        }
        _ => panic!("Expected Check command"),
    }
}

// ============================================================================
// Stats Subcommand Tests
// ============================================================================

#[test]
fn cli_stats_summary_command() {
    let cli = Cli::parse_from(["sloc-guard", "stats", "summary", "src"]);
    match cli.command {
        Commands::Stats(args) => match args.action {
            StatsAction::Summary(summary_args) => {
                assert_eq!(summary_args.common.paths, vec![PathBuf::from("src")]);
            }
            _ => panic!("Expected Summary action"),
        },
        _ => panic!("Expected Stats command"),
    }
}

#[test]
fn cli_stats_files_command() {
    let cli = Cli::parse_from([
        "sloc-guard",
        "stats",
        "files",
        "--top",
        "10",
        "--sort",
        "code",
    ]);
    match cli.command {
        Commands::Stats(args) => match args.action {
            StatsAction::Files(files_args) => {
                assert_eq!(files_args.top, Some(10));
                assert_eq!(files_args.sort, FileSortOrder::Code);
            }
            _ => panic!("Expected Files action"),
        },
        _ => panic!("Expected Stats command"),
    }
}

#[test]
fn cli_stats_files_sort_total() {
    let cli = Cli::parse_from(["sloc-guard", "stats", "files", "--sort", "total"]);
    match cli.command {
        Commands::Stats(args) => match args.action {
            StatsAction::Files(files_args) => {
                assert_eq!(files_args.sort, FileSortOrder::Total);
            }
            _ => panic!("Expected Files action"),
        },
        _ => panic!("Expected Stats command"),
    }
}

#[test]
fn cli_stats_files_sort_comment() {
    let cli = Cli::parse_from(["sloc-guard", "stats", "files", "--sort", "comment"]);
    match cli.command {
        Commands::Stats(args) => match args.action {
            StatsAction::Files(files_args) => {
                assert_eq!(files_args.sort, FileSortOrder::Comment);
            }
            _ => panic!("Expected Files action"),
        },
        _ => panic!("Expected Stats command"),
    }
}

#[test]
fn cli_stats_files_sort_blank() {
    let cli = Cli::parse_from(["sloc-guard", "stats", "files", "--sort", "blank"]);
    match cli.command {
        Commands::Stats(args) => match args.action {
            StatsAction::Files(files_args) => {
                assert_eq!(files_args.sort, FileSortOrder::Blank);
            }
            _ => panic!("Expected Files action"),
        },
        _ => panic!("Expected Stats command"),
    }
}

#[test]
fn cli_stats_files_sort_name() {
    let cli = Cli::parse_from(["sloc-guard", "stats", "files", "--sort", "name"]);
    match cli.command {
        Commands::Stats(args) => match args.action {
            StatsAction::Files(files_args) => {
                assert_eq!(files_args.sort, FileSortOrder::Name);
            }
            _ => panic!("Expected Files action"),
        },
        _ => panic!("Expected Stats command"),
    }
}

#[test]
fn cli_stats_files_default_sort() {
    // Default sort order should be code
    let cli = Cli::parse_from(["sloc-guard", "stats", "files"]);
    match cli.command {
        Commands::Stats(args) => match args.action {
            StatsAction::Files(files_args) => {
                assert_eq!(files_args.sort, FileSortOrder::Code);
                assert!(files_args.top.is_none()); // No top limit by default
            }
            _ => panic!("Expected Files action"),
        },
        _ => panic!("Expected Stats command"),
    }
}

#[test]
fn cli_stats_files_format_json() {
    let cli = Cli::parse_from(["sloc-guard", "stats", "files", "--format", "json"]);
    match cli.command {
        Commands::Stats(args) => match args.action {
            StatsAction::Files(files_args) => {
                assert_eq!(files_args.format, StatsOutputFormat::Json);
            }
            _ => panic!("Expected Files action"),
        },
        _ => panic!("Expected Stats command"),
    }
}

#[test]
fn cli_stats_files_format_markdown() {
    let cli = Cli::parse_from(["sloc-guard", "stats", "files", "--format", "md"]);
    match cli.command {
        Commands::Stats(args) => match args.action {
            StatsAction::Files(files_args) => {
                assert_eq!(files_args.format, StatsOutputFormat::Markdown);
            }
            _ => panic!("Expected Files action"),
        },
        _ => panic!("Expected Stats command"),
    }
}

#[test]
fn cli_stats_breakdown_command() {
    let cli = Cli::parse_from(["sloc-guard", "stats", "breakdown", "--by", "dir"]);
    match cli.command {
        Commands::Stats(args) => match args.action {
            StatsAction::Breakdown(breakdown_args) => {
                assert_eq!(breakdown_args.by, BreakdownBy::Dir);
            }
            _ => panic!("Expected Breakdown action"),
        },
        _ => panic!("Expected Stats command"),
    }
}

#[test]
fn cli_stats_trend_command() {
    let cli = Cli::parse_from(["sloc-guard", "stats", "trend", "--since", "7d"]);
    match cli.command {
        Commands::Stats(args) => match args.action {
            StatsAction::Trend(trend_args) => {
                assert_eq!(trend_args.since, Some("7d".to_string()));
            }
            _ => panic!("Expected Trend action"),
        },
        _ => panic!("Expected Stats command"),
    }
}

#[test]
fn cli_stats_history_command() {
    let cli = Cli::parse_from(["sloc-guard", "stats", "history", "--limit", "5"]);
    match cli.command {
        Commands::Stats(args) => match args.action {
            StatsAction::History(history_args) => {
                assert_eq!(history_args.limit, 5);
            }
            _ => panic!("Expected History action"),
        },
        _ => panic!("Expected Stats command"),
    }
}

#[test]
fn cli_stats_report_command() {
    let cli = Cli::parse_from([
        "sloc-guard",
        "stats",
        "report",
        "--format",
        "html",
        "-o",
        "report.html",
    ]);
    match cli.command {
        Commands::Stats(args) => match args.action {
            StatsAction::Report(report_args) => {
                assert_eq!(report_args.format, ReportOutputFormat::Html);
                assert_eq!(report_args.output, Some(PathBuf::from("report.html")));
            }
            _ => panic!("Expected Report action"),
        },
        _ => panic!("Expected Stats command"),
    }
}

#[test]
fn cli_stats_report_with_config_flags() {
    use crate::cli::BreakdownBy;

    let cli = Cli::parse_from([
        "sloc-guard",
        "stats",
        "report",
        "--exclude-section",
        "trend",
        "--exclude-section",
        "files",
        "--top",
        "20",
        "--breakdown-by",
        "dir",
        "--since",
        "7d",
    ]);
    match cli.command {
        Commands::Stats(args) => match args.action {
            StatsAction::Report(report_args) => {
                assert_eq!(
                    report_args.exclude_sections,
                    vec!["trend".to_string(), "files".to_string()]
                );
                assert_eq!(report_args.top, Some(20));
                assert_eq!(report_args.breakdown_by, Some(BreakdownBy::Dir));
                assert_eq!(report_args.since, Some("7d".to_string()));
            }
            _ => panic!("Expected Report action"),
        },
        _ => panic!("Expected Stats command"),
    }
}

#[test]
fn cli_stats_common_args() {
    let cli = Cli::parse_from([
        "sloc-guard",
        "stats",
        "summary",
        "--config",
        "custom.toml",
        "--ext",
        "rs,go",
        "-x",
        "**/target/**",
        "-I",
        "src",
        "--no-cache",
        "--no-gitignore",
    ]);
    match cli.command {
        Commands::Stats(args) => match args.action {
            StatsAction::Summary(summary_args) => {
                assert_eq!(
                    summary_args.common.config,
                    Some(PathBuf::from("custom.toml"))
                );
                assert_eq!(
                    summary_args.common.ext,
                    Some(vec!["rs".to_string(), "go".to_string()])
                );
                assert_eq!(summary_args.common.exclude, vec!["**/target/**"]);
                assert_eq!(summary_args.common.include, vec!["src"]);
                assert!(summary_args.common.no_cache);
                assert!(summary_args.common.no_gitignore);
            }
            _ => panic!("Expected Summary action"),
        },
        _ => panic!("Expected Stats command"),
    }
}

// ============================================================================
// Init Command Tests
// ============================================================================

#[test]
fn cli_init_command() {
    let cli = Cli::parse_from(["sloc-guard", "init"]);
    match cli.command {
        Commands::Init(args) => {
            assert_eq!(args.output, PathBuf::from(".sloc-guard.toml"));
            assert!(!args.force);
        }
        _ => panic!("Expected Init command"),
    }
}

#[test]
fn cli_init_with_output() {
    let cli = Cli::parse_from(["sloc-guard", "init", "--output", "config.toml"]);
    match cli.command {
        Commands::Init(args) => {
            assert_eq!(args.output, PathBuf::from("config.toml"));
        }
        _ => panic!("Expected Init command"),
    }
}

#[test]
fn cli_init_with_force() {
    let cli = Cli::parse_from(["sloc-guard", "init", "--force"]);
    match cli.command {
        Commands::Init(args) => {
            assert!(args.force);
        }
        _ => panic!("Expected Init command"),
    }
}

// ============================================================================
// Global Flags Tests
// ============================================================================

#[test]
fn cli_global_verbose() {
    let cli = Cli::parse_from(["sloc-guard", "-v", "check"]);
    assert_eq!(cli.verbose, 1);

    let cli = Cli::parse_from(["sloc-guard", "-vv", "check"]);
    assert_eq!(cli.verbose, 2);
}

#[test]
fn cli_global_quiet() {
    let cli = Cli::parse_from(["sloc-guard", "--quiet", "check"]);
    assert!(cli.quiet);
}

#[test]
fn cli_global_color() {
    let cli = Cli::parse_from(["sloc-guard", "--color", "never", "check"]);
    assert!(matches!(cli.color, ColorChoice::Never));

    let cli = Cli::parse_from(["sloc-guard", "--color", "always", "check"]);
    assert!(matches!(cli.color, ColorChoice::Always));
}

#[test]
fn cli_global_no_config() {
    let cli = Cli::parse_from(["sloc-guard", "--no-config", "check"]);
    assert!(cli.no_config);
}

#[test]
fn cli_check_with_exclude() {
    let cli = Cli::parse_from([
        "sloc-guard",
        "check",
        "-x",
        "**/target/**",
        "-x",
        "**/node_modules/**",
    ]);
    match cli.command {
        Commands::Check(args) => {
            assert_eq!(args.exclude, vec!["**/target/**", "**/node_modules/**"]);
        }
        _ => panic!("Expected Check command"),
    }
}

#[test]
fn cli_check_with_include() {
    let cli = Cli::parse_from(["sloc-guard", "check", "-I", "src", "-I", "lib"]);
    match cli.command {
        Commands::Check(args) => {
            assert_eq!(args.include, vec!["src", "lib"]);
        }
        _ => panic!("Expected Check command"),
    }
}

#[test]
fn cli_check_count_comments() {
    let cli = Cli::parse_from(["sloc-guard", "check", "--count-comments"]);
    match cli.command {
        Commands::Check(args) => {
            assert!(args.count_comments);
        }
        _ => panic!("Expected Check command"),
    }
}

#[test]
fn cli_check_count_blank() {
    let cli = Cli::parse_from(["sloc-guard", "check", "--count-blank"]);
    match cli.command {
        Commands::Check(args) => {
            assert!(args.count_blank);
        }
        _ => panic!("Expected Check command"),
    }
}

#[test]
fn cli_check_warn_threshold() {
    let cli = Cli::parse_from(["sloc-guard", "check", "--warn-threshold", "0.8"]);
    match cli.command {
        Commands::Check(args) => {
            assert_eq!(args.warn_threshold, Some(0.8));
        }
        _ => panic!("Expected Check command"),
    }
}

#[test]
fn cli_check_output_file() {
    let cli = Cli::parse_from(["sloc-guard", "check", "--output", "report.json"]);
    match cli.command {
        Commands::Check(args) => {
            assert_eq!(args.output, Some(PathBuf::from("report.json")));
        }
        _ => panic!("Expected Check command"),
    }
}

#[test]
fn cli_config_validate() {
    let cli = Cli::parse_from(["sloc-guard", "config", "validate"]);
    match cli.command {
        Commands::Config(args) => match args.action {
            ConfigAction::Validate { config } => {
                assert_eq!(config, PathBuf::from(".sloc-guard.toml"));
            }
            ConfigAction::Show { .. } => panic!("Expected Validate action"),
        },
        _ => panic!("Expected Config command"),
    }
}

#[test]
fn cli_config_show() {
    let cli = Cli::parse_from(["sloc-guard", "config", "show", "--format", "json"]);
    match cli.command {
        Commands::Config(args) => match args.action {
            ConfigAction::Show { config, format } => {
                assert!(config.is_none());
                assert_eq!(format, ConfigOutputFormat::Json);
            }
            ConfigAction::Validate { .. } => panic!("Expected Show action"),
        },
        _ => panic!("Expected Config command"),
    }
}

#[test]
fn cli_check_with_diff_range_syntax() {
    // Explicit range: base..target
    let cli = Cli::parse_from(["sloc-guard", "check", "--diff", "main..feature"]);
    match cli.command {
        Commands::Check(args) => {
            assert_eq!(args.diff, Some("main..feature".to_string()));
        }
        _ => panic!("Expected Check command"),
    }
}

#[test]
fn cli_check_with_diff_range_tags() {
    // Tag range: v1.0..v2.0
    let cli = Cli::parse_from(["sloc-guard", "check", "--diff", "v1.0..v2.0"]);
    match cli.command {
        Commands::Check(args) => {
            assert_eq!(args.diff, Some("v1.0..v2.0".to_string()));
        }
        _ => panic!("Expected Check command"),
    }
}

#[test]
fn cli_check_with_diff_range_origin() {
    // Origin refs: origin/main..origin/feature
    let cli = Cli::parse_from([
        "sloc-guard",
        "check",
        "--diff",
        "origin/main..origin/feature",
    ]);
    match cli.command {
        Commands::Check(args) => {
            assert_eq!(args.diff, Some("origin/main..origin/feature".to_string()));
        }
        _ => panic!("Expected Check command"),
    }
}

#[test]
fn cli_check_with_diff_trailing_dots() {
    // Trailing dots: main.. (should be parsed as main..HEAD by the check command)
    let cli = Cli::parse_from(["sloc-guard", "check", "--diff", "main.."]);
    match cli.command {
        Commands::Check(args) => {
            assert_eq!(args.diff, Some("main..".to_string()));
        }
        _ => panic!("Expected Check command"),
    }
}
