//! Commit enforcement, hook installation and forwarding, repairs and clone permission
//! boundaries.

use super::support::*;
use serde_json::json;
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::fs::symlink;
use std::process::Stdio;

/// Checks must use live authentication and actual author/committer values before allowing
/// project hooks to run.
#[test]
fn commit_checks_live_account_actual_environment_and_project_order() {
    let env = Sandbox::new();
    env.init_repo();
    env.setup_defaults();
    env.success(&["repo", "allowed", "add", "personal"]);
    env.write_hook("pre-commit", "echo native >> native-log", true);
    let o = env.success(&["repo", "check"]);
    assert!(o.stdout.is_empty());
    assert!(o.stderr.is_empty());
    let o = env
        .mgh(&["repo", "check"])
        .env("GIT_AUTHOR_NAME", "Wrong Name")
        .output()
        .unwrap();
    assert!(!o.status.success());
    assert!(err(&o).contains("AUTHOR commit details do not match identity [personal]"));
    assert!(err(&o).contains("--reset-author"));
    let o = env
        .mgh(&["repo", "check"])
        .env("GIT_COMMITTER_EMAIL", "bad@e")
        .output()
        .unwrap();
    assert!(!o.status.success());
    assert!(err(&o).contains("COMMITTER"));
    assert_ok(
        &env.mgh(&["repo", "check"])
            .env("GIT_AUTHOR_EMAIL", "123+ALICE@USERS.NOREPLY.GITHUB.COM")
            .output()
            .unwrap(),
    );
    env.set_accounts(json!({
        "selected": "alice",
        "accounts": [
            {
                "login": "alice-work",
                "active": true,
                "state": "success"
            }
        ]
    }));
    let o = env.hook("pre-commit", b"", &[]);
    assert!(!o.status.success());
    assert!(err(&o).contains("GitHub is using alice-work"));
    assert!(!env.repo.join("native-log").exists());
    env.set_accounts(json!({
        "selected": "alice",
        "accounts": [
            {
                "login": "alice",
                "active": true,
                "state": "error",
                "error": "expired token"
            }
        ]
    }));
    env.rejects(&["repo", "check"], "expired token");
    env.set_accounts(json!({
        "selected": "alice",
        "accounts": [
            {
                "login": "alice",
                "active": true,
                "state": "success"
            }
        ]
    }));
    assert_ok(&env.hook("pre-commit", b"", &[]));
    assert!(env.repo.join("native-log").exists());
    env.success(&["repo", "protections", "off"]);
    env.set_local("user.name", "Wrong");
    env.rejects(&["repo", "check"], "AUTHOR");
    assert_ok(&env.hook("pre-commit", b"", &[]));
}

