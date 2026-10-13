use crate::commands::not_implemented;
use std::process::ExitCode;

#[derive(clap::Args)]
#[command(about = crate::descriptions::subcommands::internal::subcommands::WELCOME)]
pub struct WelcomeArgs;

pub(super) fn run(_args: WelcomeArgs) -> ExitCode {
    not_implemented("internal welcome")
}
