use clap::Parser;

use sloc_guard::EXIT_SUCCESS;
use sloc_guard::cli::{Cli, Commands, ConfigAction};

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

fn run_check(_args: &sloc_guard::cli::CheckArgs, _cli: &Cli) -> i32 {
    // TODO: Implement check command
    // _cli provides global options: verbose, quiet, color, no_config
    println!("Check command not yet implemented");
    EXIT_SUCCESS
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
