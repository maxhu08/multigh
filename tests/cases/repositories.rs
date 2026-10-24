//! Repository scope, explicit permissions, switching, diagnostics and completed writes on
//! failure.

use super::support::*;
use serde_json::json;
use std::fs;
use std::os::unix::fs::PermissionsExt;

/// Repository commands must reject a missing repository before reading or changing identity
/// configuration.
#[test]
fn missing_repo_and_configuration_error_order() {
    let env = Sandbox::new();
    fs::write(&env.config, "broken").unwrap();
    for args in [
        vec!["repo"],
        vec!["repo", "check"],
        vec!["repo", "allowed", "list"],
        vec!["repo", "allowed", "add", "x"],
        vec!["repo", "protections", "on"],
        vec!["repo", "protections", "off"],
    ] {
        env.rejects(
            &args,
            "No Git repository found here. Enter a repository before running mgh repo.",
        );
    }
    env.init_repo();
    env.set_local("mgh.protections", "false");
    env.set_local("mgh.allowed-identity", "stale");
    let before = bytes(env.repo.join(".git/config"));
    env.success(&["repo", "protections", "off"]);
    assert_eq!(bytes(env.repo.join(".git/config")), before);
    fs::write(env.repo.join(".git/config"), "[bad\n").unwrap();
    let o = env.rejects(&["repo"], "git repository discovery failed");
    assert!(!err(&o).contains("No Git repository found"));
}

/// Repository paths containing newlines must retain their full root and shared config path.
#[test]
fn repository_discovery_preserves_newlines_in_paths() {
    let env = Sandbox::new();
    let repo = env.root.join("repo\nwith a newline");
    fs::create_dir(&repo).unwrap();
    assert_ok(
        &env.command("git")
            .current_dir(&repo)
            .args(["init", "-b", "main"])
            .output()
            .unwrap(),
    );
    env.write_identities();
    for bare in [false, true] {
        if bare {
            assert_ok(
                &env.command("git")
                    .current_dir(&repo)
                    .args(["init", "--bare", "bare\nrepository.git"])
                    .output()
                    .unwrap(),
            );
        }
        let cwd = if bare {
            repo.join("bare\nrepository.git")
        } else {
            repo.clone()
        };
        assert_ok(
            &env.mgh(&["repo", "allowed", "add", "personal"])
                .current_dir(&cwd)
                .output()
                .unwrap(),
        );
        let config = if bare {
            cwd.join("config")
        } else {
            cwd.join(".git/config")
        };
        assert!(
            String::from_utf8(bytes(config))
                .unwrap()
                .contains("personal")
        );
        let report = env
            .mgh(&["repo", "status"])
            .current_dir(&cwd)
            .output()
            .unwrap();
        assert_ok(&report);
        assert!(out(&report).contains("with a newline"), "{}", both(&report));
        if bare {
            assert!(out(&report).contains("repository.git"), "{}", both(&report));
        }
    }
}

