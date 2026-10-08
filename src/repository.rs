use crate::{git, utils::process};
use anyhow::{Context, Result};
use std::{collections::BTreeMap, env, path::PathBuf};

pub struct Repository {
    pub root: PathBuf,
    pub common_directory: PathBuf,
    directory: PathBuf,
}

impl Repository {
    fn git_directory() -> Result<Option<PathBuf>> {
        let result = process::capture(
            "git",
            &["rev-parse", "--path-format=absolute", "--absolute-git-dir"],
        )?;

        if result.status.code() == Some(128)
            && String::from_utf8_lossy(&result.stderr).contains("not a git repository")
        {
            return Ok(None);
        }

        process::successful_output("git", result).map(|path| Some(PathBuf::from(path)))
    }

    pub fn discover() -> Result<Option<Self>> {
        let Some(git_directory) = Self::git_directory()? else {
            return Ok(None);
        };
        let bare = git::run(&["rev-parse", "--is-bare-repository"])? == "true";
        let root = if bare {
            git_directory
        } else {
            PathBuf::from(git::run(&["rev-parse", "--show-toplevel"])?)
        };
        let common_directory =
            git::run(&["rev-parse", "--path-format=absolute", "--git-common-dir"])?;

        Ok(Some(Self {
            root,
            common_directory: PathBuf::from(common_directory),
            directory: env::current_dir()?,
        }))
    }

    pub fn require() -> Result<Self> {
        Self::discover()?
            .context("No Git repository found here. Enter a repository before running mgh repo.")
    }

    pub fn config_path(&self) -> PathBuf {
        self.common_directory.join("config")
    }

    pub fn run(&self, args: &[&str]) -> Result<String> {
        let directory = self.directory.to_string_lossy();
        let mut options = vec!["-C", &directory];
        options.extend(args);

        git::run(&options)
    }

    fn optional(&self, args: &[&str], missing_codes: &[i32]) -> Result<Option<String>> {
        let directory = self.directory.to_string_lossy();
        let mut options = vec!["-C", &directory];
        options.extend(args);

        process::optional("git", &options, missing_codes)
    }

    pub fn optional_config(&self, args: &[&str]) -> Result<Option<String>> {
        self.optional(args, &[1])
    }

    pub fn value(&self, key: &str) -> Result<Option<String>> {
        self.optional_config(&["config", "--get", key])
    }

    pub fn local(&self, key: &str) -> Result<Option<String>> {
        self.optional_config(&["config", "--local", "--get", key])
    }

    pub fn set(&self, scope: git::Scope, key: &str, value: &str) -> Result<()> {
        self.run(&["config", scope.argument()?, key, value])?;
        Ok(())
    }

    pub fn entries(&self, pattern: &str) -> Result<BTreeMap<String, Vec<String>>> {
        let values =
            self.optional_config(&["config", "--local", "--null", "--get-regexp", pattern])?;

        Ok(git::parse_entries(values.as_deref().unwrap_or_default()))
    }

    pub fn commit_exists(&self, oid: &str) -> Result<bool> {
        let object = format!("{oid}^{{commit}}");

        Ok(self
            .optional(&["cat-file", "-e", &object], &[1, 128])?
            .is_some())
    }
}
