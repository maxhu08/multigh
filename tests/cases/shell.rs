//! Real Fish/Bash/Zsh integration, completion and interactive Git initialization.

use super::support::*;
use serde_json::json;
use std::fs;
use std::path::Path;
use std::process::Stdio;

/// Interactive startup must install integration once and preserve handlers; noninteractive
/// shells must stay untouched.
#[test]
fn actual_shell_startup_noninteractive_and_existing_handlers() {
    let env = Sandbox::new();
    env.driver();
    for (shell, script) in [
        ("bash", "init.bash"),
        ("zsh", "init.zsh"),
        ("fish", "init.fish"),
    ] {
        let path = env.root.join(script);
        fs::write(
            &path,
            bytes(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("shell")
                    .join(script),
            ),
        )
        .unwrap();
        let source = if shell == "fish" {
            format!(
                "source '{}'",
                path.display().to_string().replace('\'', "\\'")
            )
        } else {
            format!("source {}", shquote(path.to_str().unwrap()))
        };
        let o = env.command(shell).arg("-c").arg(&source).output().unwrap();
        assert_ok(&o);
        assert!(o.stdout.is_empty());
        assert!(o.stderr.is_empty());
        let command = if shell == "bash" {
            format!(
                concat!(
                    "function git {{ echo preserved-git; }}; ",
                    "PROMPT_COMMAND='echo existing-prompt'; {source}; {source}; ",
                    "git; printf '%s\\n' \"$PROMPT_COMMAND\"; exit\n",
                ),
                source = source,
            )
        } else if shell == "zsh" {
            format!(
                "function git {{ echo preserved-git; }}; {source}; {source}; git; print -- $chpwd_functions; exit\n"
            )
        } else {
            format!("function git; echo preserved-git; end; {source}; {source}; git; exit\n")
        };
        let args = if shell == "bash" {
            vec!["--noprofile", "--norc", "-i"]
        } else if shell == "zsh" {
            vec!["-f", "-i"]
        } else {
            vec!["--no-config", "-i"]
        };
        let o = env.pty(shell, &args, json!([["", command]]));
        assert_ok(&o);
        assert!(out(&o).contains("preserved-git"), "{shell}: {}", both(&o));
    }
    assert!(!env.state.exists());
    assert!(!env.root.join("gh.log").exists());
}

/// Git init must preserve native behavior, with a permission picker only after successful
/// interactive initialization.
#[test]
fn interactive_git_init_native_success_cancel_failure_and_options() {
    let env = Sandbox::new();
    env.setup_defaults();
    env.driver();
    let exe = env!("CARGO_BIN_EXE_mgh");
    let target = env.root.join("new ' repo");
    let o = env.pty(
        exe,
        &[
            "internal",
            "git",
            "--",
            "-c",
            "init.defaultBranch=custom",
            "init",
            "--initial-branch=other",
            "--",
            target.to_str().unwrap(),
        ],
        json!([["Which identities may use this repository?", "\u{1b}"]]),
    );
    assert_ok(&o);
    assert!(target.join(".git").exists());
    assert!(out(&o).contains("remain blocked"));
    let o = env
        .mgh(&["repo", "allowed", "add", "personal"])
        .current_dir(&target)
        .output()
        .unwrap();
    assert_ok(&o);
    let o = env.pty(
        exe,
        &[
            "internal",
            "git",
            "--",
            "-C",
            target.to_str().unwrap(),
            "init",
        ],
        json!([]),
    );
    assert_ok(&o);
    assert!(!out(&o).contains("Which identities"));
    let o = env.pty(
        exe,
        &[
            "internal",
            "git",
            "--",
            "-C",
            "/nonexistent-mgh-path",
            "init",
        ],
        json!([]),
    );
    assert!(!o.status.success());
    assert!(!out(&o).contains("Which identities"));
    let text = fs::read_to_string(env.home.join(".gitconfig")).unwrap();
    assert!(!text.contains("mgh-initialization-entry"));
}

