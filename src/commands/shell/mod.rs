use crate::cli::{Cli, IntegrationShell, ShellCommand};
use anyhow::Result;
use clap::CommandFactory;

pub fn run(command: ShellCommand) -> Result<()> {
    match command {
        ShellCommand::Init { shell } => print!(
            "{}",
            match shell {
                IntegrationShell::Fish => include_str!("../../../shell/fish.fish"),
                IntegrationShell::Bash => include_str!("../../../shell/bash.bash"),
                IntegrationShell::Zsh => include_str!("../../../shell/zsh.zsh"),
            }
        ),
        ShellCommand::Completions { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "mgh", &mut std::io::stdout());
        }
    }
    Ok(())
}
pub mod git;