/// Only explicit permission changes grant access; switching and stale-reference recovery must
/// preserve policy.
#[test]
fn explicit_permissions_switching_and_stale_recovery() {
    let env = Sandbox::new();
    env.init_repo();
    env.setup_defaults();
    env.set_local("mgh.current-identity", "personal");
    env.rejects(&["repo", "check"], "No identities are authorized");
    env.success(&["repo", "allowed", "add", "WORK"]);
    assert_eq!(
        env.git_without_hooks(&["config", "--local", "mgh.current-identity"]),
        "work"
    );
    assert_eq!(env.accounts()["selected"], "alice");
    env.success(&["repo", "allowed", "add", "work"]);
    assert_eq!(
        env.git_without_hooks(&["config", "--local", "--get-all", "mgh.allowed-identity"]),
        "work"
    );
    env.success(&["switch", "personal"]);
    assert_eq!(
        env.git_without_hooks(&["config", "--local", "user.email"]),
        "work@example.com"
    );
    assert_eq!(
        env.git_without_hooks(&["config", "--global", "user.email"]),
        "Alice@example.com"
    );
    env.success(&["repo", "allowed", "add", "personal"]);
    assert_eq!(
        env.git_without_hooks(&["config", "--local", "mgh.current-identity"]),
        "personal"
    );
    env.success(&["switch", "WoRk"]);
    assert_eq!(
        env.git_without_hooks(&["config", "--local", "mgh.current-identity"]),
        "work"
    );
    env.git_without_hooks(&[
        "config",
        "--local",
        "--add",
        "mgh.allowed-identity",
        "stale",
    ]);
    env.git_without_hooks(&[
        "config",
        "--local",
        "--add",
        "mgh.allowed-identity",
        "other-stale",
    ]);
    let local = bytes(env.repo.join(".git/config"));
    let global = bytes(env.home.join(".gitconfig"));
    let auth = env.accounts();
    env.rejects(&["switch", "personal"], "Unknown identity 'other-stale'");
    assert_eq!(bytes(env.repo.join(".git/config")), local);
    assert_eq!(bytes(env.home.join(".gitconfig")), global);
    assert_eq!(env.accounts(), auth);
    env.success(&["repo", "allowed"]);
    env.success(&["repo", "allowed", "remove", "STALE"]);
    assert!(
        env.git_without_hooks(&["config", "--local", "--get-all", "mgh.allowed-identity"])
            .contains("other-stale")
    );
    env.success(&["repo", "allowed", "remove", "other-stale"]);
    env.success(&["repo", "allowed", "remove", "personal"]);
    env.success(&["repo", "allowed", "remove", "work"]);
    assert_eq!(
        env.git_without_hooks(&["config", "--local", "--get-all", "mgh.allowed-identity"]),
        ""
    );
    assert_eq!(
        env.git_without_hooks(&["config", "--local", "mgh.current-identity"]),
        "work"
    );
    env.rejects(&["repo", "check"], "No identities are authorized");
}

/// Repository discovery and shared permissions must work in worktrees, bare repos and relative
/// Git environments.
#[test]
fn linked_worktrees_subdirectories_bare_and_relative_git_environment() {
    let env = Sandbox::new();
    env.init_repo();
    env.setup_defaults();
    env.success(&["repo", "allowed", "add", "personal"]);
    env.commit_without_hooks("Alice Example", "Alice@example.com");
    let tree = env.root.join("linked ' tree");
    env.git_without_hooks(&["worktree", "add", "-b", "linked", tree.to_str().unwrap()]);
    assert_ok(
        &env.mgh(&["repo", "allowed", "add", "work"])
            .current_dir(&tree)
            .output()
            .unwrap(),
    );
    assert_eq!(
        env.git_without_hooks(&["config", "--local", "--get-all", "mgh.allowed-identity"]),
        "personal\nwork"
    );
    env.git_without_hooks(&["config", "extensions.worktreeConfig", "true"]);
    let project = tree.join("project-hooks");
    fs::create_dir_all(&project).unwrap();
    fs::write(project.join("commit-msg"), "#!/bin/sh\nexit 0\n").unwrap();
    let o = env
        .command("git")
        .current_dir(&tree)
        .args(["config", "--worktree", "core.hooksPath", "project-hooks"])
        .output()
        .unwrap();
    assert_ok(&o);
    assert_ok(&env.mgh(&["setup"]).current_dir(&tree).output().unwrap());
    let o = env
        .command("git")
        .current_dir(&tree)
        .args(["config", "--worktree", "mgh.originalHooksPath"])
        .output()
        .unwrap();
    assert_ok(&o);
    assert_eq!(out(&o).trim(), "project-hooks");
    let sub = env.repo.join("deep/sub");
    fs::create_dir_all(&sub).unwrap();
    let o = env.mgh(&["repo"]).current_dir(&sub).output().unwrap();
    assert_ok(&o);
    assert!(out(&o).contains(env.repo.to_str().unwrap()));
    let o = env
        .mgh(&["repo", "allowed", "list"])
        .current_dir(&sub)
        .env("GIT_DIR", "../../.git")
        .env("GIT_WORK_TREE", "../..")
        .output()
        .unwrap();
    assert_ok(&o);
    let bare = env.root.join("bare.git");
    env.git_without_hooks(&["init", "--bare", bare.to_str().unwrap()]);
    let o = env
        .mgh(&["repo", "allowed", "add", "work"])
        .current_dir(&bare)
        .output()
        .unwrap();
    assert_ok(&o);
}

