use crate::{config::Config, git, github, output, policy};
use anyhow::Result;

pub fn run(config: &Config, identity_name: &str, repo: bool) -> Result<()> {
    let command = format!(
        "mgh switch {identity_name}{}",
        if repo { " --repo" } else { "" }
    );

    git::update_global(&command, |global| {
        switch(config, identity_name, repo, global)
    })
}

fn switch(
    config: &Config,
    identity_name: &str,
    repo: bool,
    global: &mut git::GlobalConfig<'_>,
) -> Result<()> {
    let identity = config.identity(identity_name)?;

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
    github::switch(identity)?;
    global.set("user.name", &identity.commit_name)?;
    global.set("user.email", &identity.commit_email)?;

    let mut allowed = policy::allowed(config)?;

    if repo {
        allowed.insert(identity_name.to_ascii_lowercase());
        policy::authorize(config, &allowed.into_iter().collect::<Vec<_>>())?;
    } else if allowed.contains(&identity_name.to_ascii_lowercase()) {
        policy::identity(config, identity_name)?;
    }

    output::section(&format!(
        "✓ Identity selected · {}",
        identity_name.to_ascii_lowercase()
    ));
    output::change("GitHub account", &before[0], &identity.username);

    output::section("Global commit defaults");
    output::change("Name", &before[1], &identity.commit_name);
    output::change("Email", &before[2], &identity.commit_email);

    super::status::repository(config, &identity.username, Some(&effective))?;
    println!();

    Ok(())
}
