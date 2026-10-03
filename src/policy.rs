use crate::{
    config::Config,
    git, github,
    output::{self, Color},
};
use anyhow::{Result, ensure};
use std::{
    collections::BTreeSet,
    io::{self, IsTerminal},
};

pub fn repository() -> Result<bool> {
    Ok(git::optional(&["rev-parse", "--git-dir"])?.is_some())
}

pub fn allowed(config: &Config) -> Result<BTreeSet<String>> {
    if !repository()? {
        return Ok(BTreeSet::new());
    }

    let explicit = git::optional(&["config", "--local", "--get-all", "mgh.allowedAccount"])?;
    let mut allowed = BTreeSet::new();

    if let Some(values) = explicit {
        for alias in values.lines() {
            config.account(alias)?;
            allowed.insert(alias.to_ascii_lowercase());
        }
    } else {
        let pinned = git::local("mgh.account")?.map(|alias| alias.to_ascii_lowercase());
        let legacy = git::local("ghguard.account")?.map(|alias| alias.to_ascii_lowercase());

        ensure!(
            pinned.is_none() || legacy.is_none() || pinned == legacy,
            "Repository account settings conflict"
        );

        if let Some(alias) = pinned.or(legacy) {
            config.account(&alias)?;
            allowed.insert(alias);
        }
    }

    Ok(allowed)
}

pub fn active(config: &Config, allowed: &BTreeSet<String>, login: &str) -> Result<String> {
    ensure!(
        !allowed.is_empty(),
        "No accounts are authorized for this repository.\nRun: mgh protections --repo"
    );

    let alias = allowed
        .iter()
        .find(|alias| config.accounts[*alias].username.eq_ignore_ascii_case(login));

    ensure!(
        alias.is_some(),
        "GitHub is using {login}, which is not allowed in this repository.\nAllowed accounts: {}\nRun: mgh switch <allowed-account>",
        allowed
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    );

    Ok(alias.unwrap().clone())
}

pub fn identity(config: &Config, alias: &str) -> Result<()> {
    let account = config.account(alias)?;

    git::set("--local", "user.name", &account.name)?;
    git::set("--local", "user.email", &account.email)?;
    git::set("--local", "mgh.account", &alias.to_ascii_lowercase())?;

    Ok(())
}

pub fn authorize(config: &Config, aliases: &[String]) -> Result<()> {
    ensure!(repository()?, "Run this command inside a Git repository");

    let aliases: BTreeSet<_> = aliases
        .iter()
        .map(|alias| alias.to_ascii_lowercase())
        .collect();

    ensure!(!aliases.is_empty(), "Select at least one allowed account");

    for alias in &aliases {
        config.account(alias)?;
    }

    let selected = github::selected()?.unwrap_or_default();
    let preferred = aliases
        .iter()
        .find(|alias| {
            config.accounts[*alias]
                .username
                .eq_ignore_ascii_case(&selected)
        })
        .unwrap_or(aliases.first().unwrap());

    git::run(&[
        "config",
        "--local",
        "--replace-all",
        "mgh.allowedAccount",
        aliases.first().unwrap(),
    ])?;

    for alias in aliases.iter().skip(1) {
        git::run(&["config", "--local", "--add", "mgh.allowedAccount", alias])?;
    }

    identity(config, preferred)?;

    if git::local("ghguard.account")?.is_some() {
        git::run(&["config", "--local", "--unset-all", "ghguard.account"])?;
    }

    Ok(())
}

pub fn interactive() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal() && io::stderr().is_terminal()
}

pub fn choose(config: &Config) -> Result<()> {
    ensure!(
        interactive(),
        "Account selection needs an interactive terminal.\nRun: mgh protections --repo\nFor automation: mgh protections --allow personal,school,work"
    );

    let current: BTreeSet<_> =
        git::optional(&["config", "--local", "--get-all", "mgh.allowedAccount"])?
            .or(git::local("mgh.account")?)
            .or(git::local("ghguard.account")?)
            .unwrap_or_default()
            .lines()
            .map(str::to_ascii_lowercase)
            .collect();
    let location = git::optional(&["rev-parse", "--show-toplevel"])?.unwrap_or_else(|| {
        std::env::current_dir()
            .unwrap_or_default()
            .display()
            .to_string()
    });

    cliclack::intro(format!("Repository · {location}"))?;

    let mut prompt = cliclack::multiselect("Which accounts may use this repository?")
        .initial_values(current.into_iter().collect())
        .max_rows(7);

    for (alias, account) in &config.accounts {
        prompt = prompt.item(alias.clone(), alias, &account.username);
    }

    let selected = prompt.interact()?;

    authorize(config, &selected)?;
    cliclack::outro("Allowed accounts saved")?;

    show(config)
}

pub fn show(config: &Config) -> Result<()> {
    let allowed = allowed(config)?;

    output::section("Allowed accounts");

    if allowed.is_empty() {
        output::row("Repository", "No accounts selected", Color::Warning);
    }

    for alias in allowed {
        output::row(&alias, &config.account(&alias)?.username, Color::Value);
    }

    println!();

    Ok(())
}