/// Actual Tab presses must complete mgh commands and options in each supported interactive
/// shell.
#[test]
fn shell_tab_completion_commands_and_options() {
    let env = Sandbox::new();
    env.driver();
    for shell in ["bash", "zsh", "fish"] {
        let script = env.root.join(format!("{shell}.init"));
        fs::write(&script, out(&env.success(&["shell", "init", shell]))).unwrap();
        let setup = if shell == "bash" {
            format!(
                "PS1='READY> '; source {}; echo LOADED\n",
                shquote(script.to_str().unwrap())
            )
        } else if shell == "zsh" {
            format!(
                "PROMPT='READY> '; source {}; echo LOADED\n",
                shquote(script.to_str().unwrap())
            )
        } else {
            format!(
                "function fish_prompt; printf 'READY> '; end; source {}; echo LOADED\n",
                shquote(script.to_str().unwrap())
            )
        };
        let args = match shell {
            "bash" => vec!["--noprofile", "--norc", "-i"],
            "zsh" => vec!["-f", "-i"],
            _ => vec!["--no-config", "-i"],
        };
        let o = env.pty(
            shell,
            &args,
            json!([
                ["", setup],
                ["LOADED", "mgh ident\t"],
                ["identity", "--help\n"],
                ["Usage: mgh identity", "mgh status --fu\t"],
                ["--full", " --help\n"],
                ["Usage: mgh status", "exit 0\n"]
            ]),
        );
        assert_ok(&o);
        let text = out(&o);
        assert!(!text.contains("command not found"), "{shell}: {text}");
    }
    assert!(!env.home.join(".zcompdump").exists());
}

/// Directory changes must trigger entry without losing Bash prompt-array handlers or changing
/// noninteractive behavior.
#[test]
fn actual_shell_directory_changes_and_prompt_array_preservation() {
    let env = Sandbox::new();
    env.init_repo();
    env.setup_defaults();
    env.success(&["repo", "allowed", "add", "personal"]);
    env.driver();
    for shell in ["bash", "zsh", "fish"] {
        let script = env.root.join(format!("cd-{shell}.init"));
        fs::write(&script, out(&env.success(&["shell", "init", shell]))).unwrap();
        let body = if shell == "bash" {
            format!(
                concat!(
                    "PROMPT_COMMAND=('echo KEEP_ARRAY' 'echo SECOND_ARRAY_HANDLER'); ",
                    "source {0}; ",
                    "source {0}; ",
                    "declare -p PROMPT_COMMAND; ",
                    "if (( BASH_VERSINFO[0] > 5 || (BASH_VERSINFO[0] == 5 && BASH_VERSINFO[1] >= 1) )); ",
                    "then echo NATIVE_ARRAY; ",
                    "else echo ADAPTED_ARRAY; ",
                    "fi; ",
                    "echo LOADED\n",
                ),
                shquote(script.to_str().unwrap())
            )
        } else {
            format!(
                "source {0}; source {0}; echo LOADED\n",
                shquote(script.to_str().unwrap())
            )
        };
        let args = match shell {
            "bash" => vec!["--noprofile", "--norc", "-i"],
            "zsh" => vec!["-f", "-i"],
            _ => vec!["--no-config", "-i"],
        };
        let steps = json!([
            ["", body],
            [
                "LOADED",
                format!("cd {}\n", shquote(env.repo.to_str().unwrap()))
            ],
            [
                "Current repository",
                format!(
                    "cd {}; echo DUPLICATE_DONE\n",
                    shquote(env.repo.to_str().unwrap())
                )
            ],
            ["DUPLICATE_DONE", "exit 0\n"]
        ]);
        let o = env
            .command("python3")
            .current_dir(&env.home)
            .arg(env.root.join("driver.py"))
            .arg(shell)
            .args(args)
            .env("MGH_PTY_STEPS", steps.to_string())
            .output()
            .unwrap();
        assert_ok(&o);
        let text = out(&o);
        assert_eq!(
            text.matches("Current repository").count(),
            1,
            "{shell}: {text}"
        );
        if shell == "bash" {
            assert!(text.contains("SECOND_ARRAY_HANDLER\r\n"), "{text}");
            if text.contains("NATIVE_ARRAY\r\n") {
                assert!(text.contains("[0]=\"echo KEEP_ARRAY\""), "{text}");
                assert!(text.contains("[2]=\"_mgh_enter\""), "{text}");
                assert!(!text.contains("[3]="), "{text}");
            } else {
                assert!(text.contains("ADAPTED_ARRAY\r\n"), "{text}");
                assert!(text.contains("[0]=\"_mgh_prompt_dispatch\""), "{text}");
            }
        }
    }
}

