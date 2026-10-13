use crate::utils::{
    config::Config,
    print::{error, identity, padding},
};
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
            error::print(format_args!("{error:#}"));
            ExitCode::FAILURE
        }
    }
}

fn list() -> Result<()> {
    let config = Config::load()?;
    let mut output = io::stdout().lock();

    padding::print(&mut output, |output| {
        if config.identities.is_empty() {
            return writeln!(output, "No identities configured");
        }

        writeln!(output, "Identities")?;

        for (index, (name, details)) in config.identities.iter().enumerate() {
            if index > 0 {
                writeln!(output)?;
            }
            identity::print(output, name, details)?;
        }

        Ok(())
    })?;
    Ok(())
}
