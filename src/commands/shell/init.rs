use crate::commands::not_implemented;
use std::process::ExitCode;

#[derive(clap::Args)]
#[command(about = crate::descriptions::subcommands::shell::subcommands::INIT)]
pub struct InitArgs {
    pub shell: IntegrationShell,
}

#[derive(Clone, clap::ValueEnum)]
pub enum IntegrationShell {
    Fish,
    Bash,
    Zsh,
}

pub(super) fn run(_args: InitArgs) -> ExitCode {
    not_implemented("shell init")
}
