use crate::{repository::Repository, storage};
use anyhow::Result;
use std::{fs, path::PathBuf};

pub fn directory() -> Result<PathBuf> {
    Ok(storage::directory("XDG_STATE_HOME", ".local/state")?.join("multigh"))
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Preference {
    Autoswitch,
    Verbose,
    Welcome,
}

impl Preference {
    fn marker(self) -> &'static str {
        match self {
            Self::Autoswitch => "autoswitch-enabled",
            Self::Verbose => "verbose-enabled",
            Self::Welcome => "welcome-enabled",
        }
    }
}

pub fn enabled(preference: Preference) -> Result<bool> {
    Ok(directory()?.join(preference.marker()).is_file())
}

pub fn set(preference: Preference, enabled: bool) -> Result<()> {
    set_marker(preference.marker(), enabled)
}

pub fn setup_complete() -> Result<bool> {
    Ok(directory()?.join("setup-complete-enabled").is_file())
}

pub fn mark_setup_complete() -> Result<()> {
    set_marker("setup-complete-enabled", true)
}

fn set_marker(name: &str, enabled: bool) -> Result<()> {
    let directory = directory()?;
    let marker = directory.join(name);

    if enabled {
        fs::create_dir_all(&directory)?;
        storage::private(&directory, true)?;
        fs::write(&marker, "")?;
    } else if marker.exists() {
        fs::remove_file(marker)?;
    }

    Ok(())
}

pub fn protections(repository: &Repository) -> Result<bool> {
    if repository.local("mgh.protections")?.is_none() {
        return Ok(true);
    }
    Ok(repository.run(&[
        "config",
        "--local",
        "--type=bool",
        "--get",
        "mgh.protections",
    ])? == "true")
}
