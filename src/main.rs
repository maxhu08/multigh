mod cli;
mod commands;
mod descriptions;

use clap::Parser;
use std::process::ExitCode;

fn main() -> ExitCode {
    commands::run(cli::Cli::parse().command)
}
