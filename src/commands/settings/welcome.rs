use crate::commands::not_implemented;
use std::process::ExitCode;

#[derive(clap::Args)]
#[command(about = crate::descriptions::subcommands::settings::subcommands::WELCOME)]
pub struct WelcomeArgs {
    pub state: Option<crate::commands::Toggle>,
}

pub(super) fn run(_args: WelcomeArgs) -> ExitCode {
    not_implemented("settings welcome")
}
