use crate::repository::Repository;
use crate::{
    config::Config,
    policy,
    utils::{
        output::{self, Color},
        terminal,
    },
};
use anyhow::{Result, ensure};

pub fn choose(repository: &Repository, config: &Config) -> Result<()> {
    ensure!(
        terminal::interactive(),
        "Identity selection needs an interactive terminal.\nRun: mgh repo allowed update\nFor automation: mgh repo allowed add <identity>"
    );

    let current: Vec<_> = policy::stored(repository)?
        .into_iter()
        .filter(|name| config.identities.contains_key(name))
        .collect();
    let location = &repository.root;

    cliclack::intro(output::form_heading(
        "Repository",
        &location.to_string_lossy(),
    ))?;

    let mut prompt = cliclack::multiselect("Which identities may use this repository?")
        .initial_values(current)
        .max_rows(7);

    for (identity_name, identity) in &config.identities {
        prompt = prompt.item(identity_name.clone(), identity_name, &identity.username);
    }

    let selected = prompt.interact()?;

    policy::authorize(repository, config, &selected.into_iter().collect())?;
    cliclack::outro("Allowed identities saved")?;

    show(repository, config)
}

pub fn show(repository: &Repository, config: &Config) -> Result<()> {
    let allowed = policy::stored(repository)?;

    output::section("Allowed identities");

    if allowed.is_empty() {
        output::row("Repository", "No identities selected", Color::Warning);
    }

    for identity_name in allowed {
        match config.identity(&identity_name) {
            Ok(identity) => output::row(&identity_name, &identity.username, Color::Value),
            Err(_) => output::row(
                &identity_name,
                "Not configured; remove this permission or run mgh repo allowed update",
                Color::Warning,
            ),
        }
    }

    println!();

    Ok(())
}
