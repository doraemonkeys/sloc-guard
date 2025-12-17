use std::path::PathBuf;

use super::*;

#[test]
fn cli_check_default_path() {
    let cli = Cli::parse_from(["sloc-guard", "check"]);
    match cli.command {
        Commands::Check(args) => {
            assert_eq!(args.paths, vec![PathBuf::from(".")]);
        }
        _ => panic!("Expected Check command"),
    }
}

#[test]
fn cli_check_with_paths() {
    let cli = Cli::parse_from(["sloc-guard", "check", "src", "tests"]);
    match cli.command {
        Commands::Check(args) => {
            assert_eq!(args.paths, vec![PathBuf::from("src"), PathBuf::from("tests")]);
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
