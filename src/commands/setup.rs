use crate::{
    config::Config,
    git, hooks,
    output::{self, Color},
    policy, settings,
};
use anyhow::{Context, Result};
use std::path::PathBuf;

pub fn run(path: PathBuf) -> Result<()> {
    let config = if path.exists() {
        Some(Config::load(path.clone())?)
    } else {
        None
    };

    if config
        .as_ref()
        .is_none_or(|config| config.identities.is_empty())
    {
        anyhow::ensure!(
            policy::interactive(),
            "Setup needs your first identity. Run mgh identity new --username <username> --email <email> <identity>, or run mgh setup in an interactive terminal.\nAccounts: {}",
            path.display()
        );
        output::section("Set up multigh");
        output::row("Accounts", &path.to_string_lossy(), Color::Muted);
        super::identity::new::run(path, None, None, None, None)?;
    } else {
        refresh(&config.context("Identity configuration unavailable")?)?;
    }

    println!("  Add more identities: mgh identity new");
    println!("  Choose permissions in a repo: mgh repo allowed update\n");
    Ok(())
}

pub fn refresh(config: &Config) -> Result<()> {
    git::update_global("mgh setup", |global| {
        hooks::compatible()?;
        policy::migrate()?;
        let directory = git::setup_identities(config, global)?;
        hooks::install(config, global)?;

        if !settings::enabled("setup-complete")? {
            settings::set("verbose", true)?;
            settings::set("setup-complete", true)?;
        }

        output::section("✓ Identity rules updated");
        output::row("Accounts", &config.path.to_string_lossy(), Color::Muted);
        output::row(
            "Git config",
            &git::global_path()?.to_string_lossy(),
            Color::Muted,
        );
        for name in config.identities.keys() {
            output::row(
                &format!("Identity ({name})"),
                &directory.join(format!("git-{name}.conf")).to_string_lossy(),
                Color::Changed,
            );
        }
        global.report();
        if policy::repository()? {
            super::repo::report()?;
        } else {
            output::row("Protections", "ON by default", Color::Changed);
            println!(
                "  Commits and pushes require allowed identities; use mgh repo protections off for a repo exception.\n"
            );
        }
        super::settings::report("verbose")?;
        super::settings::report("autoswitch")?;
        println!(
            "  Load mgh shell init <fish|bash|zsh> in your shell config for entry prompts and reports.\n"
        );
        println!("  Fish (~/.config/fish/config.fish): mgh shell init fish | source");
        println!("  Bash (~/.bashrc): eval \"$(mgh shell init bash)\"");
        println!("  Zsh (~/.zshrc): eval \"$(mgh shell init zsh)\"\n");
        Ok(())
    })
}
