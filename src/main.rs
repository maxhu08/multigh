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
    if let Err(error) = commands::run(cli::Cli::parse()) {
        output::error(&format!("{error:#}"));
        std::process::exit(1);
    }
}
