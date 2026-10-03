mod cli;
mod commands;
mod config;
mod git;
mod github;
mod guard;
mod hooks;
mod output;
mod policy;
mod process;
mod settings;

use clap::Parser;

fn main() {
    if let Err(error) = commands::run(cli::Cli::parse()) {
        output::error(&format!("{error:#}"));
        std::process::exit(1);
    }
}
