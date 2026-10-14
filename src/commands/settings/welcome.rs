use crate::{config::Config, git, github, policy, settings, utils::output};
use anyhow::Result;
use std::path::PathBuf;

pub fn run(path: PathBuf) -> Result<()> {
    if !settings::enabled(settings::Preference::Welcome)? {
        return Ok(());
    }

    let selected = github::selected()?;
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
        .and_then(|config| {
            selected
                .as_deref()
                .and_then(|login| config.identity_for_username(login))
        })
        .map(|(identity_name, _)| identity_name)
        .unwrap_or("not configured");

    output::identity_heading("Identity", selected_identity);
    output::github_username(
        "GitHub username",
        selected.as_deref().unwrap_or("unavailable"),
    );
    output::commit_email(value("user.email").unwrap_or("not configured"));

    match config {
        Ok(config) => {
            let identity_names = match policy::decode(&values) {
                Ok(names) => names,
                Err(error) => {
                    output::warning(&error.to_string());
                    return Ok(());
                }
            };
            let mut allowed_identity = None;

            for identity_name in &identity_names {
                match config.identity(identity_name) {
                    Ok(identity)
                        if selected
                            .as_deref()
                            .is_some_and(|login| identity.matches_username(login)) =>
                    {
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
                    output::warning(&format!(
                        "Commit details need: mgh switch {}",
                        output::identity_name(identity_name, output::Stream::Stdout)
                    ));
                }
            } else if !identity_names.is_empty() {
                output::warning(&format!(
                    "This repository needs: mgh switch {}",
                    identity_names
                        .iter()
                        .map(|name| output::identity_name(name, output::Stream::Stdout))
                        .collect::<Vec<_>>()
                        .join(" or ")
                ));
            }
        }
        Err(error) => output::warning(&format!("{error:#}")),
    }

    Ok(())
}
