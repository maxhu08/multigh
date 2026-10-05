mod allowed;
pub(super) mod selection;

use crate::{
    cli::{RepoCommand, Toggle},
    config::Config,
    git, github, guard, hooks,
    output::{self, Color},
    policy,
    repository::Repository,
    settings,
};
use anyhow::Result;
use std::path::PathBuf;

pub fn run(path: PathBuf, command: Option<RepoCommand>) -> Result<()> {
    let repository = Repository::require()?;

    match command.unwrap_or(RepoCommand::Status) {
        RepoCommand::Status => {
            super::status::repository(&repository, &Config::load(path)?, &github::active()?, None)
        }
        RepoCommand::Check => guard::check(&repository, &Config::load(path)?),
        RepoCommand::Allowed { command } => {
            allowed::run(&repository, &Config::load(path)?, command)
        }
        RepoCommand::Protections { state } => {
            if let Some(state) = state {
                policy::migrate(&repository)?;
                let enabled = matches!(state, Toggle::On);
                if enabled {
                    git::global::update("mgh repo protections on", |global| {
                        hooks::install::run(&Config::load(path.clone())?, Some(&repository), global)
                    })?;
                }
                repository.set(
                    "--local",
                    "mgh.protections",
                    if enabled { "true" } else { "false" },
                )?;
            }
            report(&repository)
        }
    }
}

pub fn report(repository: &Repository) -> Result<()> {
    let enabled = settings::protections(repository)?;
    output::heading(
        "Repository",
        &repository.run(&["rev-parse", "--absolute-git-dir"])?,
        Color::Heading,
    );
    output::row(
        "Repo config",
        &repository.config_path().to_string_lossy(),
        Color::Muted,
    );
    output::row(
        "Protections",
        if enabled { "ON" } else { "OFF" },
        if enabled {
            Color::Changed
        } else {
            Color::Muted
        },
    );
    println!("  Block commits and pushes from identities not allowed in this repository.\n");
    Ok(())
}
