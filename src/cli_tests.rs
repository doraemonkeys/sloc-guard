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

#[test]
fn cli_stats_command() {
    let cli = Cli::parse_from(["sloc-guard", "stats", "src"]);
    match cli.command {
        Commands::Stats(args) => {
            assert_eq!(args.paths, vec![PathBuf::from("src")]);
        }
        _ => panic!("Expected Stats command"),
    }
}

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

// Tests for new CLI options

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
fn cli_stats_with_config() {
    let cli = Cli::parse_from(["sloc-guard", "stats", "--config", "custom.toml"]);
    match cli.command {
        Commands::Stats(args) => {
            assert_eq!(args.config, Some(PathBuf::from("custom.toml")));
        }
        _ => panic!("Expected Stats command"),
    }
}

#[test]
fn cli_stats_with_exclude() {
    let cli = Cli::parse_from(["sloc-guard", "stats", "-x", "**/vendor/**"]);
    match cli.command {
        Commands::Stats(args) => {
            assert_eq!(args.exclude, vec!["**/vendor/**"]);
        }
        _ => panic!("Expected Stats command"),
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
