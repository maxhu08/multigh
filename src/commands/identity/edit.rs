use crate::{
    cli::IdentityFields,
    config::{
        CommitDetails, Config,
        editor::{Editor, IdentityInput, PendingUpdate},
    },
    git, github,
    utils::{output, terminal},
};
use anyhow::{Context, Result, ensure};
use std::path::PathBuf;

pub fn edit(path: PathBuf, identity_name: &str, fields: IdentityFields) -> Result<()> {
    let editor = Editor::open(path)?;
    let identity = editor
        .config()
        .context("Identity configuration unavailable")?
        .identity(identity_name)?;
    let interactive = fields.username.is_none() && fields.email.is_none() && fields.name.is_none();
    ensure!(
        !interactive || terminal::interactive(),
        "Editing needs a terminal or --username, --email or --name"
    );

    if interactive {
        cliclack::intro(output::form_heading(
            "Edit identity",
            &identity_name.to_ascii_lowercase(),
        ))?;
    }
    let new_username = if interactive {
        super::form::field(None, "GitHub username", Some(&identity.username))?
    } else {
        fields.username.unwrap_or_else(|| identity.username.clone())
    };
    let email = if interactive {
        super::form::field(None, "Commit email", Some(&identity.commit_email))?
    } else {
        fields
            .email
            .unwrap_or_else(|| identity.commit_email.clone())
    };
    let commit_name = if interactive {
        super::form::field(None, "Commit name", Some(&identity.commit_name))?
    } else {
        fields.name.unwrap_or_else(|| identity.commit_name.clone())
    };
    for value in [&new_username, &email, &commit_name] {
        ensure!(
            !value.chars().any(char::is_control),
            "Identity fields must be single-line values"
        );
    }

    let input = IdentityInput {
        username: new_username,
        commit: CommitDetails {
            name: commit_name,
            email,
        },
    };
    let pending = editor.edit(identity_name, &input)?;
    if interactive {
        cliclack::outro("Identity details entered")?;
    }
    github::login(&pending.identity(identity_name)?.username)?;
    let updated = save(pending, identity_name, "updated")?;
    refresh(
        &updated,
        &format!("mgh identity edit {}", identity_name.to_ascii_lowercase()),
    )
}

pub fn remove(path: PathBuf, identity_name: &str) -> Result<()> {
    let editor = Editor::open(path)?;
    editor
        .config()
        .context("Identity configuration unavailable")?
        .identity(identity_name)?;
    let updated = save(editor.remove(identity_name)?, identity_name, "removed")?;

    output::warning(
        "Repository permissions are unchanged; affected repos stay blocked until updated with mgh repo allowed remove <identity> or update. GitHub login remains signed in.",
    );
    refresh(
        &updated,
        &format!("mgh identity remove {}", identity_name.to_ascii_lowercase()),
    )
}

fn save(pending: PendingUpdate, identity_name: &str, description: &str) -> Result<Config> {
    let updated = pending.save("Configuration changed during editing; retry the command")?;

    output::heading(
        &format!("✓ Global identity {description}"),
        &identity_name.to_ascii_lowercase(),
        output::Color::Changed,
    );
    output::row(
        "Accounts",
        &updated.path.to_string_lossy(),
        output::Color::Changed,
    );

    Ok(updated)
}

fn refresh(config: &Config, command: &str) -> Result<()> {
    super::super::global::update(command, |global, _| {
        git::identity_files::refresh(config, global)?;
        Ok(())
    })
}
