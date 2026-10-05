mod doctor;
pub mod enter;
mod identity;
mod repo;
mod settings;
mod setup;
mod shell;
mod status;
mod switch;

use crate::{
    cli::{Cli, Command, InternalCommand},
    config::{self, Config},
    hooks,
};
use anyhow::Result;

pub fn run(cli: Cli) -> Result<()> {
    if let Command::Shell { command } = cli.command {
        return shell::run(command);
    }

    if let Command::Internal {
        command: InternalCommand::Git { args },
    } = cli.command
    {
        return shell::git::run(&args, cli.config);
    }

    let path = config::path(cli.config)?;

    match cli.command {
        Command::Setup => setup::run(path),
        Command::Status { full } => status::run(path, full),
        Command::Switch { identity } => switch::run(&Config::load(path)?, &identity),
        Command::Doctor => doctor::run(path),
        Command::Identity { command } => identity::run(path, command),
        Command::Repo { command } => repo::run(path, command),
        Command::Settings { command } => settings::run(path, command),
        Command::Internal {
            command: InternalCommand::Enter,
        } => enter::run(path),
        Command::Internal {
            command: InternalCommand::Welcome,
        } => settings::welcome::run(path),
        Command::Internal {
            command: InternalCommand::Hook { name, args },
        } => hooks::run(path, &name, &args),
        Command::Shell { .. }
        | Command::Internal {
            command: InternalCommand::Git { .. },
        } => unreachable!(),
    }
}