/// Autoswitch must run only when crossing repository boundaries, including nested repositories
/// and returning after leaving Git, while staying silent within the same repository.
#[test]
fn autoswitch_only_reports_on_repository_changes() {
    let env = Sandbox::new();
    env.init_repo();
    env.setup_defaults();
    env.success(&["repo", "allowed", "add", "personal"]);
    env.success(&["settings", "autoswitch", "on"]);
    let sub = env.repo.join("sub/deeper");
    let nested = env.repo.join("nested");
    let nested_sub = nested.join("sub");
    fs::create_dir_all(&sub).unwrap();
    fs::create_dir_all(&nested_sub).unwrap();
    env.git_without_hooks(&["-C", nested.to_str().unwrap(), "init", "-b", "main"]);
    let o = env
        .mgh(&["repo", "allowed", "add", "work"])
        .current_dir(&nested)
        .output()
        .unwrap();
    assert_ok(&o);
    env.driver();
    for shell in ["bash", "zsh", "fish"] {
        env.success(&["switch", "personal"]);
        let script = env.root.join(format!("repo-changes-{shell}.init"));
        fs::write(&script, out(&env.success(&["shell", "init", shell]))).unwrap();
        let args = match shell {
            "bash" => vec!["--noprofile", "--norc", "-i"],
            "zsh" => vec!["-f", "-i"],
            _ => vec!["--no-config", "-i"],
        };
        let steps = json!([
            [
                "",
                format!(
                    "source {}; echo LOADED\n",
                    shquote(script.to_str().unwrap())
                )
            ],
            [
                "LOADED",
                format!("cd {}\n", shquote(env.repo.to_str().unwrap()))
            ],
            [
                "personal is allowed, no switch needed.",
                format!("cd {}; echo INSIDE\n", shquote(sub.to_str().unwrap()))
            ],
            ["INSIDE", "cd ..; echo PARENT\n"],
            [
                "PARENT",
                format!("cd {}; echo ROOT\n", shquote(env.repo.to_str().unwrap()))
            ],
            [
                "ROOT",
                format!("cd {}\n", shquote(nested.to_str().unwrap()))
            ],
            [
                "switching to work... done.",
                format!(
                    "cd {}; echo NESTED_SUB\n",
                    shquote(nested_sub.to_str().unwrap())
                )
            ],
            [
                "NESTED_SUB",
                format!("cd {}\n", shquote(env.repo.to_str().unwrap()))
            ],
            [
                "switching to personal... done.",
                format!("cd {}; echo OUTSIDE\n", shquote(env.home.to_str().unwrap()))
            ],
            [
                "OUTSIDE",
                format!("cd {}\n", shquote(env.repo.to_str().unwrap()))
            ],
            ["personal is allowed, no switch needed.", "exit 0\n"]
        ]);
        let o = env
            .command("python3")
            .current_dir(&env.home)
            .arg(env.root.join("driver.py"))
            .arg(shell)
            .args(args)
            .env("MGH_PTY_STEPS", steps.to_string())
            .output()
            .unwrap();
        assert_ok(&o);
        let text = out(&o);
        assert_eq!(text.matches("Autoswitch:").count(), 4, "{shell}: {text}");
        assert!(!text.contains('…'), "{shell}: {text}");
        assert!(!text.contains("Current repository"), "{shell}: {text}");
        assert_eq!(env.accounts()["selected"], "alice");
    }
}

