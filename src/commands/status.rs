use crate::{
    config::{CommitDetails, Config},
    git, github, guard, hooks,
    output::{self, Color},
    policy,
    repository::Repository,
    settings,
};
use anyhow::{Result, ensure};
use std::path::PathBuf;

pub fn repository(
    repository: &Repository,
    config: &Config,
    active: &str,
    before: Option<&CommitDetails>,
) -> Result<()> {
    output::heading(
        "Current repository",
        &repository
            .root
            .file_name()
            .unwrap_or_default()
            .to_string_lossy(),
        Color::Heading,
    );
    output::row(
        "Repo config",
        &repository.config_path().to_string_lossy(),
        Color::Muted,
    );

    let allowed = policy::allowed(repository, config)?;

    output::row(
        "Active identity",
        config
            .identity_for_username(active)
            .map(|(name, _)| name)
            .unwrap_or("not configured"),
        Color::Changed,
    );
    output::row("GitHub username", active, Color::Value);

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

    let details = CommitDetails {
        name: repository.value("user.name")?.unwrap_or_default(),
        email: repository.value("user.email")?.unwrap_or_default(),
    };

    if let Some(before) = before {
        output::change("Commit name", &before.name, &details.name);
        output::change("Commit email", &before.email, &details.email);
    } else {
        output::row("Commit name", &details.name, Color::Value);
        output::row("Commit email", &details.email, Color::Value);
    }

    let enabled = settings::protections(repository)?;
    let problems = hooks::readiness::problems(repository)?;
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

    let autoswitch = settings::enabled(settings::Preference::Autoswitch)?;
    output::row(
        "Autoswitch (global)",
        if autoswitch { "ON" } else { "OFF" },
        if autoswitch {
            Color::Changed
        } else {
            Color::Muted
        },
    );

    if enabled {
        for problem in problems {
            output::warning(&problem);
        }
    }

    match policy::active(config, &allowed, active) {
        Ok((identity_name, identity)) => {
            if let Err(error) = guard::commit_details(repository, identity_name, identity) {
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
        .find(|account| account.active)
        .and_then(|account| account.login.as_deref())
        .unwrap_or("unavailable");
    let config = Config::load(path.clone());

    if let Ok(config) = &config {
        print_identities(config, &accounts)?;
    }

    let unconfigured: Vec<_> = accounts
        .iter()
        .filter(|account| {
            !config.as_ref().is_ok_and(|config| {
                config.identities.values().any(|identity| {
                    account
                        .login
                        .as_deref()
                        .is_some_and(|login| identity.matches_username(login))
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
                    account.login.as_deref().unwrap_or("unknown"),
                    Color::Value,
                    false
                ),
                if account.active {
                    format!(" {}", output::paint("(Active)", Color::Changed, false))
                } else {
                    String::new()
                }
            );

            if let Some(state) = account.state.as_deref().filter(|state| *state != "success") {
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
        Ok(config) => {
            if let Some(repo) = Repository::discover()? {
                repository(&repo, &config, active, None)?;
            }
        }
        Err(error) => output::warning(&format!("{error:#}")),
    }

    println!();

    if full {
        let result = github::report()?;
        output::block(&String::from_utf8_lossy(&result.stdout), false);
        output::block(&String::from_utf8_lossy(&result.stderr), true);
        ensure!(
            result.status.success(),
            "GitHub authentication check failed"
        );
    }

    ensure!(active != "unavailable", "No active GitHub account");

    Ok(())
}

pub fn identities(config: &Config) -> Result<()> {
    let accounts = match github::accounts() {
        Ok(accounts) => accounts,
        Err(error) => {
            output::warning(&format!("{error:#}"));
            Vec::new()
        }
    };
    print_identities(config, &accounts)?;
    output::section("Accounts");
    println!("  {}", config.path.display());
    Ok(())
}

fn print_identities(config: &Config, accounts: &[github::Account]) -> Result<()> {
    output::section("Identities");

    for (identity_name, identity) in &config.identities {
        let account = accounts.iter().find(|account| {
            account
                .login
                .as_deref()
                .is_some_and(|login| identity.matches_username(login))
        });
        let marker = match account {
            Some(account) if account.active => {
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

        let location = git::identity_files::directory()?.join(format!("git-{identity_name}.conf"));

        output::nested_row(
            "Identity file",
            &location.to_string_lossy(),
            Color::Muted,
            4,
        );

        if let Some(state) = account
            .and_then(|account| account.state.as_deref())
            .filter(|state| *state != "success")
        {
            output::warning(state);
        }

        println!();
    }
    Ok(())
}
