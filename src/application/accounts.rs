use super::platform;
use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use std::{
    io::IsTerminal,
    process::{Command, Output, Stdio},
};

// Guard code receives this query-only view, never authentication controls.
pub(super) struct AccountQueries;
impl AccountQueries {
    pub fn active(&self) -> Result<LiveAccount> {
        active()
    }
}

#[derive(Clone, Debug)]
pub(super) struct SelectedAccount(String);
impl SelectedAccount {
    pub fn login(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Deserialize)]
pub(super) struct LiveAccount {
    pub login: String,
    #[serde(default)]
    pub active: bool,
    pub state: String,
    #[serde(default)]
    pub error: Option<String>,
}

pub(super) fn selected() -> Result<Option<SelectedAccount>> {
    let mut cmd = Command::new("gh");
    cmd.args(["config", "get", "user", "--host", "github.com"]);
    let out = platform::output(&mut cmd)?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        if out.status.code() == Some(1)
            && (err.contains("not set")
                || err.contains("not found")
                || err.contains("does not exist"))
        {
            return Ok(None);
        }
        bail!(
            "gh selected-account query failed ({}): {}",
            out.status,
            err.trim()
        );
    }
    let value = platform::text(out.stdout)?.trim().to_string();
    Ok(if value.is_empty() {
        None
    } else {
        Some(SelectedAccount(value))
    })
}

pub(super) fn all() -> Result<Vec<LiveAccount>> {
    let out = platform::successful(Command::new("gh").args([
        "auth",
        "status",
        "--hostname",
        "github.com",
        "--json",
        "hosts",
    ]))?;
    #[derive(Deserialize)]
    struct Response {
        hosts: std::collections::BTreeMap<String, Vec<LiveAccount>>,
    }
    let mut response: Response =
        serde_json::from_slice(&out.stdout).context("gh account query returned malformed JSON")?;
    Ok(response.hosts.remove("github.com").unwrap_or_default())
}

pub(super) fn active() -> Result<LiveAccount> {
    let active = all()?
        .into_iter()
        .find(|a| a.active)
        .context("No active GitHub account")?;
    ensure!(
        active.state == "success",
        "GitHub authentication check failed for {}: {}",
        active.login,
        active.error.as_deref().unwrap_or(&active.state)
    );
    Ok(active)
}

pub(super) fn ensure_login(username: &str) -> Result<()> {
    if all()?
        .iter()
        .any(|a| a.login.eq_ignore_ascii_case(username) && a.state == "success")
    {
        return Ok(());
    }
    let status = Command::new("gh")
        .args(["auth", "login", "--hostname", "github.com", "--web"])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("Cannot execute gh auth login")?;
    ensure!(status.success(), "gh auth login failed ({status})");
    ensure!(
        all()?
            .iter()
            .any(|a| a.login.eq_ignore_ascii_case(username) && a.state == "success"),
        "GitHub account '{username}' is not signed in; authorize that account and retry"
    );
    Ok(())
}

pub(super) fn switch(username: &str) -> Result<()> {
    platform::successful(Command::new("gh").args([
        "auth",
        "switch",
        "--hostname",
        "github.com",
        "--user",
        username,
    ]))?;
    Ok(())
}

pub(super) fn full() -> Result<Output> {
    let mut cmd = Command::new("gh");
    cmd.args(["auth", "status", "--hostname", "github.com"]);
    if !std::io::stdout().is_terminal()
        || !std::io::stderr().is_terminal()
        || std::env::var_os("NO_COLOR").is_some()
        || std::env::var("TERM").as_deref() == Ok("dumb")
        || std::env::var("CLICOLOR").as_deref() == Ok("0")
    {
        cmd.env("CLICOLOR", "0")
            .env("NO_COLOR", "1")
            .env_remove("CLICOLOR_FORCE")
            .env_remove("GH_FORCE_TTY");
    } else {
        // The presentation adapter indents piped output. Keep the colors gh
        // would have emitted with the original terminal streams.
        cmd.env("CLICOLOR_FORCE", "1");
    }
    cmd.output().context("Cannot execute gh auth status")
}
