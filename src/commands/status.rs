use crate::{
    config::{CommitDetails, Config},
    git, github, guard, hooks, policy,
    repository::Repository,
    settings,
    utils::output,
};
use anyhow::{Context, Result, ensure};
use std::path::PathBuf;

pub fn repository(
    repository: &Repository,
    config: &Config,
    active: Option<&str>,
    before: Option<&CommitDetails>,
) -> Result<()> {
    output::repository_heading(
        "Current repository",
        &repository
            .root
            .file_name()
            .unwrap_or_default()
            .to_string_lossy(),
    );
    output::path("Repo config", &repository.config_path());

    let allowed = policy::allowed(repository, config)?;

    output::identity_value(
        "Active identity",
        active
            .and_then(|login| config.identity_for_username(login))
            .map(|(name, _)| name)
            .unwrap_or("not configured"),
    );
    output::github_username("GitHub username", active.unwrap_or("unavailable"));

    output::identity_value(
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
    );

    let details = CommitDetails {
        name: repository.value("user.name")?.unwrap_or_default(),
        email: repository.value("user.email")?.unwrap_or_default(),
    };

    output::commit_details(&details, before);

    let enabled = settings::protections(repository)?;
    let problems = hooks::readiness::problems(repository)?;
    let installed = problems.is_empty();

    if enabled && installed {
        output::success("Protection", "Commit + push checks enabled");
    } else {
        output::warning_row(
            "Protection",
            if enabled {
                "Shared hooks unavailable or overridden"
            } else {
                "OFF"
            },
        );
    }

    output::toggle(
        "Autoswitch (global)",
        settings::enabled(settings::Preference::Autoswitch)?,
    );

    if enabled {
        for problem in problems {
            output::warning(&problem);
        }
    }

    match active
        .context("No active GitHub account")
        .and_then(|login| policy::active(config, &allowed, login))
    {
        Ok((identity_name, identity)) => {
            if let Err(error) = guard::commit_details(repository, identity_name, identity) {
                output::warning(&error.to_string());
            } else {
                output::success("Status", "✓ Identity and commit details match");
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
        .and_then(|account| account.login.as_deref());
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
            output::account(
                account.login.as_deref().unwrap_or("unknown"),
                account.active,
            );

            if let Some(state) = account.state.as_deref().filter(|state| *state != "success") {
                output::warning(state);
            }
        }
    }

    output::path_section("Accounts", &path);

    match config {
        Ok(config) => {
            if let Some(repo) = Repository::discover()? {
                repository(&repo, &config, active, None)?;
            }
        }
        Err(error) => output::warning(&format!("{error:#}")),
    }

    if full {
        let result = github::report()?;
        output::block(
            &String::from_utf8_lossy(&result.stdout),
            output::Stream::Stdout,
        );
        output::block(
            &String::from_utf8_lossy(&result.stderr),
            output::Stream::Stderr,
        );
        ensure!(
            result.status.success(),
            "GitHub authentication check failed"
        );
    }

    ensure!(active.is_some(), "No active GitHub account");

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
    output::path_section("Accounts", &config.path);
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
        let state = match account {
            Some(account) if account.active => output::AccountState::Active,
            Some(_) => output::AccountState::SignedIn,
            None => output::AccountState::NotSignedIn,
        };
        let location = git::identity_files::path(&git::identity_files::directory()?, identity_name);
        output::identity(identity_name, identity, &location, state);

        if let Some(state) = account
            .and_then(|account| account.state.as_deref())
            .filter(|state| *state != "success")
        {
            output::warning(state);
        }
    }
    Ok(())
}
