use crate::commands::not_implemented;
use std::process::ExitCode;

#[derive(clap::Args)]
#[command(about = crate::descriptions::subcommands::repo::subcommands::allowed::subcommands::UPDATE)]
pub struct UpdateArgs;

pub(super) fn run(_args: UpdateArgs) -> ExitCode {
    not_implemented("repo allowed update")
}
