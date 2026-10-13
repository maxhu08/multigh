use crate::commands::not_implemented;
use crate::descriptions::subcommands::status as descriptions;
use std::process::ExitCode;

#[derive(clap::Args)]
#[command(about = descriptions::SELF)]
pub struct StatusArgs {
    #[arg(long, help = descriptions::flags::FULL)]
    pub full: bool,
}

pub(super) fn run(_args: StatusArgs) -> ExitCode {
    not_implemented("status")
}
