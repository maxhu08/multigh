use crate::{
    descriptions::subcommands::status as descriptions,
    utils::{config::Config, print::identity},
};
use anyhow::{Context, Result, ensure};
use std::{
    io::{self, Write},
    process::{Command, ExitCode},
};

#[derive(clap::Args)]
#[command(about = descriptions::SELF)]
pub struct StatusArgs;

pub(super) fn run(_args: StatusArgs) -> ExitCode {
    match status() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn status() -> Result<()> {
    let config = Config::load()?;
    let account = Command::new("gh")
        .args([
            "auth",
            "status",
            "--active",
            "--hostname",
            "github.com",
            "--json",
            "hosts",
            "--jq",
            ".hosts[\"github.com\"][] | select(.active) | .login",
        ])
        .output()
        .context("Could not run gh")?;

    ensure!(
        account.status.success(),
        "gh auth status failed ({}): {}",
        account.status,
        String::from_utf8_lossy(&account.stderr).trim()
    );

    let username = String::from_utf8(account.stdout).context("Invalid UTF-8 output from gh")?;
    let username = username.trim();
    ensure!(!username.is_empty(), "No active GitHub account");

    let (name, details) = config
        .identity_for_username(username)
        .with_context(|| format!("GitHub account '{username}' has no configured identity"))?;
    let mut output = io::stdout().lock();
    writeln!(output, "Current identity")?;
    identity::print(&mut output, name, details)?;
    Ok(())
}
