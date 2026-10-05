use crate::{
    config::Config,
    git, github, hooks,
    output::{self, Color},
    policy, process,
};
use anyhow::{Result, ensure};
use std::path::PathBuf;

pub fn run(path: PathBuf) -> Result<()> {
    output::section("multigh diagnostics");
    output::row("Accounts", &path.to_string_lossy(), Color::Muted);
    let mut healthy = true;
    for program in ["git", "gh"] {
        match process::run(program, &["--version"]) {
            Ok(version) => output::row(
                program,
                version.lines().next().unwrap_or("available"),
                Color::Changed,
            ),
            Err(error) => {
                healthy = false;
                output::warning(&format!("{error:#}"));
            }
        }
    }
    if let Ok(version) = git::run(&["--version"]) {
        let parts: Vec<_> = version
            .split_whitespace()
            .nth(2)
            .unwrap_or("")
            .split('.')
            .filter_map(|part| part.parse::<u32>().ok())
            .collect();
        if parts.len() < 2 || (parts[0], parts[1]) < (2, 45) {
            healthy = false;
            output::warning("Git 2.45 or newer is required.");
        }
    }
    match Config::load(path) {
        Ok(config) if !config.identities.is_empty() => {
            output::row("Configuration", "Valid", Color::Changed)
        }
        Ok(_) => {
            healthy = false;
            output::warning("No identities configured. Run mgh identity new.");
        }
        Err(error) => {
            healthy = false;
            output::warning(&format!("{error:#}"));
        }
    }
    match github::active() {
        Ok(account) => output::row("GitHub account", &account, Color::Changed),
        Err(error) => {
            healthy = false;
            output::warning(&format!("{error:#}"));
        }
    }
    let problems = if policy::repository()? {
        hooks::problems()?
    } else {
        let mut problems = hooks::shared_problems()?;
        if git::global("core.hooksPath")? != hooks::directory()?.to_string_lossy() {
            problems.push("Global Git hook path does not use mgh; run mgh setup.".to_owned());
        }
        problems
    };
    for problem in &problems {
        output::warning(problem);
    }
    healthy &= problems.is_empty();
    ensure!(
        healthy,
        "Resolve the reported problems; run mgh setup to install or repair hooks, then mgh doctor again"
    );
    output::section("✓ Installation checks passed");
    Ok(())
}
