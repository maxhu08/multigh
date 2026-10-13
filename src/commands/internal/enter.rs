use crate::commands::not_implemented;
use std::process::ExitCode;

#[derive(clap::Args)]
#[command(about = crate::descriptions::subcommands::internal::subcommands::ENTER)]
pub struct EnterArgs;

pub(super) fn run(_args: EnterArgs) -> ExitCode {
    not_implemented("internal enter")
}
