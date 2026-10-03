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
        "Allowed accounts",
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
        Ok(alias) => {
            if let Err(error) = guard::identity(&alias, config.account(&alias)?) {
                output::warning(&error.to_string());
            } else {
                output::row("Status", "✓ Account and identity match", Color::Changed);
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

    output::section("Identities");

    for account in &accounts {
        let login = account["login"].as_str().unwrap_or("unknown");
        let identity = config.as_ref().ok().and_then(|config| {
            config
                .accounts
                .iter()
                .find(|(_, identity)| identity.username.eq_ignore_ascii_case(login))
        });
        let email = identity
            .map(|(_, identity)| identity.email.as_str())
            .unwrap_or("not configured");

        println!(
            "  {} {}{}",
            output::paint(login, Color::Value, false),
            output::paint(email, Color::Value, false),
            if account["active"].as_bool() == Some(true) {
                format!(" {}", output::paint("(Active)", Color::Changed, false))
            } else {
                String::new()
            }
        );

        let location = match identity {
            Some((alias, _)) => git::identity_directory()?
                .join(format!("git-{alias}.conf"))
                .to_string_lossy()
                .into_owned(),
            None => "Identity not configured".to_owned(),
        };

        println!("  {}", output::paint(&location, Color::Muted, false));

        if let Some(state) = account["state"]
            .as_str()
            .filter(|state| *state != "success")
        {
            output::warning(state);
        }

        println!();
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
