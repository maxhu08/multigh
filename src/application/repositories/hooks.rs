use super::{AccountQueries, Catalog, Git, GlobalWrites, Repository, git, platform, preferences};
use anyhow::{Context, Result, ensure};
use std::{
    ffi::OsString,
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

pub(in crate::application) const HOOK_NAMES: [&str; 23] = [
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
const MARKER: &str = "# Managed by mgh; existing hooks are preserved.";
pub(in crate::application) struct HookReadiness {
    pub problems: Vec<String>,
    pub project_hooks: Vec<String>,
}

impl HookReadiness {
    pub fn healthy(&self) -> bool {
        self.problems.is_empty()
    }
}

fn directory() -> Result<PathBuf> {
    Ok(preferences::state()?.join("hooks"))
}

pub(super) fn preflight() -> Result<()> {
    let g = Git::here()?;
    let shared = directory()?;
    if let Some(path) = g.value("--global", "core.hooksPath")? {
        let resolved = if let Some(s) = path.strip_prefix("~/") {
            platform::home()?.join(s)
        } else {
            PathBuf::from(&path)
        };
        ensure!(
            git::equivalent(&resolved, &shared),
            "Unsupported custom global core.hooksPath {path}; it was left intact. \
             Configure manual integration before running mgh setup"
        );
    }
    for name in HOOK_NAMES {
        let path = shared.join(name);
        platform::reject_symlink(&path, "Managed hook destination")?;
        match fs::read_to_string(&path) {
            Ok(text) => ensure!(
                text.lines().any(|l| l == MARKER),
                "Unrecognized managed hook destination {}; it was left intact",
                path.display()
            ),
            Err(e) if e.kind() == io::ErrorKind::NotFound => (),
            Err(e) => {
                return Err(e).with_context(|| {
                    format!(
                        "Cannot read managed hook {}; it was left intact",
                        path.display()
                    )
                });
            }
        }
    }
    Ok(())
}

pub(super) fn install(
    repo: Option<&Repository>,
    config: &Path,
    origin: &str,
    writes: &mut GlobalWrites,
) -> Result<HookReadiness> {
    preflight()?;
    let shared = directory()?;
    for name in HOOK_NAMES {
        platform::write_private(&shared.join(name), launcher(name, config)?.as_bytes(), true)?;
    }
    let g = Git::here()?;
    // A local/worktree exception is repaired in the same scope before setting the global default.
    if let Some(repo) = repo {
        repo.repair_override(&shared)?;
    }
    writes.set(&g, "core.hooksPath", &shared.to_string_lossy(), origin)?;
    readiness(repo, Some(config))
}

impl Repository {
    fn repair_override(&self, shared: &Path) -> Result<()> {
        if let Some(setting) = self.git.hook_setting()?
            && matches!(setting.scope.as_str(), "local" | "worktree")
        {
            let path = git::resolve_path(&self.root, &setting.value);
            if !git::equivalent(&path, shared) {
                let scope = if setting.scope == "worktree" {
                    "--worktree"
                } else {
                    "--local"
                };
                self.git
                    .scoped(scope, "mgh.originalHooksPath", &setting.value)?;
                self.git
                    .scoped(scope, "core.hooksPath", &shared.to_string_lossy())?;
            }
        }
        Ok(())
    }

    pub fn install_hooks(
        &self,
        config: &Path,
        origin: &str,
        writes: &mut GlobalWrites,
    ) -> Result<HookReadiness> {
        install(Some(self), config, origin, writes)
    }

    fn project_directory(&self) -> Result<PathBuf> {
        let saved = self
            .git
            .optional(["config", "--path", "--get", "mgh.originalHooksPath"])?
            .map(|b| platform::text(b).map(|s| s.trim_end_matches('\n').to_string()))
            .transpose()?;
        match saved {
            Some(p) => Ok(git::resolve_path(&self.root, &p)),
            None => Ok(self.common.join("hooks")),
        }
    }
}

fn launcher(name: &str, config: &Path) -> Result<String> {
    Ok(format!(
        "#!/bin/sh\n{MARKER}\nexec {} --config {} internal hook {} -- \"$@\"\n",
        platform::quote(&std::env::current_exe()?.to_string_lossy()),
        platform::quote(&config.to_string_lossy()),
        platform::quote(name)
    ))
}

fn valid_launcher(text: &str, name: &str, config: Option<&Path>) -> Result<bool> {
    if let Some(config) = config {
        return Ok(text == launcher(name, config)?);
    }
    Ok(false)
}

pub(super) fn readiness(repo: Option<&Repository>, config: Option<&Path>) -> Result<HookReadiness> {
    let mut problems = vec![];
    let mut project_hooks = vec![];
    let shared = directory()?;
    for name in HOOK_NAMES {
        let path = shared.join(name);
        match fs::symlink_metadata(&path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                problems.push(format!("Missing launcher {}", path.display()))
            }
            Err(e) => problems.push(format!("Unreadable launcher {}: {e}", path.display())),
            Ok(m) if m.file_type().is_symlink() => {
                problems.push(format!("Changed launcher {} (symlink)", path.display()))
            }
            Ok(_) => match fs::read_to_string(&path) {
                Ok(s) if valid_launcher(&s, name, config)? => {
                    if !platform::executable(&path) {
                        problems.push(format!("Nonexecutable launcher {}", path.display()));
                    }
                }
                Ok(_) => problems.push(format!("Changed launcher {}", path.display())),
                Err(e) => problems.push(format!("Unreadable launcher {}: {e}", path.display())),
            },
        }
    }
    let g = Git::here()?;
    let setting = g.hook_setting()?;
    match setting {
        Some(s) => {
            let base = repo.map(|r| r.root.as_path()).unwrap_or(Path::new("."));
            if !git::equivalent(&git::resolve_path(base, &s.value), &shared) {
                problems.push(format!(
                    "Effective core.hooksPath override ({}; {}): {}",
                    s.scope, s.origin, s.value
                ));
            }
        }
        None => problems.push("Managed core.hooksPath is not configured".into()),
    }
    if let Some(repo) = repo {
        let project = repo.project_directory()?;
        if git::equivalent(&project, &shared) {
            problems.push(format!("Preserved-hook recursion at {}", project.display()));
        } else {
            for name in HOOK_NAMES {
                let hook = project.join(name);
                if hook.exists() {
                    let meaningful = if project.ends_with(".husky/_") {
                        project.parent().unwrap().join(name)
                    } else {
                        hook.clone()
                    };
                    project_hooks.push(format!(
                        "{}{}",
                        meaningful.display(),
                        if platform::executable(&hook) {
                            ""
                        } else {
                            " (skipped: nonexecutable)"
                        }
                    ));
                }
            }
        }
    }
    Ok(HookReadiness {
        problems,
        project_hooks,
    })
}

pub(super) fn execute(
    name: &str,
    args: &[OsString],
    config: &dyn Fn() -> Result<PathBuf>,
    accounts: &AccountQueries,
    on_initial: &mut dyn FnMut() -> Result<()>,
) -> Result<()> {
    ensure!(HOOK_NAMES.contains(&name), "Unknown Git hook: {name}");
    let input = if name == "pre-push" {
        let mut bytes = vec![];
        io::stdin().read_to_end(&mut bytes)?;
        Some(bytes)
    } else {
        None
    };
    let repo = Repository::discover()?;
    let protected = match &repo {
        Some(repo) => repo.protections()?,
        None => true,
    };
    if protected {
        match name {
            "pre-commit" | "pre-merge-commit" => {
                let catalog = Catalog::load(config()?, false)?;
                if let Some(repo) = &repo {
                    repo.enforce_commit(&catalog, accounts)?;
                }
            }
            "pre-push" => {
                let repo = repo
                    .as_ref()
                    .context("Protected pre-push requires a Git repository")?;
                let catalog = Catalog::load(config()?, false)?;
                repo.enforce_push(&catalog, input.as_deref().unwrap(), accounts)?;
            }
            "post-checkout"
                if args.len() == 3
                    && args[0].to_str().is_some_and(|s| {
                        (s.len() == 40 || s.len() == 64) && s.bytes().all(|b| b == b'0')
                    })
                    && args[2] == "1" =>
            {
                on_initial()?
            }
            _ => (),
        }
    }
    let directory = match &repo {
        Some(r) => r.project_directory()?,
        None => {
            let git_dir = std::env::var_os("GIT_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(".git"));
            git_dir.join("hooks")
        }
    };
    let shared = directory_shared()?;
    ensure!(
        !git::equivalent(&directory, &shared),
        "Preserved-hook recursion at {}",
        directory.display()
    );
    let hook = directory.join(name);
    if !platform::executable(&hook) {
        return Ok(());
    }
    if let Ok(target) = hook.canonicalize() {
        ensure!(
            !target.starts_with(shared.canonicalize().unwrap_or(shared)),
            "Preserved-hook recursion at {}",
            hook.display()
        );
    }
    let mut command = Command::new(&hook);
    command
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    if let Some(repo) = repo {
        command.current_dir(repo.root);
    }
    if let Some(bytes) = input {
        let mut child = command
            .stdin(Stdio::piped())
            .spawn()
            .with_context(|| format!("Cannot execute {}", hook.display()))?;
        let mut stdin = child.stdin.take().unwrap();
        let writer = std::thread::spawn(move || stdin.write_all(&bytes));
        let status = child.wait()?;
        let written = writer
            .join()
            .map_err(|_| anyhow::anyhow!("Hook input writer failed"))?;
        ensure!(
            status.success(),
            "Existing {name} hook rejected this operation ({status})"
        );
        if let Err(e) = written
            && e.kind() != io::ErrorKind::BrokenPipe
        {
            return Err(e).context("Cannot replay project hook input");
        }
    } else {
        let status = command
            .stdin(Stdio::inherit())
            .status()
            .with_context(|| format!("Cannot execute {}", hook.display()))?;
        ensure!(
            status.success(),
            "Existing {name} hook rejected this operation ({status})"
        );
    }
    Ok(())
}

fn directory_shared() -> Result<PathBuf> {
    directory()
}
