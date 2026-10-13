use crate::commands::not_implemented;
use std::process::ExitCode;

#[derive(clap::Args)]
#[command(about = crate::descriptions::subcommands::repo::subcommands::allowed::subcommands::ADD)]
pub struct AddArgs {
    pub identity: String,
}

pub(super) fn run(_args: AddArgs) -> ExitCode {
    not_implemented("repo allowed add")
}
