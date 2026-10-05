use crate::terminal;
use crate::{config, output, process};
use anyhow::{Context, Result};
use std::{path::PathBuf, process::Command};

pub fn run(args: &[String], config_path: Option<PathBuf>) -> Result<()> {
    let status = Command::new("git")
        .args(args)
        .status()
        .context("Could not run git")?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    if !terminal::interactive() {
        return Ok(());
    }

    let Some((prefix, directory)) = initialized_directory(args) else {
        return Ok(());
    };

    if let Err(error) = enter(&prefix, directory, config_path) {
        output::warning(&format!(
            "Git initialization succeeded, but identity selection could not run: {error:#}\nRun mgh repo allowed update in the new repository."
        ));
    }

    Ok(())
}

fn enter(prefix: &[&str], directory: &str, config_path: Option<PathBuf>) -> Result<()> {
    let executable = std::env::current_exe()?;
    let path = config::path(config_path)?;
    let alias = format!(
        "alias.mgh-init=!{} --config {} internal enter",
        process::quote(&executable.to_string_lossy()),
        process::quote(&path.to_string_lossy())
    );
    let mut args = prefix.to_vec();

    args.extend(["-C", directory, "-c", &alias, "mgh-init"]);
    process::interactive("git", &args)
}

fn initialized_directory(args: &[String]) -> Option<(Vec<&str>, &str)> {
    let mut index = 0;
    let mut prefix = Vec::new();

    while let Some(arg) = args.get(index) {
        match arg.as_str() {
            "init" => break,
            "-C" | "-c" | "--git-dir" | "--work-tree" | "--namespace" | "--config-env" => {
                prefix.push(arg.as_str());
                index += 1;
                prefix.push(args.get(index)?.as_str());
            }
            "--bare" => {}
            "-p"
            | "-P"
            | "--paginate"
            | "--no-pager"
            | "--no-replace-objects"
            | "--no-lazy-fetch"
            | "--no-optional-locks"
            | "--no-advice"
            | "--literal-pathspecs"
            | "--glob-pathspecs"
            | "--noglob-pathspecs"
            | "--icase-pathspecs" => prefix.push(arg.as_str()),
            value
                if [
                    "--git-dir=",
                    "--work-tree=",
                    "--namespace=",
                    "--config-env=",
                    "--exec-path=",
                    "--attr-source=",
                ]
                .iter()
                .any(|option| value.starts_with(option)) =>
            {
                prefix.push(arg.as_str())
            }
            _ => return None,
        }

        index += 1;
    }

    if args.get(index)?.as_str() != "init" {
        return None;
    }

    let mut directory = ".";
    index += 1;

    while let Some(arg) = args.get(index) {
        match arg.as_str() {
            "--" => {
                directory = args.get(index + 1).map_or(".", String::as_str);
                break;
            }
            "-h" | "--help" => return None,
            "-b" => {
                index += 1;
                args.get(index)?;
            }
            value
                if value.starts_with("--")
                    && !value.contains('=')
                    && [
                        "--initial-branch",
                        "--template",
                        "--separate-git-dir",
                        "--object-format",
                        "--ref-format",
                    ]
                    .iter()
                    .any(|option| option.starts_with(value)) =>
            {
                index += 1;
                args.get(index)?;
            }
            value if value.starts_with('-') => {}
            value => directory = value,
        }

        index += 1;
    }

    Some((prefix, directory))
}
