use crate::{
    cli::IdentityFields,
    config::{Config, JSONC},
    git, github, output, policy,
};
use anyhow::{Context, Result, ensure};
use jsonc_parser::{
    cst::{CstObject, CstRootNode},
    json,
};
use std::{fs, io::Write, path::PathBuf};
use tempfile::NamedTempFile;

fn set(object: &CstObject, key: &str, value: &str) {
    if let Some(property) = object.get(key) {
        property.set_value(json!(value));
    } else {
        object.append(key, json!(value));
    }
}

pub fn run(path: PathBuf, name: &str, fields: Option<IdentityFields>) -> Result<()> {
    ensure!(
        !fs::symlink_metadata(&path)?.file_type().is_symlink(),
        "Configuration file is a symlink; use --config with its target"
    );
    let original = fs::read_to_string(&path)?;
    let config = Config::load(path.clone())?;
    let identity = config.identity(name)?;
    let document = CstRootNode::parse(&original, &JSONC)?;
    let property = document
        .object_value()
        .context("Configuration must be an object")?
        .properties()
        .into_iter()
        .find(|property| {
            property
                .decoded_name()
                .is_some_and(|key| key.eq_ignore_ascii_case(name))
        })
        .context("Identity not found in configuration")?;
    let editing = fields.is_some();
    let mut username = None;

    if let Some(fields) = fields {
        let interactive =
            fields.username.is_none() && fields.email.is_none() && fields.name.is_none();
        ensure!(
            !interactive || policy::interactive(),
            "Editing needs a terminal or --username, --email or --name"
        );

        if interactive {
            cliclack::intro(output::form_heading(
                "Edit identity",
                &name.to_ascii_lowercase(),
            ))?;
        }
        let object = property
            .object_value()
            .context("Identity must be an object")?;
        let commit = object
            .object_value("commit")
            .context("Commit details must be an object")?;
        let new_username = if interactive {
            super::new::field(None, "GitHub username", Some(&identity.username))?
        } else {
            fields.username.unwrap_or_else(|| identity.username.clone())
        };
        let email = if interactive {
            super::new::field(None, "Commit email", Some(&identity.commit_email))?
        } else {
            fields
                .email
                .unwrap_or_else(|| identity.commit_email.clone())
        };
        let commit_name = if interactive {
            super::new::field(None, "Commit name", Some(&identity.commit_name))?
        } else {
            fields.name.unwrap_or_else(|| identity.commit_name.clone())
        };
        for value in [&new_username, &email, &commit_name] {
            ensure!(
                !value.chars().any(char::is_control),
                "Identity fields must be single-line values"
            );
        }

        set(&object, "username", &new_username);
        set(&commit, "name", &commit_name);
        set(&commit, "email", &email);
        username = Some(new_username);
        if interactive {
            cliclack::outro("Identity details entered")?;
        }
    } else {
        property.remove();
    }

    let mut pending = NamedTempFile::new_in(
        path.parent()
            .context("Configuration has no parent directory")?,
    )?;
    write!(pending, "{document}")?;
    let mut updated = Config::load(pending.path().to_owned())?;

    if let Some(username) = username {
        github::login(&username)?;
    }

    ensure!(
        fs::read_to_string(&path)? == original
            && !fs::symlink_metadata(&path)?.file_type().is_symlink(),
        "Configuration changed during editing; retry the command"
    );
    git::private(pending.path(), false)?;
    pending.persist(&path)?;
    updated.path = path;

    output::heading(
        &format!(
            "✓ Global identity {}",
            if editing { "updated" } else { "removed" }
        ),
        &name.to_ascii_lowercase(),
        output::Color::Changed,
    );
    output::row(
        "Accounts",
        &updated.path.to_string_lossy(),
        output::Color::Changed,
    );
    if !editing {
        output::warning(
            "Repository permissions are unchanged; affected repos stay blocked until updated with mgh repo allowed remove <identity> or update. GitHub login remains signed in.",
        );
    }

    git::update_global(
        &format!(
            "mgh identity {} {}",
            if editing { "edit" } else { "remove" },
            name.to_ascii_lowercase()
        ),
        |global| {
            git::setup_identities(&updated, global)?;
            Ok(())
        },
    )
}
