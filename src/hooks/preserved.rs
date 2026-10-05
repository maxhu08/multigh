use super::{NAMES, readiness};
use crate::{git, repository::Repository, storage};
use anyhow::{Context, Result, ensure};
use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

fn directory(repository: Option<&Repository>, base: &Path) -> Result<Option<PathBuf>> {
    let args = ["config", "--path", "--get", "mgh.originalHooksPath"];
    let original = match repository {
        Some(repository) => repository.optional_config(&args)?,
        None => git::optional_config(&args)?,
    };

    if let Some(path) = original {
        let path = base.join(path);

        ensure!(
            fs::canonicalize(&path).ok() != fs::canonicalize(super::directory()?).ok()
                || !path.exists(),
            "Original hooks path points back to mgh's shared hooks"
        );

        return Ok(Some(path));
    }

    let common = repository
        .map(|repository| repository.common_directory.clone())
        .or_else(|| {
            env::var_os("GIT_COMMON_DIR")
                .or_else(|| env::var_os("GIT_DIR"))
                .map(PathBuf::from)
        });
    let cwd = env::current_dir()?;

    Ok(common.map(|path| cwd.join(path).join("hooks")))
}

fn preserved(directory: &Path, name: &str) -> Result<Option<PathBuf>> {
    let mut path = directory.join(name);

    if !path.is_file() {
        return Ok(None);
    }

    let source = fs::read(&path)?;

    if String::from_utf8_lossy(&source).contains("# Managed by mgh;")
        || String::from_utf8_lossy(&source).contains("# Managed by ghguard;")
    {
        let old = directory.join(format!("{name}.before-ghguard"));
        path = if old.exists() {
            old
        } else {
            directory.join(format!("{name}.before-mgh"))
        };
    }
    Ok(path.is_file().then_some(path))
}

pub struct DetectedHook {
    pub name: String,
    pub path: PathBuf,
    pub executable: bool,
}

pub fn detected(repository: &Repository) -> Result<Vec<DetectedHook>> {
    let root = &repository.root;
    let shared = fs::canonicalize(super::directory()?).ok();
    let overridden = readiness::setting(repository)?
        .map(|(_, path)| root.join(path))
        .filter(|path| fs::canonicalize(path).ok() != shared);
    let Some(directory) = (match overridden {
        Some(directory) => Some(directory),
        None => directory(Some(repository), root)?,
    }) else {
        return Ok(Vec::new());
    };

    let mut hooks = Vec::new();

    for &name in NAMES {
        if let Some(mut path) = preserved(&directory, name)? {
            let active = storage::executable(&path);

            if directory.ends_with(".husky/_") {
                path = directory.parent().unwrap().join(name);
                if !path.is_file() {
                    continue;
                }
            }

            hooks.push(DetectedHook {
                name: name.to_owned(),
                path: path.strip_prefix(root).unwrap_or(&path).to_path_buf(),
                executable: active,
            });
        }
    }

    Ok(hooks)
}

pub fn forward(
    repository: Option<&Repository>,
    name: &str,
    args: &[String],
    input: Option<&[u8]>,
) -> Result<()> {
    let Some(directory) = directory(repository, &env::current_dir()?)? else {
        return Ok(());
    };

    let Some(path) = preserved(&directory, name)? else {
        return Ok(());
    };

    if !storage::executable(&path) {
        return Ok(());
    }

    let mut command = Command::new(&path);
    command
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    if input.is_some() {
        command.stdin(Stdio::piped());
    }

    let mut child = command
        .spawn()
        .with_context(|| format!("Run existing hook {}", path.display()))?;
    let written = input.map(|input| child.stdin.take().unwrap().write_all(input));

    ensure!(
        child.wait()?.success(),
        "Existing {name} hook rejected this operation"
    );

    if let Some(Err(error)) = written
        && error.kind() != std::io::ErrorKind::BrokenPipe
    {
        return Err(error.into());
    }

    Ok(())
}
