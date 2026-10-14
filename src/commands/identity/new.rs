use crate::{
    cli::IdentityFields,
    config::{
        CommitDetails, Config,
        editor::{Editor, IdentityInput},
    },
    github,
    utils::{
        output::{self, IdentityField},
        process, terminal,
    },
};
use anyhow::{Result, ensure};
use std::path::PathBuf;

pub fn run(path: PathBuf, identity_name: Option<String>, fields: IdentityFields) -> Result<()> {
    let (config, identity_name) = create(path, identity_name, fields)?;
    report(&config, &identity_name);
    super::super::setup::configure(&config, Some(&identity_name))
}

pub(in crate::commands) fn create(
    path: PathBuf,
    identity_name: Option<String>,
    fields: IdentityFields,
) -> Result<(Config, String)> {
    let editor = Editor::open(path)?;

    if terminal::interactive() {
        output::form_intro("Add new identity")?;
    } else {
        output::section("Add new identity");
    }

    let identity_name =
        super::form::field(identity_name, IdentityField::Name, None)?.to_ascii_lowercase();
    let username = super::form::field(fields.username, IdentityField::Username, None)?;
    let commit_name = super::form::field(fields.name, IdentityField::CommitName, Some(&username))?;
    let commit_email = super::form::field(fields.email, IdentityField::Email, None)?;

    if terminal::interactive() {
        output::form_outro("Identity details entered")?;
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

    let input = IdentityInput {
        username,
        commit: CommitDetails {
            name: commit_name,
            email: commit_email,
        },
    };
    let pending = editor.add(&identity_name, &input)?;
    github::login(&pending.identity(&identity_name)?.username)?;
    let config =
        pending.save("Configuration changed while signing in; run mgh identity new again")?;

    Ok((config, identity_name))
}

pub(in crate::commands) fn report(config: &Config, identity_name: &str) {
    output::identity_heading("✓ Identity added", identity_name);
    output::path("Accounts", &config.path);
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
