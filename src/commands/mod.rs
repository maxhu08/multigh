mod doctor;
pub mod enter;
mod global;
mod hook;
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
};
use anyhow::Result;

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Setup => setup::run(config::path(cli.config)?),
        Command::Status { full } => status::run(config::path(cli.config)?, full),
        Command::Switch { identity } => {
            switch::run(&Config::load(config::path(cli.config)?)?, &identity)
        }
        Command::Doctor => doctor::run(config::path(cli.config)?),
        Command::Identity { command } => identity::run(config::path(cli.config)?, command),
        Command::Repo { command } => repo::run(config::path(cli.config)?, command),
        Command::Settings { command } => settings::run(config::path(cli.config)?, command),
        Command::Shell { command } => shell::run(command),
        Command::Internal { command } => match command {
            InternalCommand::Enter => enter::run(config::path(cli.config)?),
            InternalCommand::Welcome => settings::welcome::run(config::path(cli.config)?),
            InternalCommand::Hook { name, args } => {
                hook::run(config::path(cli.config)?, &name, &args)
            }
            InternalCommand::Git { args } => shell::git::run(&args, cli.config),
        },
    }
}
