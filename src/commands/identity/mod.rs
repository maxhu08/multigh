mod edit;
pub(super) mod new;

use crate::{cli::IdentityCommand, config::Config};
use anyhow::Result;
use std::path::PathBuf;

pub fn run(path: PathBuf, command: Option<IdentityCommand>) -> Result<()> {
    match command.unwrap_or(IdentityCommand::List) {
        IdentityCommand::List => super::status::identities(&Config::load(path)?),
        IdentityCommand::New { identity, fields } => {
            new::run(path, identity, fields.username, fields.email, fields.name)
        }
        IdentityCommand::Edit { identity, fields } => edit::run(path, &identity, Some(fields)),
        IdentityCommand::Remove { identity } => edit::run(path, &identity, None),
    }
}
