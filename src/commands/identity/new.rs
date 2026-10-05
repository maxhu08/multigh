use crate::{
    config::{Config, editor::Editor},
    git, github,
    repository::Repository,
    utils::{output, process, terminal},
};
use anyhow::{Context, Result, ensure};
use std::path::PathBuf;

pub fn run(
    path: PathBuf,
    identity_name: Option<String>,
    username: Option<String>,
    commit_email: Option<String>,
    commit_name: Option<String>,
) -> Result<()> {
    let (config, identity_name) = create(path, identity_name, username, commit_email, commit_name)?;
    report(&config, &identity_name);
    let result: Result<()> = (|| {
        let repository = Repository::discover()?;
        git::global::update("mgh setup", |global| {
            let directory = super::super::setup::install(&config, repository.as_ref(), global)?;
            super::super::setup::report(&config, repository.as_ref(), &directory, global)
        })?;
        git::global::update(&format!("mgh switch {identity_name}"), |global| {
            let selected =
                super::super::switch::select(&config, &identity_name, repository.as_ref(), global)?;
            super::super::switch::report(&config, repository.as_ref(), &selected, true)
        })
    })();

    result.with_context(|| recovery_instructions(&config, &identity_name))
}

pub(in crate::commands) fn create(
    path: PathBuf,
    identity_name: Option<String>,
    username: Option<String>,
    commit_email: Option<String>,
    commit_name: Option<String>,
) -> Result<(Config, String)> {
    let editor = Editor::open(path)?;

    if terminal::interactive() {
        cliclack::intro("Add new identity")?;
    } else {
        output::section("Add new identity");
    }

    let identity_name =
        super::form::field(identity_name, "Identity name", None)?.to_ascii_lowercase();
    let username = super::form::field(username, "GitHub username", None)?;
    let commit_name = super::form::field(commit_name, "Commit name", Some(&username))?;
    let commit_email = super::form::field(commit_email, "Commit email", None)?;

    if terminal::interactive() {
        cliclack::outro("Identity details entered")?;
    }

    for value in [&identity_name, &username, &commit_name, &commit_email] {
        ensure!(
            !value.chars().any(char::is_control),
            "Identity fields must be single-line values"
        );
    }

    ensure!(
        editor
            .config()
            .is_none_or(|config| !config.identities.contains_key(&identity_name)),
        "Identity '{identity_name}' already exists"
    );

    let pending = editor.add(&identity_name, &username, &commit_name, &commit_email)?;
    github::login(&username)?;
    let config =
        pending.save("Configuration changed while signing in; run mgh identity new again")?;

    Ok((config, identity_name))
}

pub(in crate::commands) fn report(config: &Config, identity_name: &str) {
    output::heading("✓ Identity added", identity_name, output::Color::Changed);
    output::row(
        "Accounts",
        &config.path.to_string_lossy(),
        output::Color::Changed,
    );
}

pub(in crate::commands) fn recovery_instructions(config: &Config, identity_name: &str) -> String {
    let config_option = format!(
        "--config {}",
        process::quote(&config.path.to_string_lossy())
    );

    format!(
        "Identity saved in {}. After fixing the problem, run mgh {config_option} setup and mgh {config_option} switch {identity_name}",
        config.path.display(),
    )
}
