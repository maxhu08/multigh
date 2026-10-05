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
        AllowedCommand::Add { identity } => change(repository, config, &identity, true),
        AllowedCommand::Remove { identity } => change(repository, config, &identity, false),
    }
}

fn change(repository: &Repository, config: &Config, identity_name: &str, add: bool) -> Result<()> {
    let mut allowed = policy::stored(repository)?;
    let name = identity_name.to_ascii_lowercase();

    if add {
        config.identity(&name)?;
        allowed.insert(name);
    } else {
        allowed.remove(&name);
    }

    if allowed.is_empty()
        || !add
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
