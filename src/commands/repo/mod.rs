mod allowed;

use crate::{
    cli::{RepoCommand, Toggle},
    config::Config,
    git, github, guard, hooks,
    output::{self, Color},
    policy, settings,
};
use anyhow::{Result, ensure};
use std::path::PathBuf;

pub fn run(path: PathBuf, command: Option<RepoCommand>) -> Result<()> {
    ensure!(
        policy::repository()?,
        "No Git repository found here. Enter a repository before running mgh repo."
    );

    match command.unwrap_or(RepoCommand::Status) {
        RepoCommand::Status => {
            super::status::repository(&Config::load(path)?, &github::active()?, None)
        }
        RepoCommand::Check => guard::check(&Config::load(path)?),
        RepoCommand::Allowed { command } => allowed::run(&Config::load(path)?, command),
        RepoCommand::Protections { state } => {
            if let Some(state) = state {
                policy::migrate()?;
                let enabled = matches!(state, Toggle::On);
                if enabled {
                    git::update_global("mgh repo protections on", |global| {
                        hooks::install(&Config::load(path.clone())?, global)
                    })?;
                }
                git::set(
                    "--local",
                    "mgh.protections",
                    if enabled { "true" } else { "false" },
                )?;
            }
            report()
        }
    }
}

pub fn report() -> Result<()> {
    let enabled = settings::protections()?;
    output::heading(
        "Repository",
        &git::run(&["rev-parse", "--absolute-git-dir"])?,
        Color::Heading,
    );
    output::row(
        "Repo config",
        &git::local_path()?.to_string_lossy(),
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
