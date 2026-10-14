mod cli;
mod commands;
mod config;
mod git;
mod github;
mod guard;
mod hooks;
mod policy;
mod repository;
mod settings;
mod utils;

use clap::Parser;
use utils::output;

fn main() {
    let cli = match cli::Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            output::cli_message(&error);
            std::process::exit(error.exit_code());
        }
    };
    let result = commands::run(cli);
    output::finish();
    if let Err(error) = result {
        output::error(&format!("{error:#}"));
        std::process::exit(1);
    }
}