/// Inspection must not repair state, and account-query failures must follow each command's
/// documented policy.
#[test]
fn inspection_is_readonly_and_error_policies_differ() {
    let env = Sandbox::new();
    env.init_repo();
    env.setup_defaults();
    env.success(&["repo", "allowed", "add", "personal"]);
    env.success(&["settings", "welcome", "on"]);
    let global = bytes(env.home.join(".gitconfig"));
    let local = bytes(env.repo.join(".git/config"));
    let config = bytes(&env.config);
    let auth = env.accounts();
    for args in [
        vec!["status"],
        vec!["repo"],
        vec!["doctor"],
        vec!["internal", "welcome"],
        vec!["identity"],
    ] {
        env.success(&args);
    }
    assert_eq!(bytes(env.home.join(".gitconfig")), global);
    assert_eq!(bytes(env.repo.join(".git/config")), local);
    assert_eq!(bytes(&env.config), config);
    assert_eq!(env.accounts(), auth);
    let o = env
        .mgh(&["identity"])
        .env("MGH_QUERY_FAIL", "1")
        .output()
        .unwrap();
    assert_ok(&o);
    assert!(out(&o).contains("account query unavailable"));
    let o = env
        .mgh(&["status"])
        .env("MGH_QUERY_FAIL", "1")
        .output()
        .unwrap();
    assert!(!o.status.success());
    assert!(err(&o).contains("account query exploded"));
    let o = env
        .mgh(&["status", "--full"])
        .env("MGH_FULL_FAIL", "1")
        .output()
        .unwrap();
    assert!(!o.status.success());
    assert!(out(&o).contains("native auth stdout"));
    assert!(err(&o).contains("native auth stderr"));
    assert!(err(&o).contains("GitHub authentication check failed"));
    let o = env
        .mgh(&["internal", "welcome"])
        .env("MGH_QUERY_FAIL", "1")
        .output()
        .unwrap();
    assert_ok(&o);
    let o = env
        .mgh(&["internal", "welcome"])
        .env("MGH_SELECTED_FAIL", "1")
        .output()
        .unwrap();
    assert!(!o.status.success());
    fs::write(&env.config, "bad").unwrap();
    let o = env.success(&["status"]);
    assert!(out(&o).contains("GitHub account"));
    assert!(out(&o).contains("alice (Active)"));
    assert!(!out(&o).contains("Active GitHub account"));
    assert!(out(&o).contains("invalid identity configuration"));
    env.success(&["internal", "welcome"]);
    fs::remove_file(env.state.join("hooks/pre-commit")).unwrap();
    let o = env
        .mgh(&["doctor"])
        .env("MGH_QUERY_FAIL", "1")
        .output()
        .unwrap();
    assert!(!o.status.success());
    assert!(out(&o).contains("Missing launcher"));
    assert!(out(&o).contains("invalid identity configuration"));
    assert!(out(&o).contains("account query exploded"));
    assert!(!env.state.join("hooks/pre-commit").exists());
}

/// A failed account switch must report earlier writes without claiming the operation was
/// unchanged.
#[test]
fn partial_switch_failure_reports_completed_writes_only() {
    let env = Sandbox::new();
    env.write_identities();
    let o = env
        .mgh(&["switch", "personal"])
        .env("MGH_SWITCH_FAIL", "1")
        .output()
        .unwrap();
    assert!(!o.status.success());
    assert!(out(&o).contains("Global Git config updated"));
    assert!(!out(&o).contains("unchanged"));
    assert!(env.state.join("identities/git-personal.conf").exists());
    let global = bytes(env.home.join(".gitconfig"));
    let o = env
        .mgh(&["switch", "personal"])
        .env("MGH_SWITCH_FAIL", "1")
        .output()
        .unwrap();
    assert!(!o.status.success());
    assert!(!out(&o).contains("Global Git config unchanged"));
    assert_eq!(bytes(env.home.join(".gitconfig")), global);
}

