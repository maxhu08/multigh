use crate::{config::Config, guard, hooks, repository::Repository, settings};
use anyhow::{Context, Result, ensure};
use std::{io::Read, path::PathBuf};

pub fn run(path: PathBuf, name: &str, args: &[String]) -> Result<()> {
    ensure!(hooks::NAMES.contains(&name), "Unknown Git hook: {name}");

    let mut updates = Vec::new();

    if name == "pre-push" {
        std::io::stdin().read_to_end(&mut updates)?;
    }

    let repository = Repository::discover()?;
    let protections_enabled = repository
        .as_ref()
        .map(settings::protections)
        .transpose()?
        .unwrap_or(true);

    if protections_enabled {
        match name {
            "pre-commit" | "pre-merge-commit" => {
                let config = Config::load(path)?;
                if let Some(repository) = &repository {
                    guard::check(repository, &config)?;
                }
            }
            "pre-push" => {
                let repository = repository
                    .as_ref()
                    .context("A push hook requires a Git repository")?;
                guard::push(
                    repository,
                    &Config::load(path)?,
                    std::str::from_utf8(&updates)?,
                )?;
            }
            "post-checkout"
                if args.len() == 3 && args[0].chars().all(|c| c == '0') && args[2] == "1" =>
            {
                if let Some(repository) = &repository {
                    super::enter::enter(repository, path)?;
                }
            }
            _ => {}
        }
    }

    hooks::preserved::forward(
        repository.as_ref(),
        name,
        args,
        (name == "pre-push").then_some(updates.as_slice()),
    )
}
