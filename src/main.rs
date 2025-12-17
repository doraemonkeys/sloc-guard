use clap::Parser;

use sloc_guard::cli::{Cli, Commands};
use sloc_guard::EXIT_SUCCESS;

fn main() {
    let cli = Cli::parse();

    let exit_code = match cli.command {
        Commands::Check(args) => run_check(args),
        Commands::Stats(args) => run_stats(args),
        Commands::Init(args) => run_init(args),
    };

    std::process::exit(exit_code);
}

fn run_check(_args: sloc_guard::cli::CheckArgs) -> i32 {
    // TODO: Implement check command
    println!("Check command not yet implemented");
    EXIT_SUCCESS
}

fn run_stats(_args: sloc_guard::cli::StatsArgs) -> i32 {
    // TODO: Implement stats command
    println!("Stats command not yet implemented");
    EXIT_SUCCESS
}

fn run_init(_args: sloc_guard::cli::InitArgs) -> i32 {
    // TODO: Implement init command
    println!("Init command not yet implemented");
    EXIT_SUCCESS
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
