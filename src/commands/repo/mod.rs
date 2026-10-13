mod allowed;
mod check;
mod protections;
mod status;

use crate::descriptions::subcommands::repo as descriptions;
use clap::Subcommand;
use std::process::ExitCode;

#[derive(Subcommand)]
pub enum RepoCommand {
    Status(status::StatusArgs),
    Check(check::CheckArgs),
    Protections(protections::ProtectionsArgs),
    #[command(about = descriptions::subcommands::allowed::SELF)]
    Allowed {
        #[command(subcommand)]
        command: Option<allowed::AllowedCommand>,
    },
}

pub(super) fn run(command: Option<RepoCommand>) -> ExitCode {
    match command.unwrap_or(RepoCommand::Status(status::StatusArgs)) {
        RepoCommand::Status(args) => status::run(args),
        RepoCommand::Check(args) => check::run(args),
        RepoCommand::Protections(args) => protections::run(args),
        RepoCommand::Allowed { command } => allowed::run(command),
    }
}
