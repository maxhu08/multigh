use crate::{
    config::Identity,
    utils::{output, process},
};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::{env, process::Output};

#[derive(Deserialize)]
pub struct Account {
    pub login: Option<String>,
    #[serde(default)]
    pub active: bool,
    pub state: Option<String>,
}

pub fn report() -> Result<Output> {
    let styled = output::color_enabled(output::Stream::Stdout)
        && output::color_enabled(output::Stream::Stderr)
        && !env::var("CLICOLOR").is_ok_and(|value| value == "0");
    let preference = if styled {
        ("CLICOLOR_FORCE", "1")
    } else {
        ("NO_COLOR", "1")
    };

    process::capture_with_env("gh", &["auth", "status"], &[preference])
}

pub fn active() -> Result<String> {
    process::run(
        "gh",
        &[
            "auth",
            "status",
            "--active",
            "--hostname",
            "github.com",
            "--json",
            "hosts",
            "--jq",
            ".hosts[\"github.com\"][] | select(.active) | .login",
        ],
    )
}

pub fn selected() -> Result<Option<String>> {
    process::optional(
        "gh",
        &["config", "get", "user", "--host", "github.com"],
        &[1],
    )
}

pub fn accounts() -> Result<Vec<Account>> {
    let data = process::run(
        "gh",
        &[
            "auth",
            "status",
            "--hostname",
            "github.com",
            "--json",
            "hosts",
            "--jq",
            "(.hosts[\"github.com\"] // []) | map({login, active, state})",
        ],
    )?;

    serde_json::from_str(&data).context("Read GitHub authentication accounts")
}

pub fn switch(identity: &Identity) -> Result<()> {
    process::run(
        "gh",
        &[
            "auth",
            "switch",
            "--hostname",
            "github.com",
            "--user",
            &identity.username,
        ],
    )?;

    Ok(())
}

pub fn login(username: &str) -> Result<()> {
    let signed_in = || -> Result<bool> {
        Ok(accounts()?.iter().any(|account| {
            account
                .login
                .as_deref()
                .is_some_and(|login| login.eq_ignore_ascii_case(username))
                && account.state.as_deref() == Some("success")
        }))
    };

    if signed_in()? {
        return Ok(());
    }

    crate::utils::output::section(&format!("Sign in to GitHub as {username}"));

    crate::utils::process::interactive(
        "gh",
        &["auth", "login", "--hostname", "github.com", "--web"],
    )?;

    ensure!(
        signed_in()?,
        "GitHub account '{username}' is not signed in; authorize that account in the browser and retry"
    );

    Ok(())
}
