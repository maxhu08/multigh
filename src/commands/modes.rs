use crate::{
    cli::Toggle,
    config::Config,
    git, hooks,
    output::{self, Color},
    policy, settings,
};
use anyhow::Result;
use std::path::PathBuf;

pub fn report(name: &str) -> Result<()> {
    let enabled = settings::enabled(name)?;

    output::row(
        if name == "protections" {
            "Protections"
        } else {
            "Verbose"
        },
        if enabled { "ON" } else { "OFF" },
        if enabled {
            Color::Changed
        } else {
            Color::Muted
        },
    );
    println!(
        "  {}",
        if name == "protections" {
            "Ask for allowed accounts in new repos; block commits and pushes from other accounts."
        } else {
            "Show this repo's allowed accounts when entering it or starting a terminal in it."
        }
    );
    println!();

    Ok(())
}

pub fn protections(
    path: PathBuf,
    state: Option<Toggle>,
    repo: bool,
    allow: Vec<String>,
) -> Result<()> {
    if let Some(state) = state {
        let enabled = matches!(state, Toggle::On);

        if enabled {
            git::update_global("mgh protections on", |global| {
                hooks::install(&Config::load(path.clone())?, global)
            })?;
        }

        settings::set("protections", enabled)?;
        report("protections")?;

        if enabled {
            super::enter::run(path)?;
        }

        return Ok(());
    }

    report("protections")?;

    if repo || !allow.is_empty() || policy::repository()? {
        let config = Config::load(path)?;

        if repo {
            policy::choose(&config)?;
        } else if !allow.is_empty() {
            policy::authorize(&config, &allow)?;
            policy::show(&config)?;
        } else {
            policy::show(&config)?;
        }
    }

    Ok(())
}

pub fn verbose(state: Option<Toggle>) -> Result<()> {
    if let Some(state) = state {
        settings::set("verbose", matches!(state, Toggle::On))?;
    }

    report("verbose")
}
