use super::{MARKER, NAMES, directory};
use crate::{git::Scope, repository::Repository, utils::storage};
use anyhow::{Context, Result};
use std::{fs, path::PathBuf};

pub(super) struct HookSetting {
    pub scope: Scope,
    pub path: PathBuf,
}

pub(super) fn setting(repository: &Repository) -> Result<Option<HookSetting>> {
    let value = repository.optional_config(&[
        "config",
        "--null",
        "--show-scope",
        "--path",
        "--get",
        "core.hooksPath",
    ])?;

    value
        .map(|value| {
            let (scope, path) = value
                .split_once('\0')
                .context("Invalid Git hooks path setting")?;
            Ok(HookSetting {
                scope: Scope::parse(scope)?,
                path: PathBuf::from(path.trim_end_matches('\0')),
            })
        })
        .transpose()
}

pub fn shared_problems() -> Result<Vec<String>> {
    let directory = directory()?;
    let mut problems = Vec::new();

    for &name in NAMES {
        let path = directory.join(name);

        if !path.is_file() {
            problems.push(format!("Missing mgh hook: {name}"));
        } else if !storage::executable(&path) {
            problems.push(format!("mgh hook is not executable: {name}"));
        } else if !fs::read_to_string(&path).is_ok_and(|source| source.contains(MARKER)) {
            problems.push(format!("mgh hook is unreadable or changed: {name}"));
        }
    }

    Ok(problems)
}

pub fn problems(repository: &Repository) -> Result<Vec<String>> {
    let shared = fs::canonicalize(directory()?).ok();
    let mut problems = shared_problems()?;

    match setting(repository)? {
        Some(HookSetting { path, .. })
            if shared.is_some() && fs::canonicalize(repository.root.join(&path)).ok() == shared => {
        }
        Some(HookSetting { scope, path }) => problems.insert(
            0,
            format!("{scope} hook path overrides mgh: {}", path.display()),
        ),
        None => problems.insert(0, "Git is not configured to use mgh's hooks".to_owned()),
    }

    Ok(problems)
}

pub fn installed(repository: &Repository) -> Result<bool> {
    Ok(problems(repository)?.is_empty())
}
