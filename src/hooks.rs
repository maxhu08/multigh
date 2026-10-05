use crate::{commands, config::Config, git, guard, process, settings};
use anyhow::{Context, Result, ensure};
use std::{
    env, fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use tempfile::NamedTempFile;

const MARKER: &str = "# Managed by mgh; existing hooks are preserved.";

const NAMES: &[&str] = &[
    "applypatch-msg",
    "pre-applypatch",
    "post-applypatch",
    "pre-commit",
    "pre-merge-commit",
    "prepare-commit-msg",
    "commit-msg",
    "post-commit",
    "pre-rebase",
    "pre-push",
    "post-checkout",
    "post-merge",
    "pre-receive",
    "update",
    "proc-receive",
    "post-receive",
    "post-update",
    "reference-transaction",
    "push-to-checkout",
    "pre-auto-gc",
    "post-rewrite",
    "sendemail-validate",
    "fsmonitor-watchman",
];

pub fn directory() -> Result<PathBuf> {
    Ok(settings::directory()?.join("hooks"))
}

pub fn compatible() -> Result<()> {
    let existing = git::global("core.hooksPath")?;

    ensure!(
        existing.is_empty() || Path::new(&existing) == directory()?,
        "A custom global hooks path is configured; it was left untouched. Integrate mgh manually before enabling protections."
    );

    Ok(())
}

pub fn install(config: &Config, global: &mut git::GlobalConfig<'_>) -> Result<()> {
    compatible()?;

    let directory = directory()?;

    fs::create_dir_all(&directory)?;
    git::private(&directory, true)?;

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

    if git::optional(&["rev-parse", "--git-dir"])?.is_some() {
        integrate()?;
    }

    Ok(())
}

fn executable(path: &Path) -> bool {
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

fn root() -> Result<PathBuf> {
    Ok(PathBuf::from(
        match git::optional(&["rev-parse", "--show-toplevel"])? {
            Some(root) => root,
            None => git::run(&["rev-parse", "--absolute-git-dir"])?,
        },
    ))
}

fn setting() -> Result<Option<(String, PathBuf)>> {
    let value = git::optional(&[
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
        } else if !executable(&path) {
            problems.push(format!("mgh hook is not executable: {name}"));
        } else if !fs::read_to_string(&path).is_ok_and(|source| source.contains(MARKER)) {
            problems.push(format!("mgh hook is unreadable or changed: {name}"));
        }
    }

    Ok(problems)
}

pub fn problems() -> Result<Vec<String>> {
    let root = root()?;
    let shared = fs::canonicalize(directory()?).ok();
    let mut problems = shared_problems()?;

    match setting()? {
        Some((_, path))
            if shared.is_some() && fs::canonicalize(root.join(&path)).ok() == shared => {}
        Some((scope, path)) => problems.insert(
            0,
            format!("{scope} hook path overrides mgh: {}", path.display()),
        ),
        None => problems.insert(0, "Git is not configured to use mgh's hooks".to_owned()),
    }

    Ok(problems)
}

pub fn installed() -> Result<bool> {
    Ok(problems()?.is_empty())
}

pub fn integrate() -> Result<bool> {
    if !shared_problems()?.is_empty() {
        return Ok(false);
    }

    let Some((scope, original)) = setting()? else {
        return Ok(false);
    };

    let shared = directory()?;

    if fs::canonicalize(root()?.join(&original)).ok() == Some(fs::canonicalize(&shared)?) {
        return installed();
    }

    if !["local", "worktree"].contains(&scope.as_str()) {
        return Ok(false);
    }

    let scope = format!("--{scope}");

    git::set(&scope, "mgh.originalHooksPath", &original.to_string_lossy())?;
    git::set(&scope, "core.hooksPath", &shared.to_string_lossy())?;

    installed()
}

fn original_directory(base: &Path) -> Result<Option<PathBuf>> {
    if let Some(path) = git::optional(&["config", "--path", "--get", "mgh.originalHooksPath"])? {
        let path = base.join(path);

        ensure!(
            fs::canonicalize(&path).ok() != fs::canonicalize(directory()?).ok() || !path.exists(),
            "Original hooks path points back to mgh's shared hooks"
        );

        return Ok(Some(path));
    }

    let common = git::optional(&["rev-parse", "--path-format=absolute", "--git-common-dir"])?
        .map(PathBuf::from)
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

pub fn detected() -> Result<Vec<(String, PathBuf, bool)>> {
    let root = root()?;
    let shared = fs::canonicalize(directory()?).ok();
    let overridden = setting()?
        .map(|(_, path)| root.join(path))
        .filter(|path| fs::canonicalize(path).ok() != shared);
    let Some(directory) = (match overridden {
        Some(directory) => Some(directory),
        None => original_directory(&root)?,
    }) else {
        return Ok(Vec::new());
    };

    let mut hooks = Vec::new();

    for &name in NAMES {
        if let Some(mut path) = preserved(&directory, name)? {
            let active = executable(&path);

            if directory.ends_with(".husky/_") {
                path = directory.parent().unwrap().join(name);
                if !path.is_file() {
                    continue;
                }
            }

            hooks.push((
                name.to_owned(),
                path.strip_prefix(&root).unwrap_or(&path).to_path_buf(),
                active,
            ));
        }
    }

    Ok(hooks)
}

fn original(name: &str, args: &[String], input: Option<&[u8]>) -> Result<()> {
    let Some(directory) = original_directory(&env::current_dir()?)? else {
        return Ok(());
    };

    let Some(path) = preserved(&directory, name)? else {
        return Ok(());
    };

    if !executable(&path) {
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

pub fn run(path: PathBuf, name: &str, args: &[String]) -> Result<()> {
    ensure!(NAMES.contains(&name), "Unknown Git hook: {name}");

    let mut updates = Vec::new();

    if name == "pre-push" {
        std::io::stdin().read_to_end(&mut updates)?;
    }

    if settings::protections()? {
        match name {
            "pre-commit" | "pre-merge-commit" => guard::check(&Config::load(path)?)?,
            "pre-push" => guard::push(&Config::load(path)?, std::str::from_utf8(&updates)?)?,
            "post-checkout"
                if args.len() == 3 && args[0].chars().all(|c| c == '0') && args[2] == "1" =>
            {
                commands::enter::run(path)?
            }
            _ => {}
        }
    }

    original(
        name,
        args,
        (name == "pre-push").then_some(updates.as_slice()),
    )
}
