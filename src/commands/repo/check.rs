use crate::commands::not_implemented;
use std::process::ExitCode;

#[derive(clap::Args)]
#[command(about = crate::descriptions::subcommands::repo::subcommands::CHECK)]
pub struct CheckArgs;

pub(super) fn run(_args: CheckArgs) -> ExitCode {
    not_implemented("repo check")
}
