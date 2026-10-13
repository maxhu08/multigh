use crate::utils::config::Config;
use anyhow::Result;
use std::{
    io::{self, Write},
    process::ExitCode,
};

#[derive(clap::Args)]
#[command(about = crate::descriptions::subcommands::identity::subcommands::LIST)]
pub struct ListArgs;

pub(super) fn run(_args: ListArgs) -> ExitCode {
    match list() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn list() -> Result<()> {
    let config = Config::load()?;
    let mut output = io::stdout().lock();

    if config.identities.is_empty() {
        writeln!(output, "No identities configured")?;
        return Ok(());
    }

    writeln!(output, "Identities")?;

    for (name, identity) in config.identities {
        writeln!(output, "  {name}")?;
        writeln!(output, "    GitHub username: {}", identity.username)?;
        writeln!(output, "    Commit name: {}", identity.commit.name)?;
        writeln!(output, "    Commit email: {}", identity.commit.email)?;
        writeln!(output)?;
    }

    Ok(())
}
