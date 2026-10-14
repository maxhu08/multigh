mod allowed;
pub(super) mod selection;

use crate::{
    cli::{RepoCommand, Toggle},
    config::Config,
    git, github, guard, hooks, policy,
    repository::Repository,
    settings,
    utils::output,
};
use anyhow::Result;
use std::path::PathBuf;

pub fn run(path: PathBuf, command: Option<RepoCommand>) -> Result<()> {
    let repository = Repository::require()?;

    match command.unwrap_or(RepoCommand::Status) {
        RepoCommand::Status => super::status::repository(
            &repository,
            &Config::load(path)?,
            Some(&github::active()?),
            None,
        ),
        RepoCommand::Check => guard::check(&repository, &Config::load(path)?),
        RepoCommand::Allowed { command } => {
            allowed::run(&repository, &Config::load(path)?, command)
        }
        RepoCommand::Protections { state } => {
            if let Some(state) = state {
                policy::migrate(&repository)?;
                let enabled = matches!(state, Toggle::On);
                if enabled {
                    super::global::update("mgh repo protections on", |global, _| {
                        hooks::install::run(&Config::load(path.clone())?, Some(&repository), global)
                    })?;
                }
                repository.set(
                    git::Scope::Local,
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
    output::repository_heading(
        "Repository",
        &repository.run(&["rev-parse", "--absolute-git-dir"])?,
    );
    output::path("Repo config", &repository.config_path());
    output::preference(
        "Protections",
        enabled,
        "Block commits and pushes from identities not allowed in this repository.",
    );
    Ok(())
}
