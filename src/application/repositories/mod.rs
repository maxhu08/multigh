mod enforcement;
mod git;
mod hooks;
mod identity_rules;

use super::{
    accounts::{AccountQueries, SelectedAccount},
    identities::{Catalog, Identity, IdentitySelector},
    platform, preferences,
};
use anyhow::{Context, Result, bail, ensure};
pub(super) use git::GlobalWrites as Writes;
use git::{Git, GlobalWrites};
pub(super) use hooks::{HOOK_NAMES, HookReadiness};
use std::{collections::BTreeSet, path::PathBuf};

pub(super) struct Repository {
    git: Git,
    root: PathBuf,
    common: PathBuf,
}

#[derive(Clone, Debug)]
pub(super) struct StoredPermission(String);
impl StoredPermission {
    pub fn name(&self) -> &str {
        &self.0
    }
}

pub(super) struct PermissionSelection<'a> {
    identities: Vec<&'a Identity>,
}

impl<'a> PermissionSelection<'a> {
    pub fn resolve(catalog: &'a Catalog, names: &[String]) -> Result<Self> {
        let names = names
            .iter()
            .map(|s| s.to_ascii_lowercase())
            .collect::<BTreeSet<_>>();
        ensure!(!names.is_empty(), "Input required");
        let identities = names
            .iter()
            .map(|s| catalog.resolve(&IdentitySelector::new(s)))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { identities })
    }
}

impl Repository {
    pub fn discover() -> Result<Option<Self>> {
        let git = Git::here()?;
        let args = [
            "rev-parse",
            "--is-bare-repository",
            "--path-format=absolute",
            "--git-common-dir",
        ];
        let out = match platform::output(git.command().args(args)) {
            Ok(out) if out.status.success() => out,
            Ok(out)
                if String::from_utf8_lossy(&out.stderr)
                    .starts_with("fatal: not a git repository") =>
            {
                return Ok(None);
            }
            _ => {
                // Keep the original discovery diagnostics when the combined query fails.
                let out =
                    platform::output(git.command().args(["rev-parse", "--absolute-git-dir"]))?;
                if !out.status.success() {
                    let err = String::from_utf8_lossy(&out.stderr);
                    if err.starts_with("fatal: not a git repository") {
                        return Ok(None);
                    }
                    bail!(
                        "git repository discovery failed ({}): {}",
                        out.status,
                        err.trim()
                    );
                }
                git.run(args)?
            }
        };
        let details = git::checked_text(out)?;
        // The boolean comes first so paths containing newlines remain intact.
        let (bare, common) = details
            .split_once('\n')
            .context("Malformed Git repository discovery response")?;
        let common = PathBuf::from(common.trim());
        let root = if bare.trim() == "true" {
            PathBuf::from(git::checked_text(git.run(["rev-parse", "--absolute-git-dir"])?)?.trim())
        } else {
            PathBuf::from(git::checked_text(git.run(["rev-parse", "--show-toplevel"])?)?.trim())
        };
        Ok(Some(Self { git, root, common }))
    }

    pub fn require() -> Result<Self> {
        Self::discover()?
            .context("No Git repository found here. Enter a repository before running mgh repo.")
    }

    pub fn root(&self) -> &std::path::Path {
        &self.root
    }

    pub fn config(&self) -> PathBuf {
        self.common.join("config")
    }

    pub fn stored(&self) -> Result<Vec<StoredPermission>> {
        let names = self.git.values(&self.config(), "mgh.allowed-identity")?;
        if names.iter().any(String::is_empty) {
            return Ok(Vec::new());
        }
        Ok(names
            .into_iter()
            .map(|s| s.to_ascii_lowercase())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(StoredPermission)
            .collect())
    }

