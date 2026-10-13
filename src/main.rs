mod cli;
mod commands;
#[rustfmt::skip]
mod descriptions;
mod utils;

use clap::Parser;
use std::{io, process::ExitCode};
use utils::print::padding;

fn main() -> ExitCode {
    match cli::Cli::try_parse() {
        Ok(cli) => commands::run(cli.command),
        Err(error) => {
            let result = if error.use_stderr() {
                padding::print(&mut io::stderr().lock(), |_| error.print())
            } else {
                padding::print(&mut io::stdout().lock(), |_| error.print())
            };

            if result.is_err() {
                return ExitCode::FAILURE;
            }

            ExitCode::from(error.exit_code() as u8)
        }
    }
}
