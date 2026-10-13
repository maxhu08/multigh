use crate::commands::not_implemented;
use std::process::ExitCode;

#[derive(clap::Args)]
#[command(about = crate::descriptions::subcommands::identity::subcommands::REMOVE)]
pub struct RemoveArgs {
    pub identity: String,
}

pub(super) fn run(_args: RemoveArgs) -> ExitCode {
    not_implemented("identity remove")
}