/// An empty permission marker must block access, including when mixed with a valid identity
/// name.
#[test]
fn explicit_empty_permission_marker_never_grants_access() {
    let env = Sandbox::new();
    env.init_repo();
    env.setup_defaults();
    env.set_local("mgh.allowed-identity", "");
    env.git_without_hooks(&[
        "config",
        "--local",
        "--add",
        "mgh.allowed-identity",
        "personal",
    ]);
    env.set_local("mgh.current-identity", "personal");
    env.rejects(&["repo", "check"], "No identities are authorized");
    let o = env.success(&["repo", "allowed"]);
    assert!(out(&o).contains("none selected"));
    env.rejects(
        &["identity", "new", "", "--username", "bob", "--email", "b@e"],
        "Invalid identity name",
    );
    env.success(&["repo", "allowed", "add", "personal"]);
    assert_eq!(
        env.git_without_hooks(&["config", "--local", "--get-all", "mgh.allowed-identity"]),
        "personal"
    );
}

/// Protections default on, and a global preference cannot disable a repository's checks.
#[test]
fn protection_boolean_defaults_and_global_exception_does_not_authorize() {
    let env = Sandbox::new();
    env.init_repo();
    env.setup_defaults();
    env.git_without_hooks(&["config", "--global", "mgh.protections", "false"]);
    let o = env.success(&["repo", "protections"]);
    assert!(out(&o).contains("ON"));
    env.set_local("mgh.protections", "invalid");
    env.rejects(&["repo", "protections"], "fail closed");
    env.write_hook("pre-commit", "echo unsafe > called", true);
    assert!(!env.hook("pre-commit", b"", &[]).status.success());
    assert!(!env.repo.join("called").exists());
    env.success(&["repo", "protections", "off"]);
    fs::remove_file(&env.config).unwrap();
    assert_ok(&env.hook("pre-commit", b"", &[]));
    assert!(env.repo.join("called").exists());
}

/// A later email-write failure must preserve and report the completed name change and its
/// provenance.
#[test]
fn completed_name_write_survives_later_email_failure_and_retains_provenance() {
    let env = Sandbox::new();
    env.write_identities();
    env.success(&["setup"]);
    let fake = env.bin.join("git");
    fs::write(
        &fake,
        r#"#!/usr/bin/env python3
import os
import sys

args = sys.argv[1:]
if '--replace-all' in args and 'user.email' in args and os.getenv('MGH_FAIL_EMAIL'):
    print('email write fixture failed', file=sys.stderr)
    sys.exit(4)
os.execv(os.environ['MGH_TEST_REAL_GIT'], ['git'] + args)
"#,
    )
    .unwrap();
    fs::set_permissions(fake, fs::Permissions::from_mode(0o700)).unwrap();
    let o = env
        .mgh(&["switch", "PeRsOnAl"])
        .env("MGH_FAIL_EMAIL", "1")
        .output()
        .unwrap();
    assert!(!o.status.success());
    assert!(out(&o).contains("Global Git config updated by mgh switch PeRsOnAl"));
    assert!(err(&o).contains("email write fixture failed"));
    assert_eq!(
        env.git_without_hooks(&["config", "--global", "user.name"]),
        "Alice Example"
    );
    assert!(
        !env.native_git(&["config", "--global", "--get", "user.email"])
            .status
            .success()
    );
    assert_eq!(env.accounts()["selected"], "alice");
    assert!(
        fs::read_to_string(env.home.join(".gitconfig"))
            .unwrap()
            .contains("Generated by multigh (mgh switch PeRsOnAl)")
    );
}

