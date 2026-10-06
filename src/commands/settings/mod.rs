pub(super) mod autoswitch;
pub mod welcome;

use crate::{
    cli::{SettingsCommand, Toggle},
    repository::Repository,
    settings::{self, Preference},
    utils::output::{self, Color},
};
use anyhow::Result;
use std::path::PathBuf;

pub fn report(preference: Preference) -> Result<()> {
    let enabled = settings::enabled(preference)?;
    let (label, description) = match preference {
        Preference::Autoswitch => (
            "Autoswitch",
            "Switch to the sole allowed identity on repo entry; always ask when several are allowed.",
        ),
        Preference::Welcome => (
            "Welcome",
            "Show the active identity in the terminal greeting.",
        ),
        Preference::Verbose => (
            "Verbose",
            "Show allowed identities when entering a repo or starting a terminal there.",
        ),
    };

    if preference == Preference::Autoswitch {
        println!();
    }
    output::row(
        label,
        if enabled { "ON" } else { "OFF" },
        if enabled {
            Color::Changed
        } else {
            Color::Muted
        },
    );
    println!("  {description}\n");
    Ok(())
}

pub fn run(path: PathBuf, command: Option<SettingsCommand>) -> Result<()> {
    let (preference, state) = match command {
        Some(SettingsCommand::Autoswitch { state }) => (Preference::Autoswitch, state),
        Some(SettingsCommand::Verbose { state }) => (Preference::Verbose, state),
        Some(SettingsCommand::Welcome { state }) => (Preference::Welcome, state),
        None => {
            report(Preference::Autoswitch)?;
            report(Preference::Verbose)?;
            return report(Preference::Welcome);
        }
    };
    let enable_autoswitch =
        preference == Preference::Autoswitch && matches!(state, Some(Toggle::On));

    if let Some(state) = state {
        settings::set(preference, matches!(state, Toggle::On))?;
    }
    report(preference)?;

    if enable_autoswitch && let Some(repository) = Repository::discover()? {
        super::enter::enter(&repository, path)?;
    }

    Ok(())
}
