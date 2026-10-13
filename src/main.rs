mod cli;
mod commands;
#[rustfmt::skip]
mod descriptions;
mod utils;

use clap::Parser;
use std::process::ExitCode;

fn main() -> ExitCode {
    commands::run(cli::Cli::parse().command)
}
