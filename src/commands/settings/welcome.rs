use crate::{
    config::Config,
    git, github,
    output::{self, Color},
    policy, settings,
};
use anyhow::Result;
use std::path::PathBuf;

pub fn run(path: PathBuf) -> Result<()> {
    if !settings::enabled(settings::Preference::Welcome)? {
        return Ok(());
    }

    let selected = github::selected()?.unwrap_or_else(|| "unavailable".into());
    let values = git::entries(
        None,
        "^(user\\.(name|email)|mgh\\.(current-identity|allowed-identity|account|allowedaccount)|ghguard\\.account)$",
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
        .and_then(|config| config.identity_for_username(&selected))
        .map(|(identity_name, _)| identity_name)
        .unwrap_or("not configured");

    output::heading("Identity", selected_identity, Color::Changed);
    output::row("GitHub username", &selected, Color::Value);
    output::row(
        "Commit email",
        value("user.email").unwrap_or("not configured"),
        Color::Value,
    );

    match config {
        Ok(config) => {
            let identity_names = match policy::decode(&values) {
                Ok(names) => names,
                Err(error) => {
                    output::warning(&error.to_string());
                    println!();
                    return Ok(());
                }
            };
            let mut allowed_identity = None;

            for identity_name in &identity_names {
                match config.identity(identity_name) {
                    Ok(identity) if identity.matches_username(&selected) => {
                        allowed_identity = Some((identity_name, identity))
                    }
                    Err(error) => output::warning(&error.to_string()),
                    _ => {}
                }
            }

            if let Some((identity_name, identity)) = allowed_identity {
                if !identity.matches_commit(
                    value("user.name").unwrap_or_default(),
                    value("user.email").unwrap_or_default(),
                ) {
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
