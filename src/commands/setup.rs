use crate::{
    config::Config,
    git, hooks, policy,
    repository::Repository,
    settings,
    utils::{
        output::{self, Color},
        terminal,
    },
};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub fn run(path: PathBuf) -> Result<()> {
    let config = if path.exists() {
        Some(Config::load(path.clone())?)
    } else {
        None
    };

    let (config, first_identity) = if let Some(config) =
        config.filter(|config| !config.identities.is_empty())
    {
        (config, None)
    } else {
        anyhow::ensure!(
            terminal::interactive(),
            "Setup needs your first identity. Run mgh identity new --username <username> --email <email> <identity>, or run mgh setup in an interactive terminal.\nAccounts: {}",
            path.display()
        );
        output::section("Set up multigh");
        output::row("Accounts", &path.to_string_lossy(), Color::Muted);
        let (config, name) = super::identity::new::create(path, None, None, None, None)?;
        super::identity::new::report(&config, &name);
        (config, Some(name))
    };

    let result: Result<()> = (|| {
        let repository = Repository::discover()?;
        git::global::update("mgh setup", |global| {
            let directory = install(&config, repository.as_ref(), global)?;
            report(&config, repository.as_ref(), &directory, global)
        })?;

        if let Some(name) = &first_identity {
            git::global::update(&format!("mgh switch {name}"), |global| {
                let selected = super::switch::select(&config, name, repository.as_ref(), global)?;
                super::switch::report(&config, repository.as_ref(), &selected, true)
            })?;
        }
        Ok(())
    })();

    if let Some(name) = &first_identity {
        result.with_context(|| super::identity::new::recovery_instructions(&config, name))?;
    } else {
        result?;
    }

    println!("  Add more identities: mgh identity new");
    println!("  Choose permissions in a repo: mgh repo allowed update\n");
    Ok(())
}

pub(super) fn report(
    config: &Config,
    repository: Option<&Repository>,
    directory: &Path,
    global: &mut git::global::Writer<'_>,
) -> Result<()> {
    output::section("✓ Identity rules updated");
    output::row("Accounts", &config.path.to_string_lossy(), Color::Muted);
    output::row(
        "Git config",
        &git::global::path()?.to_string_lossy(),
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
    if let Some(repository) = repository {
        super::repo::report(repository)?;
    } else {
        output::row("Protections", "ON by default", Color::Changed);
        println!(
            "  Commits and pushes require allowed identities; use mgh repo protections off for a repo exception.\n"
        );
    }
    super::settings::report(settings::Preference::Verbose)?;
    super::settings::report(settings::Preference::Autoswitch)?;
    println!(
        "  Load mgh shell init <fish|bash|zsh> in your shell config for entry prompts and reports.\n"
    );
    println!("  Fish (~/.config/fish/config.fish): mgh shell init fish | source");
    println!("  Bash (~/.bashrc): eval \"$(mgh shell init bash)\"");
    println!("  Zsh (~/.zshrc): eval \"$(mgh shell init zsh)\"\n");
    Ok(())
}

pub(super) fn install(
    config: &Config,
    repository: Option<&Repository>,
    global: &mut git::global::Writer<'_>,
) -> Result<PathBuf> {
    hooks::install::compatible()?;
    if let Some(repository) = repository {
        policy::migrate(repository)?;
    }
    let directory = git::identity_files::refresh(config, global)?;
    hooks::install::run(config, repository, global)?;

    if !settings::setup_complete()? {
        settings::set(settings::Preference::Verbose, true)?;
        settings::mark_setup_complete()?;
    }

    Ok(directory)
}
