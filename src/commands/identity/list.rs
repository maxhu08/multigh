use crate::commands::not_implemented;
use std::process::ExitCode;

#[derive(clap::Args)]
#[command(about = crate::descriptions::subcommands::identity::subcommands::LIST)]
pub struct ListArgs;

pub(super) fn run(_args: ListArgs) -> ExitCode {
    not_implemented("identity list")
}
