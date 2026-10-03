use crate::{config::Config, git, github, output, policy};
use anyhow::Result;

pub fn run(config: &Config, alias: &str, repo: bool) -> Result<()> {
    let command = format!("mgh switch {alias}{}", if repo { " --repo" } else { "" });

    git::update_global(&command, |global| switch(config, alias, repo, global))
}

fn switch(
    config: &Config,
    alias: &str,
    repo: bool,
    global: &mut git::GlobalConfig<'_>,
) -> Result<()> {
    let account = config.account(alias)?;

    if repo {
        anyhow::ensure!(
            policy::repository()?,
            "--repo must be run inside a Git repository"
        );
    }

    let before = [
        github::active().unwrap_or_default(),
        git::global("user.name")?,
        git::global("user.email")?,
    ];
    let effective = [
        git::value("user.name")?.unwrap_or_default(),
        git::value("user.email")?.unwrap_or_default(),
    ];

    git::setup_identities(config, global)?;
    github::switch(account)?;
    global.set("user.name", &account.name)?;
    global.set("user.email", &account.email)?;

    let mut allowed = policy::allowed(config)?;

    if repo {
        allowed.insert(alias.to_ascii_lowercase());
        policy::authorize(config, &allowed.into_iter().collect::<Vec<_>>())?;
    } else if allowed.contains(&alias.to_ascii_lowercase()) {
        policy::identity(config, alias)?;
    }

    output::section(&format!(
        "✓ GitHub account selected · {}",
        alias.to_uppercase()
    ));
    output::change("GitHub account", &before[0], &account.username);

    output::section("Global commit defaults");
    output::change("Name", &before[1], &account.name);
    output::change("Email", &before[2], &account.email);

    super::status::repository(config, &account.username, Some(&effective))?;
    println!();

    Ok(())
}
