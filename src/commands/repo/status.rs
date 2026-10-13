use crate::commands::not_implemented;
use std::process::ExitCode;

#[derive(clap::Args)]
#[command(about = crate::descriptions::subcommands::repo::subcommands::STATUS)]
pub struct StatusArgs;

pub(super) fn run(_args: StatusArgs) -> ExitCode {
    not_implemented("repo status")
}
