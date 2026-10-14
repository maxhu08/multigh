use crate::{
    config::Config,
    github, policy,
    repository::Repository,
    utils::{output, terminal},
};
use anyhow::{Result, ensure};

pub enum Selection {
    None,
    AlreadyActive,
    Switched,
}

pub fn select(repository: &Repository, config: &Config) -> Result<Selection> {
    let allowed = policy::allowed(repository, config)?;
    if allowed.is_empty() {
        return Ok(Selection::None);
    }
    let login = github::selected()?;
    let current = allowed.iter().find(|name| {
        login
            .as_ref()
            .is_some_and(|login| config.identities[*name].matches_username(login))
    });
    let name = if allowed.len() == 1 {
        let name = allowed.first().unwrap();
        output::autoswitch(name, current.is_some());
        if current.is_some() {
            return Ok(Selection::AlreadyActive);
        }

        name.clone()
    } else {
        ensure!(
            terminal::interactive(),
            "Several identities are allowed; choose one in an interactive terminal.\nRun: mgh switch <allowed-identity>"
        );
        output::form_intro("Switch identity")?;
        let mut prompt = cliclack::select("Which allowed identity should be active?").max_rows(7);
        if let Some(current) = current {
            prompt = prompt.initial_value(current.clone());
        }
        for name in &allowed {
            prompt = prompt.item(
                name.clone(),
                output::identity_name(name, output::Stream::Stderr),
                output::username(&config.identities[name].username, output::Stream::Stderr),
            );
        }
        let selected = prompt.interact()?;
        output::form_outro("Identity chosen")?;
        selected
    };
    super::super::global::update(&format!("mgh switch {name}"), |global, report| {
        let selected = super::super::switch::select(config, &name, Some(repository), global)?;
        super::super::switch::report(
            config,
            Some(repository),
            &selected,
            super::super::switch::ReportDetail::Brief,
        )?;
        report.suppress();
        Ok(())
    })?;

    Ok(Selection::Switched)
}
