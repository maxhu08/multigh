mod doctor;
mod identity;
mod internal;
mod onboard;
mod repo;
mod settings;
mod shell;
mod status;
mod switch;

use crate::descriptions::subcommands as descriptions;
use clap::{Subcommand, ValueEnum};
use identity::IdentityCommand;
use internal::InternalCommand;
use repo::RepoCommand;
use settings::SettingsCommand;
use shell::ShellCommand;
use std::process::ExitCode;

#[derive(Subcommand)]
pub enum Command {
    Onboard(onboard::OnboardArgs),
    Status(status::StatusArgs),
    Switch(switch::SwitchArgs),
    Doctor(doctor::DoctorArgs),
    #[command(about = descriptions::identity::SELF)]
    Identity {
        #[command(subcommand)]
        command: Option<IdentityCommand>,
    },
    #[command(about = descriptions::repo::SELF)]
    Repo {
        #[command(subcommand)]
        command: Option<RepoCommand>,
    },
    #[command(about = descriptions::settings::SELF)]
    Settings {
        #[command(subcommand)]
        command: Option<SettingsCommand>,
    },
    #[command(about = descriptions::shell::SELF)]
    Shell {
        #[command(subcommand)]
        command: ShellCommand,
    },
    #[command(hide = true)]
    Internal {
        #[command(subcommand)]
        command: InternalCommand,
    },
}

#[derive(Clone, ValueEnum)]
pub enum Toggle {
    On,
    Off,
}

pub fn run(command: Command) -> ExitCode {
    match command {
        Command::Onboard(args) => onboard::run(args),
        Command::Status(args) => status::run(args),
        Command::Switch(args) => switch::run(args),
        Command::Doctor(args) => doctor::run(args),
        Command::Identity { command } => identity::run(command),
        Command::Repo { command } => repo::run(command),
        Command::Settings { command } => settings::run(command),
        Command::Shell { command } => shell::run(command),
        Command::Internal { command } => internal::run(command),
    }
}

fn not_implemented(name: &str) -> ExitCode {
    crate::utils::print::error::print(format_args!("{name} is not implemented"));
    ExitCode::FAILURE
}
