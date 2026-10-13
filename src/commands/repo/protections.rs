use crate::commands::not_implemented;
use std::process::ExitCode;

#[derive(clap::Args)]
#[command(about = crate::descriptions::subcommands::repo::subcommands::PROTECTIONS)]
pub struct ProtectionsArgs {
    pub state: Option<crate::commands::Toggle>,
}

pub(super) fn run(_args: ProtectionsArgs) -> ExitCode {
    not_implemented("repo protections")
}