/// Every launcher must preserve native arguments, binary stdin, output streams and project-hook
/// rejection.
#[test]
fn hooks_install_all_forward_arguments_binary_input_and_failures() {
    let env = Sandbox::new();
    env.init_repo();
    env.setup_defaults();
    env.success(&["repo", "protections", "off"]);
    fs::remove_file(&env.config).unwrap();
    let names = [
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
    for name in names {
        let launcher = env.state.join("hooks").join(name);
        assert!(launcher.exists());
        assert_ne!(
            fs::metadata(&launcher).unwrap().permissions().mode() & 0o111,
            0
        );
        env.write_hook(name,"printf '%s\\n' \"$@\" > hook-args; cat > hook-input; echo project-stdout; echo project-stderr >&2",true);
        let input = b"\xff\x00payload\n";
        let mut child = env
            .command(&launcher)
            .args(["", "--option", "a ' b"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(input).unwrap();
        let o = child.wait_with_output().unwrap();
        assert_ok(&o);
        assert_eq!(bytes(env.repo.join("hook-args")), b"\n--option\na ' b\n");
        assert_eq!(bytes(env.repo.join("hook-input")), input);
        assert_eq!(out(&o), "project-stdout\n");
        assert_eq!(err(&o), "project-stderr\n");
    }
    env.write_hook("pre-push", "exit 0", true);
    assert_ok(&env.hook("pre-push", &vec![b'x'; 1024 * 1024], &[]));
    env.write_hook("commit-msg", "echo native-rejected >&2; exit 7", true);
    let o = env.hook("commit-msg", b"", &["message"]);
    assert!(!o.status.success());
    assert!(err(&o).contains("Existing commit-msg hook rejected"));
    assert!(err(&o).contains("native-rejected"));
}

/// Missing or damaged integration must be reported honestly and repaired without replacing
/// foreign hooks.
#[test]
fn readiness_failures_repairs_overrides_and_preservation() {
    let env = Sandbox::new();
    env.init_repo();
    env.setup_defaults();
    env.success(&["repo", "allowed", "add", "personal"]);
    let hook = env.state.join("hooks/pre-commit");
    fs::remove_file(&hook).unwrap();
    let o = env.success(&["repo"]);
    assert!(out(&o).contains("Missing launcher"));
    assert!(!out(&o).contains("Commit + push checks enabled"));
    env.success(&["internal", "enter"]);
    assert!(hook.exists());
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(out(&env.success(&["repo"])).contains("Nonexecutable launcher"));
    env.success(&["setup"]);
    fs::write(&hook, "foreign").unwrap();
    env.rejects(&["setup"], "Unrecognized managed hook destination");
    assert_eq!(bytes(&hook), b"foreign");
    env.success(&["repo", "protections", "off"]);
    env.rejects(&["repo", "protections", "on"], "Unrecognized");
    assert_eq!(
        env.git_without_hooks(&["config", "--local", "mgh.protections"]),
        "false"
    );
    let env = Sandbox::new();
    env.init_repo();
    env.write_identities();
    env.git_without_hooks(&["config", "--global", "core.hooksPath", "/foreign/hooks"]);
    let before = bytes(env.home.join(".gitconfig"));
    env.rejects(&["setup"], "Unsupported custom global core.hooksPath");
    assert_eq!(bytes(env.home.join(".gitconfig")), before);
    assert!(!env.state.exists());
}

/// Only an initial clone checkout should request permissions; ordinary checkouts must not
/// reprompt.
#[test]
fn pty_initial_clone_checkout_prompts_and_ordinary_checkout_does_not() {
    let env = Sandbox::new();
    env.init_repo();
    env.setup_defaults();
    env.driver();
    let exe = env!("CARGO_BIN_EXE_mgh");
    let zero = "0".repeat(40);
    let id = "a".repeat(40);
    let o = env.pty(
        exe,
        &["internal", "hook", "post-checkout", &zero, &id, "1"],
        json!([["Which identities may use this repository?", "\u{1b}"]]),
    );
    assert_ok(&o);
    assert!(out(&o).contains("remain blocked"));
    assert!(
        !env.native_git(&["config", "--local", "--get-all", "mgh.allowed-identity"])
            .status
            .success()
    );
    let o = env.hook("post-checkout", b"", &[&id, &id, "1"]);
    assert_ok(&o);
    assert!(!out(&o).contains("Which identities"));
}

/// Hooks must handle early initialization while unprotected project hooks remain usable without
/// identity config.
#[test]
fn protected_hook_before_repository_discovery_and_plain_hooks_without_config() {
    let env = Sandbox::new();
    env.write_identities();
    assert_ok(&env.hook("pre-commit", b"", &[]));
    let o = env.hook("pre-push", b"", &[]);
    assert!(!o.status.success());
    assert!(err(&o).contains("requires a Git repository"));
    fs::remove_file(&env.config).unwrap();
    assert_ok(&env.hook("post-commit", b"", &[]));
    let o = env.hook("pre-commit", b"", &[]);
    assert!(!o.status.success());
    assert!(err(&o).contains("identity configuration"));
    let gitdir = env.root.join("initializing");
    fs::create_dir_all(gitdir.join("hooks")).unwrap();
    let hook = gitdir.join("hooks/commit-msg");
    fs::write(&hook, "#!/bin/sh\nprintf '%s' \"$1\" > before-discovery\n").unwrap();
    fs::set_permissions(hook, fs::Permissions::from_mode(0o700)).unwrap();
    let o = env
        .mgh(&["internal", "hook", "commit-msg", "message"])
        .env("GIT_DIR", &gitdir)
        .output()
        .unwrap();
    assert_ok(&o);
    assert_eq!(bytes(env.repo.join("before-discovery")), b"message");
}

/// Integration must recheck effective hook paths and report overrides, recursion and
/// nonexecutable hooks.
#[test]
fn hook_overrides_include_rechecks_recursion_and_nonexecutable_reports() {
    let env = Sandbox::new();
    env.init_repo();
    env.setup_defaults();
    env.success(&["repo", "allowed", "add", "personal"]);
    env.write_hook("post-commit", "echo should-not-run", false);
    let o = env.success(&["repo"]);
    assert!(out(&o).contains("skipped: nonexecutable"));
    let o = env
        .mgh(&["repo"])
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "core.hooksPath")
        .env("GIT_CONFIG_VALUE_0", "/command/hooks")
        .output()
        .unwrap();
    assert_ok(&o);
    assert!(out(&o).contains("command"));
    assert!(!out(&o).contains("Commit + push checks enabled"));
    env.set_local(
        "mgh.originalHooksPath",
        env.state.join("hooks").to_str().unwrap(),
    );
    let o = env.hook("post-commit", b"", &[]);
    assert!(!o.status.success());
    assert!(err(&o).contains("recursion"));
    env.git_without_hooks(&["config", "--local", "--unset-all", "mgh.originalHooksPath"]);
    let alias = env.root.join("shared-alias");
    symlink(env.state.join("hooks"), &alias).unwrap();
    env.set_local("core.hooksPath", alias.to_str().unwrap());
    env.success(&["setup"]);
    assert!(
        !env.native_git(&["config", "--local", "mgh.originalHooksPath"])
            .status
            .success()
    );
    // The overriding include remains later than the local key that repair writes.
    let include = env.root.join("late.conf");
    fs::write(&include, "[core]\n hooksPath = /included/hooks\n").unwrap();
    env.set_local("include.path", include.to_str().unwrap());
    let o = env.success(&["internal", "enter"]);
    assert!(out(&o).contains("Protections are not active"));
    assert!(!out(&o).contains("Commit + push checks enabled"));
}

/// Keeping a managed marker must not make altered launcher contents look healthy.
#[test]
fn changed_launcher_with_intact_marker_is_unhealthy() {
    let env = Sandbox::new();
    env.init_repo();
    env.setup_defaults();
    let path = env.state.join("hooks/pre-commit");
    let text = fs::read_to_string(&path).unwrap();
    fs::write(&path, format!("{text}exit 0\n")).unwrap();
    let o = env.success(&["repo"]);
    assert!(out(&o).contains("Changed launcher"));
    assert!(!out(&o).contains("Commit + push checks enabled"));
    env.success(&["setup"]);
}

/// Disabling mgh protections must still run existing Husky and Cocogitto commit-message checks.
#[test]
fn husky_and_cocogitto_message_hooks_coexist_when_disabled() {
    let env = Sandbox::new();
    env.init_repo();
    env.setup_defaults();
    env.success(&["repo", "allowed", "add", "personal"]);
    let husky = env.repo.join(".husky/_");
    fs::create_dir_all(&husky).unwrap();
    let wrapper = husky.join("commit-msg");
    fs::write(&wrapper, "#!/bin/sh\nexec sh .husky/commit-msg \"$@\"\n").unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(
        env.repo.join(".husky/commit-msg"),
        "cog verify --ignore-merge-commits --file \"$1\"\n",
    )
    .unwrap();
    env.set_local("core.hooksPath", ".husky/_");
    env.success(&["setup"]);
    let o = env.success(&["repo"]);
    assert!(out(&o).contains(".husky/commit-msg"));
    let managed = bytes(env.state.join("hooks/commit-msg"));
    fs::write(env.repo.join("message"), "not conventional\n").unwrap();
    let o = env.hook("commit-msg", b"", &["message"]);
    assert!(!o.status.success());
    assert!(err(&o).contains("Existing commit-msg hook rejected"));
    env.success(&["repo", "protections", "off"]);
    let o = env.hook("commit-msg", b"", &["message"]);
    assert!(!o.status.success());
    fs::write(env.repo.join("message"), "feat: valid fixture\n").unwrap();
    assert_ok(&env.hook("commit-msg", b"", &["message"]));
    assert_eq!(bytes(env.state.join("hooks/commit-msg")), managed);
}

/// A real rejected Git commit must leave HEAD unchanged and must not run later project hooks.
#[test]
fn native_commit_rejection_preserves_refs_and_native_hooks() {
    let env = Sandbox::new();
    env.init_repo();
    env.setup_defaults();
    env.success(&["repo", "allowed", "add", "personal"]);
    env.commit_without_hooks("Alice Example", "Alice@example.com");
    let before = env.git_without_hooks(&["rev-parse", "HEAD"]);
    fs::write(env.repo.join("file"), "changed").unwrap();
    env.git_without_hooks(&["add", "file"]);
    env.write_hook("pre-commit", "echo native > native-called", true);
    let o = env.native_git(&[
        "commit",
        "--author=Wrong <wrong@else>",
        "-m",
        "feat: rejected",
    ]);
    assert!(!o.status.success());
    assert!(err(&o).contains("AUTHOR"));
    assert_eq!(env.git_without_hooks(&["rev-parse", "HEAD"]), before);
    assert!(!env.repo.join("native-called").exists());
    assert_ok(&env.native_git(&["commit", "-m", "feat: accepted"]));
    assert!(env.repo.join("native-called").exists());
}

/// Real normal, bare and no-checkout clones must not inherit the source repository's local
/// authorization.
#[test]
fn actual_clones_do_not_inherit_permissions() {
    let env = Sandbox::new();
    env.init_repo();
    env.setup_defaults();
    env.success(&["repo", "allowed", "add", "personal"]);
    env.commit_without_hooks("Alice Example", "Alice@example.com");
    for (kind, args) in [
        ("normal", vec![]),
        ("bare", vec!["--bare"]),
        ("no-checkout", vec!["--no-checkout"]),
    ] {
        let target = env.root.join(format!("clone-{kind}"));
        let o = env
            .command("git")
            .arg("clone")
            .args(args)
            .arg(&env.repo)
            .arg(&target)
            .output()
            .unwrap();
        assert_ok(&o);
        let o = env
            .mgh(&["repo", "allowed"])
            .current_dir(&target)
            .output()
            .unwrap();
        assert_ok(&o);
        assert!(out(&o).contains("none selected"));
        let local = if kind == "bare" {
            target.join("config")
        } else {
            target.join(".git/config")
        };
        assert!(
            !fs::read_to_string(local)
                .unwrap()
                .contains("allowed-identity")
        );
    }
    let empty = env.root.join("empty");
    env.git_without_hooks(&["init", "-b", "main", empty.to_str().unwrap()]);
    let clone = env.root.join("empty-clone");
    let o = env
        .command("git")
        .arg("clone")
        .arg(&empty)
        .arg(&clone)
        .output()
        .unwrap();
    assert_ok(&o);
    assert!(
        !fs::read_to_string(clone.join(".git/config"))
            .unwrap()
            .contains("allowed-identity")
    );
}

/// When Git closes a clone hook's input, entry must show recovery guidance and keep the clone
/// blocked.
#[test]
fn actual_interactive_clone_reports_closed_input_and_stays_blocked() {
    let env = Sandbox::new();
    env.init_repo();
    env.setup_defaults();
    env.success(&["repo", "allowed", "add", "personal"]);
    env.commit_without_hooks("Alice Example", "Alice@example.com");
    env.driver();
    let target = env.root.join("interactive-clone");
    let o = env.pty(
        "git",
        &[
            "clone",
            env.repo.to_str().unwrap(),
            target.to_str().unwrap(),
        ],
        json!([]),
    );
    assert_ok(&o);
    assert!(out(&o).contains("mgh repo allowed update"));
    assert!(!out(&o).contains("Which identities may use this repository?"));
    let o = env
        .command("git")
        .arg("-C")
        .arg(&target)
        .args(["config", "--local", "mgh.allowed-identity"])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
}

/// A native hook argument that resembles an mgh flag must be forwarded unchanged.
#[test]
fn hook_first_native_argument_can_be_a_global_application_flag() {
    let env = Sandbox::new();
    env.init_repo();
    env.setup_defaults();
    env.success(&["repo", "protections", "off"]);
    env.write_hook("post-commit", "printf '%s\\n' \"$@\" > flag-args", true);
    fs::remove_file(&env.config).unwrap();
    let o = env
        .command(env.state.join("hooks/post-commit"))
        .args(["--config", "native-value", "--help", "--version", ""])
        .output()
        .unwrap();
    assert_ok(&o);
    assert_eq!(
        bytes(env.repo.join("flag-args")),
        b"--config\nnative-value\n--help\n--version\n\n"
    );
}

/// Changing a launcher's selected configuration must be detected even when other managed text
/// remains intact.
#[test]
fn launcher_configuration_modification_is_not_reported_as_healthy() {
    let env = Sandbox::new();
    env.init_repo();
    env.setup_defaults();
    let path = env.state.join("hooks/pre-commit");
    let text = fs::read_to_string(&path).unwrap();
    fs::write(
        &path,
        text.replace(
            &shquote(env.config.to_str().unwrap()),
            &shquote("/changed/config"),
        ),
    )
    .unwrap();
    let o = env.success(&["repo"]);
    assert!(out(&o).contains("Changed launcher"));
    assert!(!out(&o).contains("Commit + push checks enabled"));
}

/// Merge hooks and deletion-only pushes still need authorization before project hooks execute.
#[test]
fn protected_merge_hook_and_push_deletion_enforce_before_project_hooks() {
    let env = Sandbox::new();
    env.init_repo();
    env.setup_defaults();
    env.success(&["repo", "allowed", "add", "personal"]);
    env.write_hook("pre-merge-commit", "echo called > merge-called", true);
    assert_ok(&env.hook("pre-merge-commit", b"", &[]));
    fs::remove_file(env.repo.join("merge-called")).unwrap();
    env.write_hook("pre-push", "echo called > push-called", true);
    env.set_accounts(json!({
        "selected": "alice",
        "accounts": [
            {
                "login": "alice-work",
                "active": true,
                "state": "success"
            }
        ]
    }));
    let o = env.hook("pre-merge-commit", b"", &[]);
    assert!(!o.status.success());
    assert!(!env.repo.join("merge-called").exists());
    let zero = "0".repeat(40);
    let input = format!("(delete) {zero} refs/heads/deleted {zero}\n");
    let o = env.hook("pre-push", input.as_bytes(), &[]);
    assert!(!o.status.success());
    assert!(err(&o).contains("not allowed"));
    assert!(!env.repo.join("push-called").exists());
}
