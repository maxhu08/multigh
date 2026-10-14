use crate::{
    config::Config,
    git, hooks, policy,
    repository::Repository,
    settings,
    utils::{output, terminal},
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
        output::path("Accounts", &path);
        let (config, name) = super::identity::new::create(path, None, Default::default())?;
        super::identity::new::report(&config, &name);
        (config, Some(name))
    };

    configure(&config, first_identity.as_deref())?;

    output::paragraph(
        "Add more identities: mgh identity new\nChoose permissions in a repo: mgh repo allowed update",
    );
    Ok(())
}

pub(super) fn configure(config: &Config, first_identity: Option<&str>) -> Result<()> {
    let result: Result<()> = (|| {
        let repository = Repository::discover()?;
        super::global::update("mgh setup", |global, reporting| {
            let directory = install(config, repository.as_ref(), global)?;
            report(config, repository.as_ref(), &directory, global, reporting)
        })?;

        if let Some(name) = first_identity {
            super::global::update(&format!("mgh switch {name}"), |global, _| {
                let selected = super::switch::select(config, name, repository.as_ref(), global)?;
                super::switch::report(
                    config,
                    repository.as_ref(),
                    &selected,
                    super::switch::ReportDetail::Detailed,
                )
            })?;
        }
        Ok(())
    })();

    if let Some(name) = first_identity {
        result.with_context(|| super::identity::new::recovery_instructions(config, name))
    } else {
        result
    }
}

pub(super) fn report(
    config: &Config,
    repository: Option<&Repository>,
    directory: &Path,
    global: &git::global::Writer<'_>,
    reporting: &mut super::global::Report,
) -> Result<()> {
    output::section("✓ Identity rules updated");
    output::path("Accounts", &config.path);
    output::path("Git config", &git::global::path()?);
    for name in config.identities.keys() {
        output::identity_file(name, &git::identity_files::path(directory, name));
    }
    reporting.print(global);
    if let Some(repository) = repository {
        super::repo::report(repository)?;
    } else {
        output::success("Protections", "ON by default");
        output::text(
            "Commits and pushes require allowed identities; use mgh repo protections off for a repo exception.",
        );
    }
    super::settings::report(settings::Preference::Verbose)?;
    super::settings::report(settings::Preference::Autoswitch)?;
    output::paragraph(
        "Load mgh shell init <fish|bash|zsh> in your shell config for entry prompts and reports.",
    );
    output::paragraph(concat!(
        "Fish (~/.config/fish/config.fish): mgh shell init fish | source\n",
        "Bash (~/.bashrc): eval \"$(mgh shell init bash)\"\n",
        "Zsh (~/.zshrc): eval \"$(mgh shell init zsh)\""
    ));
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
