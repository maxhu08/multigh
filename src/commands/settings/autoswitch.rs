use crate::commands::not_implemented;
use std::process::ExitCode;

#[derive(clap::Args)]
#[command(about = crate::descriptions::subcommands::settings::subcommands::AUTOSWITCH)]
pub struct AutoswitchArgs {
    pub state: Option<crate::commands::Toggle>,
}

pub(super) fn run(_args: AutoswitchArgs) -> ExitCode {
    not_implemented("settings autoswitch")
}
