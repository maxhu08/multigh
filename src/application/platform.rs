use anyhow::{Context, Result, bail};
#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, Output},
};

pub(super) fn home() -> Result<PathBuf> {
    env::var_os("HOME")
        .filter(|s| !s.is_empty())
        .or_else(|| env::var_os("USERPROFILE").filter(|s| !s.is_empty()))
        .map(PathBuf::from)
        .context("Cannot locate your home directory")
}

pub(super) fn location(variable: &str, fallback: &str) -> Result<PathBuf> {
    Ok(match env::var_os(variable).filter(|s| !s.is_empty()) {
        Some(p) => PathBuf::from(p),
        None => home()?.join(fallback),
    }
    .join("multigh"))
}

pub(super) fn private_dir(path: &Path) -> Result<()> {
    if path.is_dir() {
        return Ok(());
    }
    let mut b = fs::DirBuilder::new();
    b.recursive(true);
    #[cfg(unix)]
    b.mode(0o700);
    b.create(path)
        .with_context(|| format!("Cannot create {}", path.display()))
}

pub(super) fn output(command: &mut Command) -> Result<Output> {
    let description = format!("{command:?}");
    command
        .output()
        .with_context(|| format!("Cannot execute {description}"))
}

pub(super) fn successful(command: &mut Command) -> Result<Output> {
    let out = output(command)?;
    if !out.status.success() {
        bail!(
            "{command:?} failed ({}): {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(out)
}

pub(super) fn text(bytes: Vec<u8>) -> Result<String> {
    String::from_utf8(bytes).context("Invalid UTF-8 from external command")
}

pub(super) fn quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

pub(super) fn reject_symlink(path: &Path, description: &str) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_symlink() => bail!(
            "{description} {} is a symlink and was left intact; use --config with its target",
            path.display()
        ),
        Ok(_) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e).with_context(|| format!("Cannot inspect {}", path.display())),
    }
}

pub(super) fn write_private(path: &Path, bytes: &[u8], executable: bool) -> Result<bool> {
    reject_symlink(path, "Protected destination")?;
    if fs::read(path).ok().as_deref() == Some(bytes) {
        #[cfg(unix)]
        if executable {
            fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
        }
        return Ok(false);
    }
    private_dir(path.parent().context("No parent directory")?)?;
    let mut f = tempfile::NamedTempFile::new_in(path.parent().unwrap())?;
    f.write_all(bytes)?;
    f.as_file().sync_all()?;
    #[cfg(unix)]
    f.as_file()
        .set_permissions(fs::Permissions::from_mode(if executable {
            0o700
        } else {
            0o600
        }))?;
    f.persist(path)
        .map_err(|e| e.error)
        .with_context(|| format!("Cannot save {}", path.display()))?;
    Ok(true)
}

pub(super) fn executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
    }

    #[cfg(not(unix))]
    {
        path.is_file()
    }
}
