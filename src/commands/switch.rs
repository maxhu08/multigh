use crate::commands::not_implemented;
use crate::descriptions::subcommands::switch as descriptions;
use std::process::ExitCode;

#[derive(clap::Args)]
#[command(about = descriptions::SELF)]
pub struct SwitchArgs {
    #[arg(help = descriptions::args::IDENTITY)]
    pub identity: Option<String>,
}

pub(super) fn run(_args: SwitchArgs) -> ExitCode {
    not_implemented("switch")
}
