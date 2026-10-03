use crate::{
    config::Config,
    git, github, guard, hooks,
    output::{self, Color},
    policy, settings,
};
use anyhow::{Result, ensure};
use std::{path::PathBuf, process::Command};

pub fn repository(config: &Config, active: &str, before: Option<&[String; 2]>) -> Result<()> {
    let Some(location) = git::optional(&["rev-parse", "--show-toplevel"])? else {
        return Ok(());
    };

    output::section(&format!(
        "Current repository · {}",
        PathBuf::from(location)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
    ));

    let allowed = policy::allowed(config)?;

    output::row(
        "Allowed identities",
        &if allowed.is_empty() {
            "none selected".to_owned()
        } else {
            allowed
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        },
        Color::Value,
    );

    for (index, (label, key)) in [("Commit name", "user.name"), ("Commit email", "user.email")]
        .iter()
        .enumerate()
    {
        let value = git::value(key)?.unwrap_or_default();

        if let Some(before) = before {
            output::change(label, &before[index], &value);
        } else {
            output::row(label, &value, Color::Value);
        }
    }

    let enabled = settings::enabled("protections")?;
    let problems = hooks::problems()?;
    let installed = problems.is_empty();

    output::row(
        "Protection",
        if !enabled {
            "OFF"
        } else if installed {
            "Commit + push checks enabled"
        } else {
            "Shared hooks unavailable or overridden"
        },
        if enabled && installed {
            Color::Changed
        } else {
            Color::Warning
        },
    );

    if enabled {
        for problem in problems {
            output::warning(&problem);
        }
    }

    match policy::active(config, &allowed, active) {
        Ok(identity_name) => {
            if let Err(error) =
                guard::commit_details(&identity_name, config.identity(&identity_name)?)
            {
                output::warning(&error.to_string());
            } else {
                output::row(
                    "Status",
                    "✓ Identity and commit details match",
                    Color::Changed,
                );
            }
        }
        Err(error) => output::warning(&error.to_string()),
    }

    Ok(())
}

pub fn run(path: PathBuf, full: bool) -> Result<()> {
    let accounts = github::accounts()?;
    let active = accounts
        .iter()
        .find(|account| account["active"].as_bool() == Some(true))
        .and_then(|account| account["login"].as_str())
        .unwrap_or("unavailable");
    let config = Config::load(path.clone());

    if let Ok(config) = &config {
        output::section("Identities");

        for (identity_name, identity) in &config.identities {
            let account = accounts.iter().find(|account| {
                account["login"]
                    .as_str()
                    .is_some_and(|login| identity.username.eq_ignore_ascii_case(login))
            });
            let marker = match account {
                Some(account) if account["active"].as_bool() == Some(true) => {
                    format!(" {}", output::paint("(Active)", Color::Changed, false))
                }
                None => format!(
                    " {}",
                    output::paint("(Not signed in)", Color::Warning, false)
                ),
                _ => String::new(),
            };

            println!(
                "  {}{marker}",
                output::paint(identity_name, Color::Value, false),
            );
            output::nested_row("GitHub username", &identity.username, Color::Value, 4);
            output::nested_row("Commit name", &identity.commit_name, Color::Value, 4);
            output::nested_row("Commit email", &identity.commit_email, Color::Value, 4);

            let location = git::identity_directory()?.join(format!("git-{identity_name}.conf"));

            output::nested_row(
                "Identity file",
                &location.to_string_lossy(),
                Color::Muted,
                4,
            );

            if let Some(state) = account
                .and_then(|account| account["state"].as_str())
                .filter(|state| *state != "success")
            {
                output::warning(state);
            }

            println!();
        }
    }

    let unconfigured: Vec<_> = accounts
        .iter()
        .filter(|account| {
            !config.as_ref().is_ok_and(|config| {
                config.identities.values().any(|identity| {
                    account["login"]
                        .as_str()
                        .is_some_and(|login| identity.username.eq_ignore_ascii_case(login))
                })
            })
        })
        .collect();

    if !unconfigured.is_empty() {
        output::section(if config.is_ok() {
            "Unconfigured GitHub accounts"
        } else {
            "GitHub accounts"
        });

        for account in unconfigured {
            println!(
                "  {}{}",
                output::paint(
                    account["login"].as_str().unwrap_or("unknown"),
                    Color::Value,
                    false
                ),
                if account["active"].as_bool() == Some(true) {
                    format!(" {}", output::paint("(Active)", Color::Changed, false))
                } else {
                    String::new()
                }
            );

            if let Some(state) = account["state"]
                .as_str()
                .filter(|state| *state != "success")
            {
                output::warning(state);
            }

            println!();
        }
    }

    output::section("Accounts");
    println!(
        "  {}",
        output::paint(&path.to_string_lossy(), Color::Muted, false)
    );

    match config {
        Ok(config) => repository(&config, active, None)?,
        Err(error) => output::warning(&format!("{error:#}")),
    }

    println!();

    if full {
        ensure!(
            Command::new("gh")
                .args(["auth", "status"])
                .status()?
                .success(),
            "GitHub authentication check failed"
        );
    }

    ensure!(active != "unavailable", "No active GitHub account");

    Ok(())
}
