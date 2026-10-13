mod add;
mod list;
mod remove;
mod update;

use clap::Subcommand;
use std::process::ExitCode;

#[derive(Subcommand)]
pub enum AllowedCommand {
    List(list::ListArgs),
    Add(add::AddArgs),
    Remove(remove::RemoveArgs),
    Update(update::UpdateArgs),
}

pub(super) fn run(command: Option<AllowedCommand>) -> ExitCode {
    match command.unwrap_or(AllowedCommand::List(list::ListArgs)) {
        AllowedCommand::List(args) => list::run(args),
        AllowedCommand::Add(args) => add::run(args),
        AllowedCommand::Remove(args) => remove::run(args),
        AllowedCommand::Update(args) => update::run(args),
    }
}
