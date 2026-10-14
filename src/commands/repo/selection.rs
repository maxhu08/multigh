use crate::repository::Repository;
use crate::{
    config::Config,
    policy,
    utils::{output, terminal},
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

    output::form_intro(&output::form_repository(location))?;

    let mut prompt = cliclack::multiselect("Which identities may use this repository?")
        .initial_values(current)
        .max_rows(7);

    for (identity_name, identity) in &config.identities {
        prompt = prompt.item(
            identity_name.clone(),
            output::identity_name(identity_name, output::Stream::Stderr),
            output::username(&identity.username, output::Stream::Stderr),
        );
    }

    let selected = prompt.interact()?;

    policy::authorize(repository, config, &selected.into_iter().collect())?;
    output::form_outro("Allowed identities saved")?;

    show(repository, config)
}

pub fn show(repository: &Repository, config: &Config) -> Result<()> {
    let allowed = policy::stored(repository)?;

    output::section("Allowed identities");

    if allowed.is_empty() {
        output::warning_row("Repository", "No identities selected");
    }

    for identity_name in allowed {
        output::allowed_identity(
            &identity_name,
            config
                .identity(&identity_name)
                .ok()
                .map(|identity| identity.username.as_str()),
        );
    }

    Ok(())
}
