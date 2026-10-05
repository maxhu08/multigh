use super::{MARKER, NAMES, directory};
use crate::{repository::Repository, utils::storage};
use anyhow::Result;
use std::{fs, path::PathBuf};

pub(super) fn setting(repository: &Repository) -> Result<Option<(String, PathBuf)>> {
    let value = repository.optional_config(&[
        "config",
        "--null",
        "--show-scope",
        "--path",
        "--get",
        "core.hooksPath",
    ])?;

    Ok(value.and_then(|value| {
        let mut fields = value.split('\0');

        Some((fields.next()?.to_owned(), PathBuf::from(fields.next()?)))
    }))
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
        Some((_, path))
            if shared.is_some() && fs::canonicalize(repository.root.join(&path)).ok() == shared => {
        }
        Some((scope, path)) => problems.insert(
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
