use crate::commands::not_implemented;
use std::process::ExitCode;

#[derive(clap::Args)]
#[command(about = crate::descriptions::subcommands::ONBOARD)]
pub struct OnboardArgs;

pub(super) fn run(_args: OnboardArgs) -> ExitCode {
    not_implemented("onboard")
}
