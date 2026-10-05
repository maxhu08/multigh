use anyhow::{Context, Result};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

pub fn private(path: &Path, directory: bool) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(
            path,
            fs::Permissions::from_mode(if directory { 0o700 } else { 0o600 }),
        )?;
    }

    #[cfg(not(unix))]
    let _ = (path, directory);

    Ok(())
}

pub fn directory(variable: &str, fallback: &str) -> Result<PathBuf> {
    if let Some(value) = env::var_os(variable).filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(value));
    }

    let home = env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .context("Cannot locate your home directory")?;

    Ok(PathBuf::from(home).join(fallback))
}

pub fn executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::{ffi::CString, os::unix::ffi::OsStrExt};

        path.is_file()
            && CString::new(path.as_os_str().as_bytes())
                .is_ok_and(|path| unsafe { libc::access(path.as_ptr(), libc::X_OK) == 0 })
    }

    #[cfg(not(unix))]
    path.is_file()
}
