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

    output::section("Identity");
    output::row("Selected account", &selected, Color::Value);
    output::row(
        "Commit email",
        value("user.email").unwrap_or("not configured"),
        Color::Value,
    );

    match Config::load(path) {
        Ok(config) => {
            let aliases: BTreeSet<_> = values
                .get("mgh.allowedaccount")
                .cloned()
                .unwrap_or_else(|| {
                    value("mgh.account")
                        .or(value("ghguard.account"))
                        .map(|alias| vec![alias.to_owned()])
                        .unwrap_or_default()
                })
                .into_iter()
                .map(|alias| alias.to_ascii_lowercase())
                .collect();
            let mut selected_alias = None;

            for alias in &aliases {
                match config.account(alias) {
                    Ok(account) if account.username.eq_ignore_ascii_case(&selected) => {
                        selected_alias = Some(alias)
                    }
                    Err(error) => output::warning(&error.to_string()),
                    _ => {}
                }
            }

            if let Some(alias) = selected_alias {
                let account = config.account(alias)?;

                if value("user.name") != Some(account.name.as_str())
                    || !account
                        .emails
                        .contains(&value("user.email").unwrap_or_default().to_ascii_lowercase())
                {
                    output::warning(&format!("Commit identity needs: mgh switch {alias}"));
                }
            } else if !aliases.is_empty() {
                output::warning(&format!(
                    "This repository needs: mgh switch {}",
                    aliases
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
