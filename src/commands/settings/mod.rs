mod autoswitch;
mod verbose;
mod welcome;

use crate::commands::not_implemented;
use clap::Subcommand;
use std::process::ExitCode;

#[derive(Subcommand)]
pub enum SettingsCommand {
    Autoswitch(autoswitch::AutoswitchArgs),
    Verbose(verbose::VerboseArgs),
    Welcome(welcome::WelcomeArgs),
}

pub(super) fn run(command: Option<SettingsCommand>) -> ExitCode {
    match command {
        None => not_implemented("settings"),
        Some(SettingsCommand::Autoswitch(args)) => autoswitch::run(args),
        Some(SettingsCommand::Verbose(args)) => verbose::run(args),
        Some(SettingsCommand::Welcome(args)) => welcome::run(args),
    }
}