/// Git init options and targets must identify the correct repository without leaking a
/// persistent alias.
#[test]
fn git_init_option_matrix_preserves_native_targets_and_scope() {
    for variant in [
        "repeated-c",
        "attached-branch",
        "long-branch",
        "abbreviated-branch",
        "abbreviated-equals",
        "bare",
        "global-bare",
        "git-dir-work-tree",
        "separate-git-dir",
        "sha256",
        "template",
        "shared",
        "separator",
    ] {
        let env = Sandbox::new();
        env.setup_defaults();
        env.driver();
        let exe = env!("CARGO_BIN_EXE_mgh");
        let target = env.root.join("new ' repository");
        let metadata = env.root.join("separate ' metadata");
        let template = env.root.join("template ' directory");
        fs::create_dir_all(&template).unwrap();
        let target_s = target.to_str().unwrap().to_string();
        let metadata_s = metadata.to_str().unwrap().to_string();
        let mut args = vec!["internal".to_string(), "git".to_string(), "--".to_string()];
        let native = match variant {
            "repeated-c" => {
                let parent = env.root.join("parent");
                fs::create_dir_all(&parent).unwrap();
                vec![
                    "-C".into(),
                    env.root.to_str().unwrap().into(),
                    "-C".into(),
                    "parent".into(),
                    "-c".into(),
                    "user.name=command scope".into(),
                    "init".into(),
                    "-b".into(),
                    "topic".into(),
                    target_s.clone(),
                ]
            }
            "attached-branch" => vec![
                "-C".into(),
                env.root.to_str().unwrap().into(),
                "init".into(),
                "-btopic".into(),
                target_s.clone(),
            ],
            "long-branch" => vec![
                "init".into(),
                "--initial-branch".into(),
                "topic".into(),
                target_s.clone(),
            ],
            "abbreviated-branch" => vec![
                "init".into(),
                "--initial-b".into(),
                "topic".into(),
                target_s.clone(),
            ],
            "abbreviated-equals" => {
                vec!["init".into(), "--initial-b=topic".into(), target_s.clone()]
            }
            "bare" => vec!["init".into(), "--bare".into(), target_s.clone()],
            "global-bare" => vec!["--bare".into(), "init".into(), target_s.clone()],
            "git-dir-work-tree" => {
                fs::create_dir_all(&target).unwrap();
                vec![
                    "--git-dir".into(),
                    metadata_s.clone(),
                    "--work-tree".into(),
                    target_s.clone(),
                    "init".into(),
                ]
            }
            "separate-git-dir" => vec![
                "init".into(),
                "--separate-git-dir".into(),
                metadata_s.clone(),
                target_s.clone(),
            ],
            "sha256" => vec![
                "init".into(),
                "--object-format".into(),
                "sha256".into(),
                target_s.clone(),
            ],
            "template" => vec![
                "init".into(),
                "--template".into(),
                template.to_str().unwrap().into(),
                target_s.clone(),
            ],
            "shared" => vec!["init".into(), "--shared=group".into(), target_s.clone()],
            "separator" => vec!["init".into(), "--".into(), target_s.clone()],
            _ => unreachable!(),
        };
        args.extend(native);
        let refs = args.iter().map(String::as_str).collect::<Vec<_>>();
        let o = env.pty(
            exe,
            &refs,
            json!([["Which identities may use this repository?", " \r"]]),
        );
        assert!(o.status.success(), "{variant}: {}", both(&o));
        let mut git = env.command("git");
        if variant == "git-dir-work-tree" {
            git.args(["--git-dir", &metadata_s, "--work-tree", &target_s]);
        } else {
            git.arg("-C").arg(&target);
        }
        let o = git
            .args(["config", "--local", "mgh.allowed-identity"])
            .output()
            .unwrap();
        assert!(o.status.success(), "{variant}: {}", both(&o));
        assert_eq!(out(&o).trim(), "personal", "{variant}");
    }
}

