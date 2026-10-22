//! Repository entry, autoswitch, permission pickers and the read-only welcome message.

use super::support::*;
use serde_json::Value;
use serde_json::json;
use std::fs;

/// Quiet entry must stay silent; unavailable prompts must explain recovery and retain saved
/// preferences.
#[test]
fn entry_all_off_and_noninteractive_guidance_and_partial_preferences() {
    let env = Sandbox::new();
    env.init_repo();
    env.set_local("mgh.protections", "false");
    env.success(&["internal", "enter"]);
    assert!(!env.root.join("gh.log").exists());
    assert!(!env.state.exists());
    env.write_identities();
    env.set_local("mgh.protections", "true");
    let o = env.success(&["internal", "enter"]);
    assert!(out(&o).contains("mgh repo allowed add <identity>"));
    assert!(out(&o).contains("remain blocked"));
    assert!(
        !env.native_git(&["config", "--local", "--get-all", "mgh.allowed-identity"])
            .status
            .success()
    );
    env.success(&["repo", "allowed", "add", "personal"]);
    env.success(&["repo", "allowed", "add", "work"]);
    let o = env.success(&["settings", "autoswitch", "on"]);
    assert!(out(&o).contains("mgh switch <allowed-identity>"));
    env.set_local("mgh.allowed-identity", "stale");
    env.rejects(&["settings", "autoswitch", "on"], "Unknown identity");
    assert!(env.state.join("autoswitch-enabled").exists());
}

/// The terminal checklist must require a selection, scroll and offer only allowed autoswitch
/// identities.
#[test]
fn pty_checklists_empty_selection_scroll_and_autoswitch_choices() {
    let env = Sandbox::new();
    env.init_repo();
    env.setup_defaults();
    env.driver();
    let exe = env!("CARGO_BIN_EXE_mgh");
    let o = env.pty(
        exe,
        &["repo", "allowed", "update"],
        json!([
            ["Which identities may use this repository?", "\r"],
            ["Input required", " \r"]
        ]),
    );
    assert_ok(&o);
    assert_eq!(
        env.git_without_hooks(&["config", "--local", "mgh.allowed-identity"]),
        "personal"
    );
    let before = bytes(env.repo.join(".git/config"));
    let o = env.pty(
        exe,
        &["repo", "allowed", "update"],
        json!([["Which identities may use this repository?", "\u{1b}"]]),
    );
    assert!(!o.status.success());
    assert_eq!(bytes(env.repo.join(".git/config")), before);
    env.success(&["repo", "allowed", "add", "work"]);
    env.success(&["settings", "autoswitch", "on"]);
    let o = env.pty(
        exe,
        &["internal", "enter"],
        json!([["Which allowed identity should be active?", "\u{1b}[B\r"]]),
    );
    assert_ok(&o);
    assert_eq!(env.accounts()["selected"], "alice-work");
    let mut value = serde_json::Map::new();
    for n in 0..12 {
        value.insert(
            format!("identity{n:02}"),
            json!({"username":format!("user-{n}"),"commit":{"email":format!("e{n}@mail")}}),
        );
    }
    fs::write(&env.config, Value::Object(value).to_string()).unwrap();
    let o = env.pty(
        exe,
        &["repo", "allowed", "update"],
        json!([[
            "Which identities may use this repository?",
            format!("{} \r", "\u{1b}[B".repeat(10))
        ]]),
    );
    assert_ok(&o);
    assert_eq!(
        env.git_without_hooks(&["config", "--local", "mgh.allowed-identity"]),
        "identity10"
    );
}

/// Sole-identity autoswitch must skip an already selected account and otherwise select the
/// allowed one.
#[test]
fn sole_identity_autoswitch_skips_an_already_selected_account() {
    let env = Sandbox::new();
    env.init_repo();
    env.setup_defaults();
    env.success(&["repo", "allowed", "add", "personal"]);
    env.set_local("user.name", "Deliberately Different");
    let before = bytes(env.repo.join(".git/config"));
    env.success(&["settings", "autoswitch", "on"]);
    assert_eq!(bytes(env.repo.join(".git/config")), before);
    env.success(&["repo", "allowed", "remove", "personal"]);
    env.success(&["repo", "allowed", "add", "work"]);
    env.success(&["internal", "enter"]);
    assert_eq!(env.accounts()["selected"], "alice-work");
    assert_eq!(
        env.git_without_hooks(&["config", "--local", "user.name"]),
        "Alice Work"
    );
    env.success(&["settings", "verbose", "off"]);
    let o = env.success(&["internal", "enter"]);
    assert!(o.stdout.is_empty());
    assert!(o.stderr.is_empty());
}

