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
        for identity_name in values.lines() {
            config.identity(identity_name)?;
            allowed.insert(identity_name.to_ascii_lowercase());
        }
    } else {
        let pinned =
            git::local("mgh.account")?.map(|identity_name| identity_name.to_ascii_lowercase());
        let legacy =
            git::local("ghguard.account")?.map(|identity_name| identity_name.to_ascii_lowercase());

        ensure!(
            pinned.is_none() || legacy.is_none() || pinned == legacy,
            "Repository identity settings conflict"
        );

        if let Some(identity_name) = pinned.or(legacy) {
            config.identity(&identity_name)?;
            allowed.insert(identity_name);
        }
    }

    Ok(allowed)
}

pub fn active(config: &Config, allowed: &BTreeSet<String>, login: &str) -> Result<String> {
    ensure!(
        !allowed.is_empty(),
        "No identities are authorized for this repository.\nRun: mgh protections --repo"
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

    git::set("--local", "user.name", &identity.commit_name)?;
    git::set("--local", "user.email", &identity.commit_email)?;
    git::set(
        "--local",
        "mgh.account",
        &identity_name.to_ascii_lowercase(),
    )?;

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

    git::run(&[
        "config",
        "--local",
        "--replace-all",
        "mgh.allowedAccount",
        identity_names.first().unwrap(),
    ])?;

    for identity_name in identity_names.iter().skip(1) {
        git::run(&[
            "config",
            "--local",
            "--add",
            "mgh.allowedAccount",
            identity_name,
        ])?;
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
        "Identity selection needs an interactive terminal.\nRun: mgh protections --repo\nFor automation: mgh protections --allow personal,school,work"
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

    let mut prompt = cliclack::multiselect("Which identities may use this repository?")
        .initial_values(current.into_iter().collect())
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
    let allowed = allowed(config)?;

    output::section("Allowed identities");

    if allowed.is_empty() {
        output::row("Repository", "No identities selected", Color::Warning);
    }

    for identity_name in allowed {
        output::row(
            &identity_name,
            &config.identity(&identity_name)?.username,
            Color::Value,
        );
    }

    println!();

    Ok(())
}