/// Git forwarding must preserve native failures and information commands while keeping a
/// successful init successful.
#[test]
fn git_forwarding_noninteractive_information_aliases_and_entry_failure() {
    let env = Sandbox::new();
    let o = env.success(&["internal", "git", "--", "init", "ordinary"]);
    assert!(out(&o).contains("Initialized"));
    assert!(!env.state.exists());
    assert!(!env.root.join("gh.log").exists());
    let native = env.native_git(&["-C", "/nonexistent-mgh-test", "status"]);
    let wrapped = env.run(&[
        "internal",
        "git",
        "--",
        "-C",
        "/nonexistent-mgh-test",
        "status",
    ]);
    assert_eq!(wrapped.status.code(), native.status.code());
    assert_eq!(wrapped.stdout, native.stdout);
    assert_eq!(wrapped.stderr, native.stderr);
    env.driver();
    let exe = env!("CARGO_BIN_EXE_mgh");
    let o = env.pty(exe, &["internal", "git", "--", "--version"], json!([]));
    assert_ok(&o);
    assert!(!env.state.exists());
    env.git_without_hooks(&[
        "config",
        "--global",
        "alias.includesinit",
        "!echo alias containing init",
    ]);
    let o = env.pty(exe, &["internal", "git", "--", "includesinit"], json!([]));
    assert_ok(&o);
    assert!(!out(&o).contains("Which identities"));
    env.setup_defaults();
    fs::write(&env.config, "invalid").unwrap();
    let o = env.pty(
        exe,
        &["internal", "git", "--", "init", "entry-failure"],
        json!([]),
    );
    assert_ok(&o);
    assert!(out(&o).contains("Git initialization succeeded"));
    assert!(out(&o).contains("mgh repo allowed update"));
    assert!(env.repo.join("entry-failure/.git").exists());
}

/// Generated integration must quote selected config and executable paths containing spaces or
/// apostrophes.
#[test]
fn shell_init_selected_config_and_executable_paths_are_shell_safe() {
    let env = Sandbox::new();
    env.driver();
    let custom = env.root.join("custom ' config.jsonc");
    fs::write(&custom, IDENTITIES).unwrap();
    let exe = env.root.join("my ' mgh");
    fs::copy(env!("CARGO_BIN_EXE_mgh"), &exe).unwrap();
    let o = env
        .command(&exe)
        .args(["--config", custom.to_str().unwrap(), "setup"])
        .output()
        .unwrap();
    assert_ok(&o);
    env.init_repo();
    env.set_local("mgh.protections", "false");
    env.write_hook("commit-msg", "printf '%s' \"$1\" > quote-proof", true);
    let child = env
        .command(env.state.join("hooks/commit-msg"))
        .arg("message ' with spaces")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    assert_ok(&child.wait_with_output().unwrap());
    assert_eq!(
        bytes(env.repo.join("quote-proof")),
        b"message ' with spaces"
    );
    fs::remove_file(&env.config).ok();
    for shell in ["bash", "zsh", "fish"] {
        let o = env.success(&["--config", custom.to_str().unwrap(), "shell", "init", shell]);
        let script = env.root.join(format!("custom-{shell}"));
        fs::write(&script, out(&o)).unwrap();
        let body = format!(
            "source {}; echo CONFIG_LOADED; exit 0\n",
            shquote(script.to_str().unwrap())
        );
        let args = match shell {
            "bash" => vec!["--noprofile", "--norc", "-i"],
            "zsh" => vec!["-f", "-i"],
            _ => vec!["--no-config", "-i"],
        };
        let o = env.pty(shell, &args, json!([["", body]]));
        assert_ok(&o);
        assert!(!out(&o).contains("Cannot read identity configuration"));
        assert!(!out(&o).contains("command not found"));
    }
}

