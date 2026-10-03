use crate::{
    cli::Toggle,
    config::Config,
    git, github,
    output::{self, Color},
    settings,
};
use anyhow::Result;
use std::{collections::BTreeSet, path::PathBuf};

pub fn run(path: PathBuf, state: Option<Toggle>) -> Result<()> {
    if let Some(state) = state {
        let enabled = matches!(state, Toggle::On);

        settings::set("welcome", enabled)?;
        output::section(if enabled {
            "✓ Identity welcome enabled"
        } else {
            "✓ Identity welcome disabled"
        });
        println!();
        return Ok(());
    }

    if !settings::enabled("welcome")? {
        return Ok(());
    }

    let selected = github::selected()?.unwrap_or_else(|| "unavailable".into());
    let values = git::entries(
        None,
        "^(user\\.(name|email)|mgh\\.(account|allowedaccount)|ghguard\\.account)$",
    )?;
    let value = |key: &str| {
        values
            .get(key)
            .and_then(|items| items.last())
            .map(String::as_str)
    };
    let config = Config::load(path);
    let selected_identity = config
        .as_ref()
        .ok()
        .and_then(|config| {
            config
                .identities
                .iter()
                .find(|(_, identity)| identity.username.eq_ignore_ascii_case(&selected))
        })
        .map(|(identity_name, _)| identity_name.as_str())
        .unwrap_or("not configured");

    output::section(&format!("Identity {selected_identity}"));
    output::row("GitHub username", &selected, Color::Value);
    output::row(
        "Commit email",
        value("user.email").unwrap_or("not configured"),
        Color::Value,
    );

    match config {
        Ok(config) => {
            let identity_names: BTreeSet<_> = values
                .get("mgh.allowedaccount")
                .cloned()
                .unwrap_or_else(|| {
                    value("mgh.account")
                        .or(value("ghguard.account"))
                        .map(|identity_name| vec![identity_name.to_owned()])
                        .unwrap_or_default()
                })
                .into_iter()
                .map(|identity_name| identity_name.to_ascii_lowercase())
                .collect();
            let mut allowed_identity = None;

            for identity_name in &identity_names {
                match config.identity(identity_name) {
                    Ok(identity) if identity.username.eq_ignore_ascii_case(&selected) => {
                        allowed_identity = Some(identity_name)
                    }
                    Err(error) => output::warning(&error.to_string()),
                    _ => {}
                }
            }

            if let Some(identity_name) = allowed_identity {
                let identity = config.identity(identity_name)?;

                if value("user.name") != Some(identity.commit_name.as_str())
                    || !identity
                        .allowed_emails
                        .contains(&value("user.email").unwrap_or_default().to_ascii_lowercase())
                {
                    output::warning(&format!("Commit details need: mgh switch {identity_name}"));
                }
            } else if !identity_names.is_empty() {
                output::warning(&format!(
                    "This repository needs: mgh switch {}",
                    identity_names
                        .iter()
                        .map(String::as_str)
                        .collect::<Vec<_>>()
                        .join(" or ")
                ));
            }
        }
        Err(error) => output::warning(&format!("{error:#}")),
    }

    println!();

    Ok(())
}