    pub fn allowed<'a>(&self, catalog: &'a Catalog) -> Result<Vec<&'a Identity>> {
        self.stored()?
            .iter()
            .map(|s| catalog.resolve(&IdentitySelector::new(s.name())))
            .collect()
    }

    pub fn protections(&self) -> Result<bool> {
        let config = self.config();
        let value = self
            .git
            .optional([
                std::ffi::OsStr::new("config"),
                std::ffi::OsStr::new("--file"),
                config.as_os_str(),
                std::ffi::OsStr::new("--type=bool"),
                std::ffi::OsStr::new("--get"),
                std::ffi::OsStr::new("mgh.protections"),
            ])
            .context("Invalid stored mgh.protections boolean; protections fail closed")?;
        match value {
            None => Ok(true),
            Some(v) => Ok(platform::text(v)?.trim() == "true"),
        }
    }

    pub fn disable(&self) -> Result<()> {
        self.git.local("mgh.protections", "false")
    }

    pub fn enable(
        &self,
        config: &std::path::Path,
        origin: &str,
        writes: &mut GlobalWrites,
    ) -> Result<HookReadiness> {
        let readiness = self.install_hooks(config, origin, writes)?;
        ensure!(
            readiness.healthy(),
            "Protections are not active: {}. Run mgh setup; overriding global/command settings need manual integration",
            readiness.problems.join("; ")
        );
        self.git.local("mgh.protections", "true")?;
        Ok(readiness)
    }

    pub fn effective_details(&self) -> Result<(Option<String>, Option<String>)> {
        self.git.details(None)
    }

    pub fn current(&self) -> Result<Option<String>> {
        self.git.value("--local", "mgh.current-identity")
    }

    fn save_names(&self, names: &[String]) -> Result<()> {
        self.git.unset("--local", "mgh.allowed-identity")?;
        if names.is_empty() {
            self.git.local("mgh.allowed-identity", "")?;
        } else {
            for name in names {
                self.git
                    .run(["config", "--local", "--add", "mgh.allowed-identity", name])?;
            }
        }
        Ok(())
    }

    pub fn authorize(
        &self,
        selection: PermissionSelection<'_>,
        selected: Option<&SelectedAccount>,
    ) -> Result<()> {
        let chosen = selection
            .identities
            .iter()
            .copied()
            .find(|i| selected.is_some_and(|s| s.login().eq_ignore_ascii_case(i.username())))
            .unwrap_or(selection.identities[0]);
        self.save_names(
            &selection
                .identities
                .iter()
                .map(|i| i.name().to_string())
                .collect::<Vec<_>>(),
        )?;
        self.apply(chosen)
    }

    pub fn revoke(
        &self,
        name: &str,
        catalog: Option<&Catalog>,
        selected: Option<&SelectedAccount>,
    ) -> Result<()> {
        let names = self
            .stored()?
            .into_iter()
            .filter(|n| !n.name().eq_ignore_ascii_case(name))
            .map(|n| n.0)
            .collect::<Vec<_>>();
        self.save_names(&names)?;
        if let Some(catalog) = catalog
            && !names.is_empty()
            && names.iter().all(|n| catalog.find(n).is_some())
        {
            self.apply(
                PermissionSelection::resolve(catalog, &names)?
                    .identities
                    .iter()
                    .copied()
                    .find(|i| {
                        selected.is_some_and(|s| s.login().eq_ignore_ascii_case(i.username()))
                    })
                    .unwrap_or(catalog.find(&names[0]).unwrap()),
            )?;
        }
        Ok(())
    }

    fn apply(&self, identity: &Identity) -> Result<()> {
        self.git.local("user.name", identity.commit_name())?;
        self.git.local("user.email", identity.email())?;
        self.git.local("mgh.current-identity", identity.name())
    }

    pub fn prepare_selection<'a>(
        &'a self,
        catalog: &Catalog,
        target: &'a Identity,
    ) -> Result<PreparedSelection<'a>> {
        let allowed = self.allowed(catalog)?;
        let apply = allowed.iter().any(|i| i.name() == target.name());
        let _ = self.effective_details()?;
        Ok(PreparedSelection {
            repo: self,
            target,
            apply,
        })
    }

    pub fn commit_check(&self, catalog: &Catalog, accounts: &AccountQueries) -> Result<()> {
        self.enforce_commit(catalog, accounts)
    }

    pub fn readiness(&self, config: &std::path::Path) -> Result<HookReadiness> {
        hooks::readiness(Some(self), Some(config))
    }
}

pub(super) struct PreparedSelection<'a> {
    repo: &'a Repository,
    target: &'a Identity,
    apply: bool,
}

impl PreparedSelection<'_> {
    pub fn apply(self) -> Result<()> {
        if self.apply {
            self.repo.apply(self.target)?;
        }
        Ok(())
    }
}

pub(super) fn refresh(catalog: &Catalog, origin: &str, writes: &mut GlobalWrites) -> Result<()> {
    identity_rules::refresh(catalog, origin, writes)
}

pub(super) fn global_defaults() -> Result<(Option<String>, Option<String>)> {
    Git::here()?.details(Some("--global"))
}

pub(super) fn set_global(
    identity: &Identity,
    origin: &str,
    writes: &mut GlobalWrites,
) -> Result<()> {
    let g = Git::here()?;
    writes.set(&g, "user.name", identity.commit_name(), origin)?;
    writes.set(&g, "user.email", identity.email(), origin)
}

pub(super) fn install(
    repo: Option<&Repository>,
    config: &std::path::Path,
    origin: &str,
    writes: &mut GlobalWrites,
) -> Result<HookReadiness> {
    hooks::install(repo, config, origin, writes)
}

pub(super) fn readiness(
    repo: Option<&Repository>,
    config: Option<&std::path::Path>,
) -> Result<HookReadiness> {
    hooks::readiness(repo, config)
}

pub(super) fn global_path(writes: &GlobalWrites) -> Result<String> {
    writes.path(&Git::here()?)
}

pub(super) fn new_writes() -> GlobalWrites {
    GlobalWrites::default()
}

pub(super) fn hook(
    name: &str,
    args: &[std::ffi::OsString],
    config: &dyn Fn() -> Result<PathBuf>,
    accounts: &AccountQueries,
    on_initial: &mut dyn FnMut() -> Result<()>,
) -> Result<()> {
    hooks::execute(name, args, config, accounts, on_initial)
}

pub(super) fn preflight_hooks() -> Result<()> {
    hooks::preflight()
}

pub(super) fn git_version() -> Result<String> {
    git::checked_text(Git::here()?.run(["--version"])?)
}

pub(super) fn state_identity(identity: &Identity) -> Result<PathBuf> {
    Ok(preferences::state()?
        .join("identities")
        .join(format!("git-{}.conf", identity.name())))
}