/// Piped Fish initialization must still use the terminal, and existing shell aliases must be
/// preserved.
#[test]
fn shell_piped_fish_init_uses_terminal_and_preserves_aliases() {
    let env = Sandbox::new();
    env.init_repo();
    env.setup_defaults();
    env.driver();
    let o = env.pty(
        "fish",
        &["--no-config", "-i"],
        json!([
            ["", "mgh shell init fish | source\n"],
            ["Which identities may use this repository?", "\u{1b}"],
            ["remain blocked", "exit 0\n"]
        ]),
    );
    assert_ok(&o);
    assert!(
        !env.native_git(&["config", "--local", "--get-all", "mgh.allowed-identity"])
            .status
            .success()
    );
    for shell in ["bash", "zsh"] {
        let script = env.root.join(format!("alias-{shell}.init"));
        fs::write(&script, out(&env.success(&["shell", "init", shell]))).unwrap();
        let body = format!(
            "alias git='echo PRESERVED_ALIAS'; source {}; echo ALIAS_LOADED\n",
            shquote(script.to_str().unwrap())
        );
        let args = if shell == "bash" {
            vec!["--noprofile", "--norc", "-i"]
        } else {
            vec!["-f", "-i"]
        };
        let o = env
            .command("python3")
            .current_dir(&env.home)
            .arg(env.root.join("driver.py"))
            .arg(shell)
            .args(args)
            .env(
                "MGH_PTY_STEPS",
                json!([
                    ["", body],
                    ["ALIAS_LOADED", "git --version\n"],
                    ["PRESERVED_ALIAS --version", "exit 0\n"]
                ])
                .to_string(),
            )
            .output()
            .unwrap();
        assert_ok(&o);
        assert!(
            out(&o).contains("PRESERVED_ALIAS --version"),
            "{shell}: {}",
            both(&o)
        );
    }
}

/// Initialization must retain relative Git environment paths and command-scoped configuration.
#[test]
fn git_init_relative_environment_options_and_command_configuration() {
    let env = Sandbox::new();
    env.setup_defaults();
    env.driver();
    let exe = env!("CARGO_BIN_EXE_mgh");
    let o = env.pty(
        exe,
        &[
            "internal",
            "git",
            "--",
            "--git-dir=relative-metadata",
            "--work-tree=relative-worktree",
            "init",
        ],
        json!([["Which identities may use this repository?", " \r"]]),
    );
    assert_ok(&o);
    let o = env
        .command("git")
        .args([
            "--git-dir=relative-metadata",
            "config",
            "--local",
            "mgh.allowed-identity",
        ])
        .output()
        .unwrap();
    assert_ok(&o);
    assert_eq!(out(&o).trim(), "personal");
    let target = env.root.join("scoped-off");
    let o = env.pty(
        exe,
        &[
            "internal",
            "git",
            "--",
            "-c",
            "mgh.protections=false",
            "-c",
            "core.hooksPath=/command/override",
            "init",
            target.to_str().unwrap(),
        ],
        json!([["Which identities may use this repository?", "\u{1b}"]]),
    );
    assert_ok(&o);
    assert!(out(&o).contains("/command/override"));
    assert!(
        !fs::read_to_string(target.join(".git/config"))
            .unwrap()
            .contains("allowed-identity")
    );
}

/// Abbreviated branch options without a directory must still enter the repository initialized
/// in place.
#[test]
fn abbreviated_initial_branch_without_target_enters_the_invocation_repository() {
    for option in ["--i", "--init", "--initial", "--initial-b"] {
        let env = Sandbox::new();
        env.setup_defaults();
        env.driver();
        let o = env.pty(
            env!("CARGO_BIN_EXE_mgh"),
            &["internal", "git", "--", "init", option, "topic"],
            json!([["Which identities may use this repository?", " \r"]]),
        );
        assert!(o.status.success(), "{option}: {}", both(&o));
        assert_eq!(
            env.git_without_hooks(&["symbolic-ref", "--short", "HEAD"]),
            "topic"
        );
        assert_eq!(
            env.git_without_hooks(&["config", "--local", "mgh.allowed-identity"]),
            "personal"
        );
    }
}
