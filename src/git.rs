pub mod global;
pub mod identity_files;

use crate::utils::process;
use anyhow::{Result, bail};
use std::{collections::BTreeMap, fmt};

#[derive(Clone, Copy)]
pub enum Scope {
    System,
    Global,
    Local,
    Worktree,
    Command,
}

impl Scope {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "system" => Ok(Self::System),
            "global" => Ok(Self::Global),
            "local" => Ok(Self::Local),
            "worktree" => Ok(Self::Worktree),
            "command" => Ok(Self::Command),
            _ => bail!("Unknown Git configuration scope: {value}"),
        }
    }

    pub fn argument(self) -> Result<&'static str> {
        Ok(match self {
            Self::System => "--system",
            Self::Global => "--global",
            Self::Local => "--local",
            Self::Worktree => "--worktree",
            Self::Command => bail!("Command Git configuration has no persistent scope"),
        })
    }
}

impl fmt::Display for Scope {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::System => "system",
            Self::Global => "global",
            Self::Local => "local",
            Self::Worktree => "worktree",
            Self::Command => "command",
        })
    }
}

pub fn run(args: &[&str]) -> Result<String> {
    process::run("git", args)
}

pub fn optional_config(args: &[&str]) -> Result<Option<String>> {
    process::optional("git", args, &[1])
}

pub fn entries(scope: Option<Scope>, pattern: &str) -> Result<BTreeMap<String, Vec<String>>> {
    let mut args = vec!["config"];

    if let Some(scope) = scope {
        args.push(scope.argument()?);
    }

    args.extend(["--null", "--get-regexp", pattern]);

    Ok(parse_entries(
        optional_config(&args)?.as_deref().unwrap_or_default(),
    ))
}

pub(crate) fn parse_entries(source: &str) -> BTreeMap<String, Vec<String>> {
    let mut values: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for entry in source.split('\0') {
        if let Some((key, value)) = entry.split_once('\n') {
            values
                .entry(key.to_owned())
                .or_default()
                .push(value.to_owned());
        }
    }

    values
}
