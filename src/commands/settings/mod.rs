pub(super) mod autoswitch;
pub mod welcome;

use crate::{
    cli::{SettingsCommand, Toggle},
    output::{self, Color},
    settings,
};
use anyhow::Result;
use std::path::PathBuf;

pub fn report(name: &str) -> Result<()> {
    let enabled = settings::enabled(name)?;
    let (label, description) = match name {
        "autoswitch" => (
            "Autoswitch",
            "Switch to the sole allowed identity on repo entry; always ask when several are allowed.",
        ),
        "welcome" => (
            "Welcome",
            "Show the active identity in the terminal greeting.",
        ),
        _ => (
            "Verbose",
            "Show allowed identities when entering a repo or starting a terminal there.",
        ),
    };
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
    let (name, state) = match command {
        Some(SettingsCommand::Autoswitch { state }) => ("autoswitch", state),
        Some(SettingsCommand::Verbose { state }) => ("verbose", state),
        Some(SettingsCommand::Welcome { state }) => ("welcome", state),
        None => {
            report("autoswitch")?;
            report("verbose")?;
            return report("welcome");
        }
    };
    let enable_autoswitch = name == "autoswitch" && matches!(state, Some(Toggle::On));

    if let Some(state) = state {
        settings::set(name, matches!(state, Toggle::On))?;
    }
    report(name)?;

    if enable_autoswitch {
        super::enter::run(path)?;
    }

    Ok(())
}
