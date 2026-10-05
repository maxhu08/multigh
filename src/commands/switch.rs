use crate::{config::Config, git, github, output, policy};
use anyhow::Result;

pub fn run(config: &Config, identity_name: &str) -> Result<()> {
    run_with_report(config, identity_name, true)
}

pub fn automatic(config: &Config, identity_name: &str) -> Result<()> {
    run_with_report(config, identity_name, false)
}

fn run_with_report(config: &Config, identity_name: &str, detailed: bool) -> Result<()> {
    let command = format!("mgh switch {identity_name}");

    git::update_global(&command, |global| {
        switch(config, identity_name, global, detailed)?;

        if !detailed {
            global.suppress_report();
        }

        Ok(())
    })
}

fn switch(
    config: &Config,
    identity_name: &str,
    global: &mut git::GlobalConfig<'_>,
    detailed: bool,
) -> Result<()> {
    let identity = config.identity(identity_name)?;

    let before = [
        github::active().unwrap_or_default(),
        git::global("user.name")?,
        git::global("user.email")?,
    ];
    let effective = if detailed {
        Some([
            git::value("user.name")?.unwrap_or_default(),
            git::value("user.email")?.unwrap_or_default(),
        ])
    } else {
        None
    };

    git::setup_identities(config, global)?;
    github::switch(identity)?;
    global.set("user.name", &identity.commit_name)?;
    global.set("user.email", &identity.commit_email)?;

    let allowed = policy::allowed(config)?;

    if allowed.contains(&identity_name.to_ascii_lowercase()) {
        policy::identity(config, identity_name)?;
    }

    output::heading(
        "✓ Identity selected",
        &identity_name.to_ascii_lowercase(),
        output::Color::Changed,
    );
    output::change("GitHub account", &before[0], &identity.username);

    output::section("Global commit defaults");
    output::change("Name", &before[1], &identity.commit_name);
    output::change("Email", &before[2], &identity.commit_email);

    if let Some(effective) = effective {
        super::status::repository(config, &identity.username, Some(&effective))?;
    }
    println!();

    Ok(())
}
