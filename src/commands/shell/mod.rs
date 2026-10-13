mod completions;
mod init;

use clap::Subcommand;
use std::process::ExitCode;

#[derive(Subcommand)]
pub enum ShellCommand {
    Init(init::InitArgs),
    Completions(completions::CompletionsArgs),
}

pub(super) fn run(command: ShellCommand) -> ExitCode {
    match command {
        ShellCommand::Init(args) => init::run(args),
        ShellCommand::Completions(args) => completions::run(args),
    }
}
