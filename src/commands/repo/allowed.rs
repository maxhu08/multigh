use crate::{cli::AllowedCommand, config::Config, policy, repository::Repository};
use anyhow::Result;

pub fn run(
    repository: &Repository,
    config: &Config,
    command: Option<AllowedCommand>,
) -> Result<()> {
    match command.unwrap_or(AllowedCommand::List) {
        AllowedCommand::List => super::selection::show(repository, config),
        AllowedCommand::Update => super::selection::choose(repository, config),
        AllowedCommand::Add { identity } => {
            change(repository, config, &identity, PermissionChange::Add)
        }
        AllowedCommand::Remove { identity } => {
            change(repository, config, &identity, PermissionChange::Remove)
        }
    }
}

enum PermissionChange {
    Add,
    Remove,
}

fn change(
    repository: &Repository,
    config: &Config,
    identity_name: &str,
    change: PermissionChange,
) -> Result<()> {
    let mut allowed = policy::stored(repository)?;
    let name = identity_name.to_ascii_lowercase();

    match change {
        PermissionChange::Add => {
            config.identity(&name)?;
            allowed.insert(name);
        }
        PermissionChange::Remove => {
            allowed.remove(&name);
        }
    }

    if allowed.is_empty()
        || matches!(change, PermissionChange::Remove)
            && allowed
                .iter()
                .any(|name| !config.identities.contains_key(name))
    {
        policy::save(repository, &allowed)?;
    } else {
        policy::authorize(repository, config, &allowed)?;
    }
    super::selection::show(repository, config)
}