/// Doctor must collect independent problems in one run without repairing configuration or
/// hooks.
#[test]
fn doctor_accumulates_tool_version_config_auth_and_hook_errors() {
    let env = Sandbox::new();
    fs::write(&env.config, "{}").unwrap();
    let fake = env.bin.join("git");
    fs::write(
        &fake,
        r#"#!/bin/sh
if [ "$1" = --version ]; then
    echo 'git version 2.44.0'
    exit 0
fi
exec "$MGH_TEST_REAL_GIT" "$@"
"#,
    )
    .unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o700)).unwrap();
    let o = env
        .mgh(&["doctor"])
        .env("MGH_QUERY_FAIL", "1")
        .output()
        .unwrap();
    assert!(!o.status.success());
    let text = out(&o);
    for expected in [
        "Git 2.45+ required",
        "Identity configuration is empty",
        "account query exploded",
        "Missing launcher",
    ] {
        assert!(text.contains(expected), "{text}");
    }
    assert!(!env.state.exists());
    fs::remove_file(&fake).unwrap();
    let o = env.mgh(&["doctor"]).env("PATH", "").output().unwrap();
    assert!(!o.status.success());
    let text = out(&o);
    assert!(text.contains("Cannot execute"));
    assert!(text.contains("gh"));
    assert!(text.contains("git"));
}

/// Protection settings must use Git's boolean semantics rather than a smaller custom parser.
#[test]
fn git_native_boolean_parsing_includes_empty_and_numeric_values() {
    let env = Sandbox::new();
    env.init_repo();
    for (value, expected) in [
        ("", "OFF"),
        ("2", "ON"),
        ("TRUE", "ON"),
        ("no", "OFF"),
        ("-1", "ON"),
    ] {
        env.set_local("mgh.protections", value);
        let o = env.success(&["repo", "protections"]);
        assert!(out(&o).contains(expected), "{value}: {}", both(&o));
    }
}

/// An unavailable previous-account query must not prevent an explicitly requested switch.
#[test]
fn switch_succeeds_when_previous_account_query_fails() {
    let env = Sandbox::new();
    env.setup_defaults();
    let o = env
        .mgh(&["switch", "work"])
        .env("MGH_SELECTED_FAIL", "1")
        .output()
        .unwrap();
    assert_ok(&o);
    assert_eq!(env.accounts()["selected"], "alice-work");
    assert_eq!(
        env.git_without_hooks(&["config", "--global", "user.email"]),
        "work@example.com"
    );
}

/// Status must distinguish expired or absent accounts from real usernames that resemble display
/// placeholders.
#[test]
fn status_expired_missing_hosts_and_placeholder_logins() {
    let env = Sandbox::new();
    env.setup_defaults();
    env.set_accounts(json!({
        "selected": "alice",
        "accounts": [
            {
                "login": "alice",
                "active": true,
                "state": "error",
                "error": "expired fixture"
            }
        ]
    }));
    let o = env.success(&["status"]);
    assert!(out(&o).contains("expired fixture"));
    assert!(out(&o).contains("alice"));
    let o = env
        .mgh(&["status"])
        .env("MGH_HOST_MISSING", "1")
        .output()
        .unwrap();
    assert!(!o.status.success());
    assert!(err(&o).contains("No active GitHub account"));
    env.set_accounts(json!({
        "selected": "foreign",
        "accounts": [
            {
                "login": "foreign",
                "active": true,
                "state": "success"
            },
            {
                "login": "alice",
                "active": false,
                "state": "success"
            }
        ]
    }));
    let text = out(&env.success(&["status"]));
    assert!(text.starts_with("\n  Identities\n"), "{text}");
    assert!(
        text.contains("\n\n  Unconfigured GitHub accounts\n"),
        "{text}"
    );
    assert_eq!(text.matches("foreign (Active)").count(), 1, "{text}");
    assert_eq!(text.matches("(Active)").count(), 1, "{text}");
    assert!(!text.contains("Active GitHub account"), "{text}");
    let env = Sandbox::new();
    fs::write(
        &env.config,
        r#"{
  "example": {
    "username": "unavailable",
    "commit": {"name": "not configured", "email": "valid@example.com"}
  }
}"#,
    )
    .unwrap();
    env.set_accounts(json!({
        "selected": "unavailable",
        "accounts": [
            {
                "login": "unavailable",
                "active": true,
                "state": "success"
            }
        ]
    }));
    env.success(&["settings", "welcome", "on"]);
    let o = env.success(&["internal", "welcome"]);
    assert!(out(&o).contains("example"));
    let o = env.success(&["identity"]);
    assert!(out(&o).contains("(Active)"));
}