/// Autoswitch must show configured identity transitions and hide low-level internal Git change
/// reports.
#[test]
fn entry_reports_identity_transition_without_internal_git_details() {
    let env = Sandbox::new();
    env.init_repo();
    env.setup_defaults();
    env.success(&["repo", "allowed", "add", "work"]);
    env.success(&["settings", "autoswitch", "on"]);
    env.success(&["switch", "personal"]);
    env.git_without_hooks(&["config", "--global", "--unset-all", "core.hooksPath"]);
    let o = env.success(&["internal", "enter"]);
    let text = out(&o);
    assert!(
        text.starts_with("\n  Autoswitch [personal → work]\n"),
        "{text}"
    );
    assert!(!text.contains("Global Git config"), "{text}");
    assert!(!text.contains("mgh internal"), "{text}");
    assert!(!text.contains("user.name"), "{text}");
    assert!(!text.contains("user.email"), "{text}");
    assert!(text.contains("    GitHub username"), "{text}");
    assert!(text.contains("alice → alice-work"), "{text}");
    assert_eq!(
        env.git_without_hooks(&["config", "--global", "user.name"]),
        "Alice Work"
    );
    assert_eq!(
        env.git_without_hooks(&["config", "--global", "user.email"]),
        "work@example.com"
    );
    assert_eq!(
        env.git_without_hooks(&["config", "--global", "core.hooksPath"]),
        env.state.join("hooks").to_str().unwrap()
    );
    assert!(!out(&o).contains("Current repository"));
    assert_eq!(env.accounts()["selected"], "alice-work");

    env.set_accounts(json!({
        "selected": "foreign",
        "accounts": [
            {
                "login": "foreign",
                "active": true,
                "state": "success"
            },
            {
                "login": "alice-work",
                "active": false,
                "state": "success"
            }
        ]
    }));
    let text = out(&env.success(&["internal", "enter"]));
    assert!(
        text.starts_with("\n  Autoswitch [foreign → work]\n"),
        "{text}"
    );
    assert!(!text.contains("mgh internal"), "{text}");

    env.set_accounts(json!({
        "selected": null,
        "accounts": [
            {
                "login": "alice-work",
                "active": false,
                "state": "success"
            }
        ]
    }));
    let text = out(&env.success(&["internal", "enter"]));
    assert!(
        text.starts_with("\n  Autoswitch [not configured → work]\n"),
        "{text}"
    );
    assert!(!text.contains("mgh internal"), "{text}");
}

/// Multiple allowed identities require a choice even when one is active; cancellation must
/// preserve state.
#[test]
fn multiple_autoswitch_always_prompts_and_cancel_preserves_state() {
    let env = Sandbox::new();
    env.init_repo();
    env.setup_defaults();
    env.success(&["repo", "allowed", "add", "personal"]);
    env.success(&["repo", "allowed", "add", "work"]);
    env.success(&["settings", "autoswitch", "on"]);
    env.driver();
    let exe = env!("CARGO_BIN_EXE_mgh");
    let global = bytes(env.home.join(".gitconfig"));
    let local = bytes(env.repo.join(".git/config"));
    let account = env.accounts();
    let o = env.pty(
        exe,
        &["internal", "enter"],
        json!([["Which allowed identity should be active?", "\u{1b}"]]),
    );
    assert_ok(&o);
    assert!(out(&o).contains("Operation cancelled"));
    assert_eq!(bytes(env.home.join(".gitconfig")), global);
    assert_eq!(bytes(env.repo.join(".git/config")), local);
    assert_eq!(env.accounts(), account);
    let o = env.pty(
        exe,
        &["internal", "enter"],
        json!([["Which allowed identity should be active?", "\r"]]),
    );
    assert_ok(&o);
    assert_eq!(env.accounts()["selected"], "alice");
}

/// Welcome must explain permission and commit mismatches without changing account or Git
/// settings.
#[test]
fn welcome_mismatch_guidance_preserves_all_observed_state() {
    let env = Sandbox::new();
    env.init_repo();
    env.setup_defaults();
    env.success(&["repo", "allowed", "add", "work"]);
    env.set_local("user.name", "Wrong");
    env.set_local("user.email", "wrong@else");
    env.success(&["settings", "welcome", "on"]);
    let local = bytes(env.repo.join(".git/config"));
    let global = bytes(env.home.join(".gitconfig"));
    let account = env.accounts();
    let o = env.success(&["internal", "welcome"]);
    let text = out(&o);
    assert!(text.contains("This repository needs: mgh switch work"));
    assert!(text.contains("Commit details need: mgh switch personal"));
    assert_eq!(bytes(env.repo.join(".git/config")), local);
    assert_eq!(bytes(env.home.join(".gitconfig")), global);
    assert_eq!(env.accounts(), account);
}
