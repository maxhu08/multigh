use crate::{
    config::{CommitDetails, Config, Identity},
    git, github, output, policy,
    repository::Repository,
};
use anyhow::Result;

pub fn run(config: &Config, identity_name: &str) -> Result<()> {
    let repository = Repository::discover()?;
    let command = format!("mgh switch {identity_name}");

    git::global::update(&command, |global| {
        let selected = select(config, identity_name, repository.as_ref(), global)?;
        report(config, repository.as_ref(), &selected, true)
    })
}

pub(super) struct SwitchResult<'a> {
    identity_name: &'a str,
    identity: &'a Identity,
    previous_login: String,
    previous_global: CommitDetails,
    previous_effective: Option<CommitDetails>,
}

pub(super) fn select<'a>(
    config: &'a Config,
    identity_name: &'a str,
    repository: Option<&Repository>,
    global: &mut git::global::Writer<'_>,
) -> Result<SwitchResult<'a>> {
    let identity = config.identity(identity_name)?;
    let allowed = repository
        .map(|repository| policy::allowed(repository, config))
        .transpose()?
        .unwrap_or_default();
    let previous_effective = repository
        .map(|repository| -> Result<CommitDetails> {
            Ok(CommitDetails {
                name: repository.value("user.name")?.unwrap_or_default(),
                email: repository.value("user.email")?.unwrap_or_default(),
            })
        })
        .transpose()?;
    let previous_login = github::active().unwrap_or_default();
    let previous_global = CommitDetails {
        name: git::global::value("user.name")?,
        email: git::global::value("user.email")?,
    };

    git::identity_files::refresh(config, global)?;
    github::switch(identity)?;
    global.set("user.name", &identity.commit_name)?;
    global.set("user.email", &identity.commit_email)?;

    if let Some(repository) = repository
        && allowed.contains(&identity_name.to_ascii_lowercase())
    {
        policy::apply_identity(repository, identity_name, identity)?;
    }

    Ok(SwitchResult {
        identity_name,
        identity,
        previous_login,
        previous_global,
        previous_effective,
    })
}

pub(super) fn report(
    config: &Config,
    repository: Option<&Repository>,
    selected: &SwitchResult<'_>,
    detailed: bool,
) -> Result<()> {
    output::heading(
        "✓ Identity selected",
        &selected.identity_name.to_ascii_lowercase(),
        output::Color::Changed,
    );
    output::change(
        "GitHub account",
        &selected.previous_login,
        &selected.identity.username,
    );
    output::section("Global commit defaults");
    output::change(
        "Name",
        &selected.previous_global.name,
        &selected.identity.commit_name,
    );
    output::change(
        "Email",
        &selected.previous_global.email,
        &selected.identity.commit_email,
    );

    if detailed && let Some(repository) = repository {
        super::status::repository(
            repository,
            config,
            &selected.identity.username,
            selected.previous_effective.as_ref(),
        )?;
    }
    println!();

    Ok(())
}
