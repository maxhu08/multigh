use super::platform;
use anyhow::{Context, Result};
use std::{fs, io, path::PathBuf};
#[derive(Clone, Copy)]
pub enum Preference {
    Verbose,
    Welcome,
}

impl Preference {
    pub fn name(self) -> &'static str {
        match self {
            Self::Verbose => "Verbose",
            Self::Welcome => "Welcome",
        }
    }

    fn marker(self) -> &'static str {
        match self {
            Self::Verbose => "verbose-enabled",
            Self::Welcome => "welcome-enabled",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Autoswitch {
    Off,
    Foreground,
    Background,
}

impl Autoswitch {
    pub fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Foreground => "foreground",
            Self::Background => "background",
        }
    }

    pub fn enabled(self) -> bool {
        self != Self::Off
    }
}

pub(super) fn autoswitch() -> Result<Autoswitch> {
    let path = state()?.join("autoswitch-mode");
    match fs::read_to_string(&path) {
        Ok(value) => match value.trim() {
            "off" => Ok(Autoswitch::Off),
            "foreground" => Ok(Autoswitch::Foreground),
            "background" => Ok(Autoswitch::Background),
            value => anyhow::bail!("Invalid autoswitch mode '{value}' in {}", path.display()),
        },
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Autoswitch::Off),
        Err(e) => Err(e).with_context(|| format!("Cannot read {}", path.display())),
    }
}

pub(super) fn set_autoswitch(mode: Autoswitch) -> Result<()> {
    platform::write_private(
        &state()?.join("autoswitch-mode"),
        format!("{}\n", mode.name()).as_bytes(),
        false,
    )?;
    Ok(())
}

pub(super) fn state() -> Result<PathBuf> {
    platform::location("XDG_STATE_HOME", ".local/state")
}

pub(super) fn get(pref: Preference) -> Result<bool> {
    exists(&state()?.join(pref.marker()))
}

fn exists(path: &std::path::Path) -> Result<bool> {
    match fs::metadata(path) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e).with_context(|| format!("Cannot read {}", path.display())),
    }
}

pub(super) fn set(pref: Preference, enabled: bool) -> Result<()> {
    let path = state()?.join(pref.marker());
    if enabled {
        platform::write_private(&path, b"", false)?;
    } else {
        match fs::remove_file(&path) {
            Ok(()) => (),
            Err(e) if e.kind() == io::ErrorKind::NotFound => (),
            Err(e) => return Err(e).with_context(|| format!("Cannot remove {}", path.display())),
        }
    }
    Ok(())
}

pub(super) fn finish_setup() -> Result<()> {
    let marker = state()?.join("setup-complete-enabled");
    if !exists(&marker)? {
        set(Preference::Verbose, true)?;
        platform::write_private(&marker, b"", false)?;
    }
    Ok(())
}
