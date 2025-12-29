use clap::Parser;

use sloc_guard::cli::{Cli, Commands};
use sloc_guard::commands::{run_check, run_config, run_explain, run_init, run_snapshot, run_stats};

fn main() {
    let cli = Cli::parse();

    let exit_code = match &cli.command {
        Commands::Check(args) => run_check(args, &cli),
        Commands::Stats(args) => run_stats(args, &cli),
        Commands::Snapshot(args) => run_snapshot(args, &cli),
        Commands::Init(args) => run_init(args),
        Commands::Config(args) => run_config(args, &cli),
        Commands::Explain(args) => run_explain(args, &cli),
    };

    std::process::exit(exit_code);
}
