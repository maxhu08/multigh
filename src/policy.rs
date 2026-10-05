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

pub const ALLOWED: &str = "mgh.allowed-identity";
pub const CURRENT: &str = "mgh.current-identity";

pub fn repository() -> Result<bool> {
    Ok(git::optional(&["rev-parse", "--git-dir"])?.is_some())
}

pub fn stored() -> Result<BTreeSet<String>> {
    if !repository()? {
        return Ok(BTreeSet::new());
    }

    let values = git::entries(
        Some("--local"),
        "^(mgh\\.(allowed-identity|current-identity|allowedaccount|account)|ghguard\\.account)$",
    )?;
    let explicit = values
        .get(ALLOWED)
        .or_else(|| values.get("mgh.allowedaccount"));
    let mut allowed = BTreeSet::new();

    if let Some(values) = explicit {
        for name in values.iter().filter(|value| !value.is_empty()) {
            allowed.insert(name.to_ascii_lowercase());
        }
    } else {
        let pinned = values
            .get(CURRENT)
            .or_else(|| values.get("mgh.account"))
            .and_then(|values| values.last())
            .map(|name| name.to_ascii_lowercase());
        let legacy = values
            .get("ghguard.account")
            .and_then(|values| values.last())
            .map(|name| name.to_ascii_lowercase());
        ensure!(
            pinned.is_none() || legacy.is_none() || pinned == legacy,
            "Repository identity settings conflict"
        );
        if let Some(name) = pinned.or(legacy) {
            allowed.insert(name);
        }
    }
    Ok(allowed)
}

pub fn migrate() -> Result<()> {
    if !repository()? {
        return Ok(());
    }

    let legacy = git::entries(Some("--local"), "^mgh\\.(allowedaccount|account)$")?;

    for (old, new) in [("mgh.allowedaccount", ALLOWED), ("mgh.account", CURRENT)] {
        if let Some(values) = legacy.get(old) {
            if git::local(new)?.is_none() {
                for value in values {
                    git::run(&["config", "--local", "--add", new, value])?;
                }
            }

            git::run(&["config", "--local", "--unset-all", old])?;
        }
    }

    Ok(())
}

pub fn allowed(config: &Config) -> Result<BTreeSet<String>> {
    let allowed = stored()?;

    for name in &allowed {
        config.identity(name)?;
    }

    Ok(allowed)
}

pub fn active(config: &Config, allowed: &BTreeSet<String>, login: &str) -> Result<String> {
    ensure!(
        !allowed.is_empty(),
        "No identities are authorized for this repository.\nRun: mgh repo allowed update"
    );

    let identity_name = allowed.iter().find(|identity_name| {
        config.identities[*identity_name]
            .username
            .eq_ignore_ascii_case(login)
    });

    ensure!(
        identity_name.is_some(),
        "GitHub is using {login}, which is not allowed in this repository.\nAllowed identities: {}\nRun: mgh switch <allowed-identity>",
        allowed
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    );

    Ok(identity_name.unwrap().clone())
}

pub fn identity(config: &Config, identity_name: &str) -> Result<()> {
    let identity = config.identity(identity_name)?;

    migrate()?;
    git::set("--local", "user.name", &identity.commit_name)?;
    git::set("--local", "user.email", &identity.commit_email)?;
    git::set("--local", CURRENT, &identity_name.to_ascii_lowercase())?;

    Ok(())
}

pub fn authorize(config: &Config, identity_names: &[String]) -> Result<()> {
    ensure!(repository()?, "Run this command inside a Git repository");

    let identity_names: BTreeSet<_> = identity_names
        .iter()
        .map(|identity_name| identity_name.to_ascii_lowercase())
        .collect();

    ensure!(
        !identity_names.is_empty(),
        "Select at least one allowed identity"
    );

    for identity_name in &identity_names {
        config.identity(identity_name)?;
    }

    let selected = github::selected()?.unwrap_or_default();
    let preferred = identity_names
        .iter()
        .find(|identity_name| {
            config.identities[*identity_name]
                .username
                .eq_ignore_ascii_case(&selected)
        })
        .unwrap_or(identity_names.first().unwrap());

    save(&identity_names)?;
    identity(config, preferred)?;

    if git::local("ghguard.account")?.is_some() {
        git::run(&["config", "--local", "--unset-all", "ghguard.account"])?;
    }

    Ok(())
}

pub fn save(identity_names: &BTreeSet<String>) -> Result<()> {
    migrate()?;

    git::run(&[
        "config",
        "--local",
        "--replace-all",
        ALLOWED,
        identity_names.first().map(String::as_str).unwrap_or(""),
    ])?;

    for name in identity_names.iter().skip(1) {
        git::run(&["config", "--local", "--add", ALLOWED, name])?;
    }

    Ok(())
}

pub fn interactive() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal() && io::stderr().is_terminal()
}

pub fn choose(config: &Config) -> Result<()> {
    ensure!(
        interactive(),
        "Identity selection needs an interactive terminal.\nRun: mgh repo allowed update\nFor automation: mgh repo allowed add <identity>"
    );

    let current: Vec<_> = stored()?
        .into_iter()
        .filter(|name| config.identities.contains_key(name))
        .collect();
    let location = git::optional(&["rev-parse", "--show-toplevel"])?.unwrap_or_else(|| {
        std::env::current_dir()
            .unwrap_or_default()
            .display()
            .to_string()
    });

    cliclack::intro(output::form_heading("Repository", &location))?;

    let mut prompt = cliclack::multiselect("Which identities may use this repository?")
        .initial_values(current)
        .max_rows(7);

    for (identity_name, identity) in &config.identities {
        prompt = prompt.item(identity_name.clone(), identity_name, &identity.username);
    }

    let selected = prompt.interact()?;

    authorize(config, &selected)?;
    cliclack::outro("Allowed identities saved")?;

    show(config)
}

pub fn show(config: &Config) -> Result<()> {
    let allowed = stored()?;

    output::section("Allowed identities");

    if allowed.is_empty() {
        output::row("Repository", "No identities selected", Color::Warning);
    }

    for identity_name in allowed {
        match config.identity(&identity_name) {
            Ok(identity) => output::row(&identity_name, &identity.username, Color::Value),
            Err(_) => output::row(
                &identity_name,
                "Not configured; remove this permission or run mgh repo allowed update",
                Color::Warning,
            ),
        }
    }

    println!();

    Ok(())
}
