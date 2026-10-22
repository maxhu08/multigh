//! Public commands, help and argument rejection without configuration or authentication.

use super::support::*;

/// Help and completion must work before setup; unsupported commands and flags must fail.
#[test]
fn cli_surface_config_independence_and_rejections() {
    let env = Sandbox::new();
    for args in [
        vec!["--help"],
        vec!["--version"],
        vec!["identity", "--help"],
        vec!["repo", "allowed", "--help"],
    ] {
        let o = env
            .mgh(&args)
            .env_remove("HOME")
            .env_remove("USERPROFILE")
            .env_remove("XDG_CONFIG_HOME")
            .env_remove("XDG_STATE_HOME")
            .output()
            .unwrap();
        assert_ok(&o);
        assert!(err(&o).is_empty());
        assert!(out(&o).starts_with('\n'));
        assert!(out(&o).ends_with("\n\n"));
    }
    let help = out(&env.success(&["--help"]));
    for command in [
        "setup", "status", "switch", "doctor", "identity", "repo", "settings", "shell",
    ] {
        assert!(help.contains(command));
    }
    assert!(!help.contains("internal"));
    for shell in ["fish", "bash", "zsh", "powershell", "elvish"] {
        let o = env
            .mgh(&["shell", "completions", shell])
            .env_remove("HOME")
            .env_remove("XDG_CONFIG_HOME")
            .output()
            .unwrap();
        assert_ok(&o);
        assert!(!o.stdout.is_empty());
    }
    for shell in ["fish", "bash", "zsh"] {
        assert_ok(
            &env.mgh(&["shell", "init", shell])
                .env_remove("HOME")
                .env_remove("XDG_CONFIG_HOME")
                .output()
                .unwrap(),
        );
    }
    for args in [
        vec!["new"],
        vec!["protect"],
        vec!["protections"],
        vec!["verbose"],
        vec!["welcome"],
        vec!["init"],
        vec!["check"],
        vec!["completions"],
        vec!["enter"],
        vec!["hook"],
        vec!["switch"],
        vec!["switch", "x", "--repo"],
        vec!["identity", "new", "--repo"],
        vec!["repo", "autoswitch"],
        vec!["repo", "allowed", "update", "personal"],
        vec!["settings", "verbose", "yes"],
        vec!["shell", "init", "elvish"],
    ] {
        let o = env.run(&args);
        assert_eq!(o.status.code(), Some(2), "{}", both(&o));
        assert!(o.stdout.is_empty());
    }
    env.rejects(
        &["internal", "hook", "unknown"],
        "Unknown Git hook: unknown",
    );
    assert!(!env.state.exists());
    assert!(!env.root.join("gh.log").exists());
}

/// Every public command needs useful plain help, with wrapping that follows the terminal.
#[test]
fn every_public_leaf_help_describes_scope_interaction_failure_and_recovery() {
    let env = Sandbox::new();
    for (path, expected) in [
        (vec![], "Global GitHub"),
        (vec!["setup"], "first-identity"),
        (vec!["status"], "Read-only"),
        (vec!["switch"], "Repository policy"),
        (vec!["doctor"], "repair"),
        (vec!["identity", "list"], "Read-only"),
        (vec!["identity", "new"], "--username"),
        (vec!["identity", "edit"], "additional emails"),
        (vec!["identity", "remove"], "Stale permissions"),
        (vec!["repo", "status"], "Read-only"),
        (vec!["repo", "check"], "Silent on success"),
        (vec!["repo", "protections"], "defaults ON"),
        (vec!["repo", "allowed", "list"], "stored permissions"),
        (vec!["repo", "allowed", "add"], "idempotent"),
        (vec!["repo", "allowed", "remove"], "unresolved"),
        (vec!["repo", "allowed", "update"], "stdin"),
        (vec!["settings", "autoswitch"], "initially OFF"),
        (vec!["settings", "verbose"], "initially OFF"),
        (vec!["settings", "welcome"], "initially OFF"),
        (vec!["shell", "init"], "Noninteractive"),
        (vec!["shell", "completions"], "Elvish"),
    ] {
        let mut args = path;
        args.push("--help");
        let o = env.success(&args);
        let text = out(&o);
        assert!(
            text.split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase()
                .contains(&expected.to_lowercase()),
            "{args:?}: {text}"
        );
        assert!(text.starts_with('\n'));
        assert!(text.ends_with("\n\n"));
        assert!(err(&o).is_empty());
        assert!(!text.contains('\x1b'), "{args:?}: {text}");
        assert!(
            text.lines().all(|line| line.chars().count() <= 100),
            "{args:?}: {text}"
        );
    }
    env.driver();
    for columns in [50, 120] {
        for path in [
            vec!["--help"],
            vec!["identity", "new", "--help"],
            vec!["help", "status"],
        ] {
            let o = env
                .command("python3")
                .arg(env.root.join("driver.py"))
                .arg(env!("CARGO_BIN_EXE_mgh"))
                .args(&path)
                .env("MGH_PTY_STEPS", "[]")
                .env("MGH_PTY_COLUMNS", columns.to_string())
                .env("TERM", "xterm-256color")
                .env("CLICOLOR_FORCE", "1")
                .env_remove("NO_COLOR")
                .output()
                .unwrap();
            assert_ok(&o);
            let text = out(&o);
            assert!(!text.contains('\x1b'), "{path:?}: {text}");
            assert!(
                text.lines().all(|line| line.chars().count() <= columns),
                "{columns} {path:?}: {text}"
            );
            if columns > 80 {
                assert!(
                    text.lines().any(|line| line.chars().count() > 80),
                    "{path:?}: {text}"
                );
            }
            assert!(text.ends_with("\r\n\r\n"), "{text}");
        }
    }
    for args in [vec!["shell"], vec!["shell", "init"]] {
        let o = env
            .command("python3")
            .arg(env.root.join("driver.py"))
            .arg(env!("CARGO_BIN_EXE_mgh"))
            .args(&args)
            .env("MGH_PTY_STEPS", "[]")
            .env("TERM", "xterm-256color")
            .env("CLICOLOR_FORCE", "1")
            .env_remove("NO_COLOR")
            .output()
            .unwrap();
        assert_eq!(o.status.code(), Some(2), "{}", both(&o));
        assert!(!out(&o).contains('\x1b'), "{args:?}: {}", out(&o));
        assert!(out(&o).contains("Usage:"));
    }
    assert!(!env.root.join("gh.log").exists());
    assert!(!env.state.exists());
}
