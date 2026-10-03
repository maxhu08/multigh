use crate::{config, git};
use anyhow::Result;
use std::{fs, path::PathBuf};

pub fn directory() -> Result<PathBuf> {
    Ok(config::directory("XDG_STATE_HOME", ".local/state")?.join("multigh"))
}

pub fn enabled(name: &str) -> Result<bool> {
    Ok(directory()?.join(format!("{name}-enabled")).is_file())
}

pub fn set(name: &str, enabled: bool) -> Result<()> {
    let directory = directory()?;
    let marker = directory.join(format!("{name}-enabled"));

    if enabled {
        fs::create_dir_all(&directory)?;
        git::private(&directory, true)?;
        fs::write(&marker, "")?;
    } else if marker.exists() {
        fs::remove_file(marker)?;
    }

    Ok(())
}
