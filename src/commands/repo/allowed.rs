use crate::{cli::AllowedCommand, config::Config, policy};
use anyhow::Result;

pub fn run(config: &Config, command: Option<AllowedCommand>) -> Result<()> {
    match command.unwrap_or(AllowedCommand::List) {
        AllowedCommand::List => policy::show(config),
        AllowedCommand::Update => policy::choose(config),
        command => {
            let mut allowed = policy::stored()?;
            let (name, add) = match command {
                AllowedCommand::Add { identity } => (identity.to_ascii_lowercase(), true),
                AllowedCommand::Remove { identity } => (identity.to_ascii_lowercase(), false),
                _ => unreachable!(),
            };
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
                policy::save(&allowed)?;
            } else {
                policy::authorize(config, &allowed.into_iter().collect::<Vec<_>>())?;
            }
            policy::show(config)
        }
    }
}
