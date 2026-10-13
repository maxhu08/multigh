mod edit;
mod list;
mod new;
mod remove;

use crate::descriptions::subcommands::identity as descriptions;
use clap::{Args, Subcommand};
use std::process::ExitCode;

#[derive(Args)]
pub struct IdentityFields {
    #[arg(long, help = descriptions::options::USERNAME)]
    pub username: Option<String>,
    #[arg(long, help = descriptions::options::EMAIL)]
    pub email: Option<String>,
    #[arg(long, help = descriptions::options::NAME)]
    pub name: Option<String>,
}

#[derive(Subcommand)]
pub enum IdentityCommand {
    List(list::ListArgs),
    New(new::NewArgs),
    Edit(edit::EditArgs),
    Remove(remove::RemoveArgs),
}

pub(super) fn run(command: Option<IdentityCommand>) -> ExitCode {
    match command.unwrap_or(IdentityCommand::List(list::ListArgs)) {
        IdentityCommand::List(args) => list::run(args),
        IdentityCommand::New(args) => new::run(args),
        IdentityCommand::Edit(args) => edit::run(args),
        IdentityCommand::Remove(args) => remove::run(args),
    }
}
