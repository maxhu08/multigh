use crate::commands::not_implemented;
use std::process::ExitCode;

#[derive(clap::Args)]
#[command(about = crate::descriptions::subcommands::shell::subcommands::COMPLETIONS)]
pub struct CompletionsArgs {
    pub shell: CompletionShell,
}

#[derive(Clone, clap::ValueEnum)]
pub enum CompletionShell {
    Bash,
    Elvish,
    Fish,
    #[value(name = "powershell")]
    PowerShell,
    Zsh,
}

pub(super) fn run(_args: CompletionsArgs) -> ExitCode {
    not_implemented("shell completions")
}
