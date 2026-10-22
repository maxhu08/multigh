use super::*;
use anyhow::Context;
use std::process::Command;

// Use a command-scoped alias after native Git succeeds. Git itself interprets all
// global/init options and gives entry the same repository/environment/config scope.
pub(super) fn forward(
    inv: &Invocation,
    args: &[OsString],
    ui: &mut dyn Interaction,
) -> Result<i32> {
    let status = Command::new("git")
        .args(args)
        .status()
        .context("Cannot execute git")?;
    if !status.success() {
        return Ok(status.code().unwrap_or(1));
    }
    if !ui.interactive() {
        return Ok(0);
    }
    let Some((global, target)) = init_target(args) else {
        return Ok(0);
    };
    let result: Result<()> = (|| {
        let exe = std::env::current_exe()?;
        let mut entry = format!("!{}", platform::quote(&exe.to_string_lossy()));
        if let Some(config) = &inv.config {
            let config = identities::config_path(Some(config))?;
            entry.push_str(&format!(
                " --config {}",
                platform::quote(&config.to_string_lossy())
            ));
        }
        entry.push_str(" internal enter");
        let alias = format!("alias.mgh-initialization-entry={entry}");
        let mut cmd = Command::new("git");
        // --bare resolves its implicit GIT_DIR before a later -C. The initialized
        // repository already records core.bare, so rediscover it at the target.
        cmd.args(global.into_iter().filter(|arg| arg != "--bare"));
        if let Some(target) = target {
            cmd.arg("-C").arg(target);
        }
        cmd.arg("-c").arg(alias).arg("mgh-initialization-entry");
        let status = cmd.status().context("Cannot start repository entry")?;
        anyhow::ensure!(status.success(), "Repository entry failed ({status})");
        Ok(())
    })();
    if let Err(error) = result {
        ui.warning(&format!(
            "Git initialization succeeded, but repository entry failed: {error:#}. \
             Run: mgh repo allowed update in the new repository."
        ));
    }
    Ok(0)
}

fn init_target(args: &[OsString]) -> Option<(Vec<OsString>, Option<OsString>)> {
    let mut index = 0;
    let mut global = vec![];
    while let Some(arg) = args.get(index) {
        let s = arg.to_str()?;
        if s == "init" {
            index += 1;
            break;
        }
        if matches!(
            s,
            "-C" | "-c" | "--git-dir" | "--work-tree" | "--namespace" | "--config-env"
        ) {
            global.push(arg.clone());
            index += 1;
            global.push(args.get(index)?.clone());
            index += 1;
            continue;
        }
        if s.starts_with("--git-dir=")
            || s.starts_with("--work-tree=")
            || s.starts_with("--namespace=")
            || s.starts_with("--config-env=")
            || (s.starts_with("-C") || s.starts_with("-c")) && s.len() > 2
        {
            global.push(arg.clone());
            index += 1;
            continue;
        }
        if matches!(
            s,
            "--bare" | "--no-optional-locks" | "--no-pager" | "--literal-pathspecs"
        ) {
            global.push(arg.clone());
            index += 1;
            continue;
        }
        return None;
    }
    if index == 0 || args.get(index - 1)?.to_str() != Some("init") {
        return None;
    }
    let mut target = None;
    let mut separator = false;
    while let Some(arg) = args.get(index) {
        let s = arg.to_str()?;
        if !separator && s == "--" {
            separator = true;
            index += 1;
            continue;
        }
        if !separator && matches!(s, "--help" | "-h" | "--version") {
            return None;
        }
        if !separator
            && matches!(
                s,
                "-b" | "--initial-branch" | "--separate-git-dir" | "--object-format" | "--template"
            )
        {
            index += 2;
            continue;
        }
        if !separator
            && s.starts_with("--")
            && "--initial-branch".starts_with(s)
            && !s.contains('=')
        {
            index += 2;
            continue;
        }
        if !separator && s.starts_with('-') {
            index += 1;
            continue;
        }
        target = Some(arg.clone());
        index += 1;
    }
    Some((global, target))
}

pub(super) fn initialization(inv: &Invocation, kind: ShellKind) -> Result<String> {
    let script = match kind {
        ShellKind::Fish => include_str!("../../shell/init.fish"),
        ShellKind::Bash => include_str!("../../shell/init.bash"),
        ShellKind::Zsh => include_str!("../../shell/init.zsh"),
    };
    Ok(if inv.config.is_some() {
        script.replace(
            "mgh internal",
            &format!(
                "mgh --config {} internal",
                platform::quote(&inv.path()?.to_string_lossy())
            ),
        )
    } else {
        script.to_string()
    })
}
