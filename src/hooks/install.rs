use super::{MARKER, NAMES, directory, readiness};
use crate::{
    config::Config,
    git,
    repository::Repository,
    utils::{process, storage},
};
use anyhow::{Result, ensure};
use std::{env, fs, io::Write, path::Path};
use tempfile::NamedTempFile;

pub fn compatible() -> Result<()> {
    let existing = git::global::value("core.hooksPath")?;

    ensure!(
        existing.is_empty() || Path::new(&existing) == directory()?,
        "A custom global hooks path is configured; it was left untouched. Integrate mgh manually before enabling protections."
    );

    Ok(())
}

pub fn run(
    config: &Config,
    repository: Option<&Repository>,
    global: &mut git::global::Writer<'_>,
) -> Result<()> {
    compatible()?;

    let directory = directory()?;

    fs::create_dir_all(&directory)?;
    storage::private(&directory, true)?;

    for &name in NAMES {
        let path = directory.join(name);

        ensure!(
            !path.exists() || fs::read_to_string(&path)?.contains(MARKER),
            "Existing hook was left untouched: {}",
            path.display()
        );
        ensure!(
            !fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.file_type().is_symlink()),
            "Hook is a symlink: {}",
            path.display()
        );
    }

    let launcher = process::quote(&env::current_exe()?.to_string_lossy());
    let configuration = process::quote(&config.path.to_string_lossy());

    for &name in NAMES {
        let command = format!("{launcher} --config {configuration} internal hook {name} \"$@\"");
        let body = if name == "post-checkout" {
            format!("if [ -t 1 ]; then\n    exec {command} 0<&1\nfi\nexec {command}\n")
        } else {
            format!("exec {command}\n")
        };

        let mut temporary = NamedTempFile::new_in(&directory)?;
        write!(temporary, "#!/bin/sh\n{MARKER}\n{body}")?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            temporary
                .as_file()
                .set_permissions(fs::Permissions::from_mode(0o755))?;
        }

        temporary.persist(directory.join(name))?;
    }

    global.set("core.hooksPath", &directory.to_string_lossy())?;

    if let Some(repository) = repository {
        integrate(repository)?;
    }

    Ok(())
}

pub fn integrate(repository: &Repository) -> Result<bool> {
    if !readiness::shared_problems()?.is_empty() {
        return Ok(false);
    }

    let Some(readiness::HookSetting {
        scope,
        path: original,
    }) = readiness::setting(repository)?
    else {
        return Ok(false);
    };

    let shared = directory()?;

    if fs::canonicalize(repository.root.join(&original)).ok() == Some(fs::canonicalize(&shared)?) {
        return readiness::installed(repository);
    }

    if !matches!(scope, git::Scope::Local | git::Scope::Worktree) {
        return Ok(false);
    }

    repository.set(scope, "mgh.originalHooksPath", &original.to_string_lossy())?;
    repository.set(scope, "core.hooksPath", &shared.to_string_lossy())?;

    readiness::installed(repository)
}
