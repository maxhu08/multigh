use crate::commands::not_implemented;
use std::process::ExitCode;

#[derive(clap::Args)]
#[command(about = crate::descriptions::subcommands::settings::subcommands::VERBOSE)]
pub struct VerboseArgs {
    pub state: Option<crate::commands::Toggle>,
}

pub(super) fn run(_args: VerboseArgs) -> ExitCode {
    not_implemented("settings verbose")
}
