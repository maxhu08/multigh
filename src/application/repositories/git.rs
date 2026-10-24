use crate::application::platform;
use anyhow::{Context, Result, bail};
use std::{
    collections::BTreeMap,
    ffi::OsStr,
    path::{Path, PathBuf},
    process::{Command, Output},
};

pub(super) struct Git {
    cwd: PathBuf,
}

impl Git {
    pub fn here() -> Result<Self> {
        Ok(Self {
            cwd: std::env::current_dir()?,
        })
    }

    pub fn command(&self) -> Command {
        let mut c = Command::new("git");
        c.current_dir(&self.cwd);
        c
    }

    pub fn run<I, S>(&self, args: I) -> Result<Output>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        platform::successful(self.command().args(args))
    }

    pub fn optional<I, S>(&self, args: I) -> Result<Option<Vec<u8>>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut cmd = self.command();
        cmd.args(args);
        let out = platform::output(&mut cmd)?;
        if out.status.success() {
            Ok(Some(out.stdout))
        } else if out.status.code() == Some(1) && out.stderr.is_empty() {
            Ok(None)
        } else {
            bail!(
                "{cmd:?} failed ({}): {}",
                out.status,
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
    }

    pub fn value(&self, scope: &str, key: &str) -> Result<Option<String>> {
        self.optional(["config", scope, "--includes", "--get", key])?
            .map(|b| platform::text(b).map(|s| s.trim_end_matches('\n').to_string()))
            .transpose()
    }

    pub fn values(&self, file: &Path, key: &str) -> Result<Vec<String>> {
        let out = self.optional([
            OsStr::new("config"),
            OsStr::new("--file"),
            file.as_os_str(),
            OsStr::new("--null"),
            OsStr::new("--get-all"),
            OsStr::new(key),
        ])?;
        split_values(out)
    }

    pub fn local(&self, key: &str, value: &str) -> Result<()> {
        self.run(["config", "--local", "--replace-all", key, value])?;
        Ok(())
    }

    pub fn scoped(&self, scope: &str, key: &str, value: &str) -> Result<()> {
        self.run(["config", scope, "--replace-all", key, value])?;
        Ok(())
    }

    pub fn unset(&self, scope: &str, key: &str) -> Result<()> {
        let mut c = self.command();
        c.args(["config", scope, "--unset-all", key]);
        let out = platform::output(&mut c)?;
        if !out.status.success() && out.status.code() != Some(5) {
            bail!(
                "git config failed: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        Ok(())
    }

    pub fn details(&self, scope: Option<&str>) -> Result<(Option<String>, Option<String>)> {
        let mut args = vec!["config"];
        args.extend(scope);
        args.extend([
            "--includes",
            "--null",
            "--get-regexp",
            "^user\\.(name|email)$",
        ]);
        let mut values = self.config_map(args)?;
        let mut last = |key| {
            values
                .remove(key)
                .and_then(|mut entries| entries.pop())
                .map(|value| value.trim_end_matches('\n').to_string())
        };
        Ok((last("user.name"), last("user.email")))
    }

    pub fn global_values(&self, key: &str) -> Result<Vec<String>> {
        split_values(self.optional([
            "config",
            "--global",
            "--includes",
            "--null",
            "--get-all",
            key,
        ])?)
    }

    pub fn config_map<I, S>(&self, args: I) -> Result<BTreeMap<String, Vec<String>>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut values = BTreeMap::<String, Vec<String>>::new();
        if let Some(bytes) = self.optional(args)? {
            for entry in platform::text(bytes)?.split_terminator('\0') {
                let (key, value) = entry.split_once('\n').unwrap_or((entry, ""));
                values.entry(key.into()).or_default().push(value.into());
            }
        }
        Ok(values)
    }

    pub fn hook_setting(&self) -> Result<Option<Setting>> {
        let Some(bytes) = self.optional([
            "config",
            "--includes",
            "--null",
            "--show-scope",
            "--show-origin",
            "--path",
            "--get",
            "core.hooksPath",
        ])?
        else {
            return Ok(None);
        };
        let text = platform::text(bytes)?;
        let fields = text.split_terminator('\0').collect::<Vec<_>>();
        if fields.len() != 3 {
            bail!("Malformed Git core.hooksPath response: {text:?}");
        }
        Ok(Some(Setting {
            scope: fields[0].into(),
            origin: fields[1].into(),
            value: fields[2].into(),
        }))
    }
}

fn split_values(out: Option<Vec<u8>>) -> Result<Vec<String>> {
    match out {
        None => Ok(vec![]),
        Some(b) => Ok(platform::text(b)?
            .split_terminator('\0')
            .map(str::to_string)
            .collect()),
    }
}

pub(super) struct Setting {
    pub scope: String,
    pub origin: String,
    pub value: String,
}

pub(in crate::application) struct GlobalChange {
    pub key: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

#[derive(Default)]
pub(in crate::application) struct GlobalWrites {
    pub completed: Vec<GlobalChange>,
}

impl GlobalWrites {
    pub(super) fn set(&mut self, git: &Git, key: &str, value: &str, origin: &str) -> Result<()> {
        let values = git.global_values(key)?;
        self.set_values(git, key, value, origin, values)
    }

    pub(super) fn set_values(
        &mut self,
        git: &Git,
        key: &str,
        value: &str,
        origin: &str,
        values: Vec<String>,
    ) -> Result<()> {
        if !values.is_empty() && values.iter().all(|v| v == value) {
            return Ok(());
        }
        git.run([
            "config",
            "--global",
            "--replace-all",
            "--comment",
            &format!("Generated by multigh ({origin})"),
            key,
            value,
        ])?;
        self.completed.push(GlobalChange {
            key: key.into(),
            before: if values.is_empty() {
                None
            } else {
                Some(values.join(", "))
            },
            after: Some(value.into()),
        });
        Ok(())
    }

    pub(super) fn remove_value(&mut self, git: &Git, key: &str, value: &str) -> Result<()> {
        git.run([
            "config",
            "--global",
            "--fixed-value",
            "--unset-all",
            key,
            value,
        ])?;
        self.completed.push(GlobalChange {
            key: key.into(),
            before: Some(value.into()),
            after: None,
        });
        Ok(())
    }

    pub(super) fn path(&self, git: &Git) -> Result<String> {
        let text = platform::text(git.run(["var", "GIT_CONFIG_GLOBAL"])?.stdout)?;
        let paths = text
            .lines()
            .filter(|p| Path::new(p).is_file())
            .collect::<Vec<_>>();
        if paths.is_empty() {
            bail!(
                "Git did not select an existing global configuration file: {}",
                text.trim()
            );
        }
        Ok(paths.join(", "))
    }
}

pub(super) fn resolve_path(base: &Path, path: &str) -> PathBuf {
    let p = PathBuf::from(path);
    if p.is_absolute() { p } else { base.join(p) }
}

pub(super) fn equivalent(a: &Path, b: &Path) -> bool {
    if a == b {
        true
    } else {
        match (a.canonicalize(), b.canonicalize()) {
            (Ok(a), Ok(b)) => a == b,
            _ => false,
        }
    }
}

pub(super) fn checked_text(out: Output) -> Result<String> {
    platform::text(out.stdout).context("Invalid Git response")
}
