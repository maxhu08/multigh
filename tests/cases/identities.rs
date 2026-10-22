//! Onboarding and identity changes, simulated sign-in, concurrent edits and partial completion.

use super::support::*;
use serde_json::Value;
use serde_json::json;
use std::fs;
use std::os::unix::fs::PermissionsExt;

/// Setup explains missing identities, preserves preferences and shows usable shell
/// instructions.
#[test]
fn onboarding_preferences_and_no_home_paths() {
    let env = Sandbox::new();
    env.rejects(&["setup"], "Setup needs a first identity");
    assert!(!env.config.exists());
    fs::write(&env.config, "{}").unwrap();
    env.rejects(&["setup"], "mgh identity new --username");
    env.success(&["settings", "verbose", "on"]);
    env.success(&["settings", "welcome", "on"]);
    env.success(&["settings", "autoswitch", "on"]);
    assert!(!env.root.join("gh.log").exists());
    env.setup_defaults();
    assert!(env.state.join("setup-complete-enabled").exists());
    env.success(&["settings", "verbose", "off"]);
    let setup = out(&env.success(&["setup"]));
    assert!(setup.contains("\n\n  Shell integration\n"), "{setup}");
    assert!(setup.contains("Add the matching command to your shell's configuration file."));
    assert!(setup.contains("repository entry checks and tab completion"));
    assert!(setup.contains("restart your shell or run its command above"));
    assert!(setup.contains("\n\n  Next steps\n"), "{setup}");
    assert!(
        setup.ends_with("\n\n    Global Git config unchanged\n\n"),
        "{setup}"
    );
    assert!(!setup.contains("\n\n\n"), "{setup}");

    let lines = setup.lines().collect::<Vec<_>>();
    for (config, command) in [
        ("~/.config/fish/config.fish", "mgh shell init fish | source"),
        ("~/.bashrc", "eval \"$(mgh shell init bash)\""),
        ("~/.zshrc", "eval \"$(mgh shell init zsh)\""),
    ] {
        let index = lines
            .iter()
            .position(|line| line.ends_with(config))
            .unwrap();
        assert!(
            lines[index + 1].starts_with("      Add this line"),
            "{setup}"
        );
        assert!(lines[index + 1].ends_with(command), "{setup}");
    }
    assert!(!env.state.join("verbose-enabled").exists());
    assert!(env.state.join("welcome-enabled").exists());
    let o = env
        .mgh(&["identity", "list"])
        .env_remove("HOME")
        .env_remove("USERPROFILE")
        .env_remove("XDG_CONFIG_HOME")
        .output()
        .unwrap();
    assert!(!o.status.success());
    assert!(err(&o).contains("Cannot locate your home directory"));
    let p = env.config.to_str().unwrap();
    let o = env
        .mgh(&["identity", "list", "--config", p])
        .env_remove("HOME")
        .env_remove("USERPROFILE")
        .output()
        .unwrap();
    assert_ok(&o);
}

/// Creation reuses an existing login, normalizes names and saves private files with the
/// expected defaults.
#[test]
fn creation_existing_login_and_case_defaults() {
    let env = Sandbox::new();
    let o = env.success(&[
        "identity",
        "new",
        "Personal",
        "--username",
        " alice ",
        "--email",
        " Alice@Example.com ",
    ]);
    assert!(out(&o).contains("Identity saved in"));
    let v: Value = serde_json::from_slice(&bytes(&env.config)).unwrap();
    assert_eq!(v["personal"]["commit"]["name"], "alice");
    assert_eq!(v["personal"]["username"], "alice");
    assert_eq!(
        env.git_without_hooks(&["config", "--global", "user.email"]),
        "Alice@Example.com"
    );
    let log = fs::read_to_string(env.root.join("gh.log")).unwrap();
    assert!(!log.contains("\"login\""));
    assert_eq!(
        fs::metadata(&env.config).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        fs::metadata(&env.state).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert!(env.state.join("verbose-enabled").exists());
}

/// Failed or conflicting sign-in must preserve the original config and remove abandoned pending
/// files.
#[test]
fn authentication_failures_concurrent_changes_and_pending_cleanup() {
    for failure in ["query", "json", "login", "wrong", "concurrent", "delete"] {
        let env = Sandbox::new();
        env.write_identities();
        let before = bytes(&env.config);
        let mut cmd = env.mgh(&[
            "identity",
            "new",
            "new",
            "--username",
            "new-user",
            "--email",
            "n@e",
        ]);
        match failure {
            "query" => {
                cmd.env("MGH_QUERY_FAIL", "1");
            }
            "json" => {
                cmd.env("MGH_QUERY_BAD_JSON", "1");
            }
            "login" => {
                cmd.env("MGH_LOGIN_FAIL", "1");
            }
            "wrong" => {
                cmd.env("MGH_LOGIN_ACCOUNT", "wrong");
            }
            "concurrent" => {
                cmd.env("MGH_LOGIN_ACCOUNT", "new-user")
                    .env("MGH_EDIT_CONFIG", &env.config);
            }
            "delete" => {
                cmd.env("MGH_LOGIN_ACCOUNT", "new-user")
                    .env("MGH_DELETE_CONFIG", &env.config);
            }
            _ => unreachable!(),
        }
        let o = cmd.output().unwrap();
        assert!(!o.status.success(), "{failure}: {}", both(&o));
        if failure == "concurrent" {
            assert_eq!(bytes(&env.config), b"{\"concurrent\":true}\n");
            assert!(err(&o).contains("Configuration changed while signing in"));
        } else if failure == "delete" {
            assert!(!env.config.exists());
        } else {
            assert_eq!(bytes(&env.config), before);
        }
        assert!(!env.home.join(".gitconfig").exists());
        assert!(
            !fs::read_dir(env.config.parent().unwrap())
                .unwrap()
                .any(|f| f.unwrap().file_name().to_string_lossy().contains("pending"))
        );
        if matches!(failure, "query" | "json") {
            assert!(
                !fs::read_to_string(env.root.join("gh.log"))
                    .unwrap()
                    .contains("\"login\"")
            );
        }
    }
}

/// Simulated login can save an identity; later setup failure must keep it and provide recovery
/// commands.
#[test]
fn simulated_login_saves_identity_before_later_setup_failure() {
    let env = Sandbox::new();
    let o = env
        .mgh(&[
            "identity",
            "new",
            "new",
            "--username",
            "new-user",
            "--email",
            "n@e",
        ])
        .env("MGH_LOGIN_ACCOUNT", "new-user")
        .output()
        .unwrap();
    assert_ok(&o);
    assert_eq!(env.accounts()["selected"], "new-user");
    let env = Sandbox::new();
    let shared = env.state.join("hooks");
    fs::create_dir_all(&shared).unwrap();
    fs::write(shared.join("pre-commit"), "foreign").unwrap();
    let custom = env.root.join("my ' identities.jsonc");
    let o = env.run(&[
        "--config",
        custom.to_str().unwrap(),
        "identity",
        "new",
        "p",
        "--username",
        "alice",
        "--email",
        "a@b",
    ]);
    assert!(!o.status.success());
    assert!(custom.exists());
    assert!(err(&o).contains("Identity saved in"));
    assert!(err(&o).contains("--config"));
    assert!(err(&o).contains("switch <identity>"));
    assert_eq!(bytes(shared.join("pre-commit")), b"foreign");
}

/// Editing JSONC must preserve comments, CRLF, extra emails, account selection and repository
/// permissions.
#[test]
fn jsonc_edit_preserves_formatting_additional_emails_and_other_state() {
    let env = Sandbox::new();
    let original = r#"// heading
{
  "PERSONAL": { // keep me
    "username": "alice",
    "commit": {
      "name": "Alice \u0045xample",
      "email": "Alice@example.com",
      "additional_emails": ["123+alice@users.noreply.github.com",],
    },
  },
}
"#
    .replace('\n', "\r\n");
    fs::write(&env.config, &original).unwrap();
    env.init_repo();
    env.set_local("mgh.allowed-identity", "personal");
    let local = bytes(env.repo.join(".git/config"));
    env.success(&["identity", "edit", "personal", "--email", "new@example.com"]);
    let text = fs::read_to_string(&env.config).unwrap();
    assert!(text.contains("// keep me\r\n"));
    assert!(text.contains("Alice \\u0045xample"));
    assert!(text.contains("additional_emails"));
    assert!(text.contains("new@example.com"));
    assert!(!text.replace("\r\n", "").contains('\n'));
    assert_eq!(bytes(env.repo.join(".git/config")), local);
    assert_eq!(env.accounts()["selected"], "alice");
    env.success(&["identity", "remove", "PERSONAL"]);
    env.success(&["identity"]);
    assert_eq!(
        env.git_without_hooks(&["config", "--local", "mgh.allowed-identity"]),
        "personal"
    );
    assert_eq!(env.accounts()["selected"], "alice");
}

/// Real terminal forms must accept defaults, save edits and leave files intact when cancelled.
#[test]
fn pty_identity_forms_defaults_edit_and_cancellation() {
    let env = Sandbox::new();
    env.driver();
    let exe = env!("CARGO_BIN_EXE_mgh");
    let o = env.pty(
        exe,
        &["identity", "new"],
        json!([
            ["Identity name", "Personal\r"],
            ["GitHub username", "alice\r"],
            ["Commit name (default: alice)", "\r"],
            ["Commit email", "a@b\r"]
        ]),
    );
    assert_ok(&o);
    assert!(env.config.exists());
    let before = bytes(&env.config);
    let o = env.pty(
        exe,
        &["identity", "edit", "personal"],
        json!([
            ["GitHub username (default: alice)", "\r"],
            ["Commit email (default: a@b)", "\r"],
            ["Commit name (default: alice)", "\r"]
        ]),
    );
    assert_ok(&o);
    assert_eq!(bytes(&env.config), before);
    for key in ["\u{1b}", "\u{3}"] {
        let o = env.pty(
            exe,
            &["identity", "edit", "personal"],
            json!([["GitHub username", key]]),
        );
        assert!(!o.status.success());
        assert!(
            out(&o).contains("Operation cancelled."),
            "key={key:?}: {}",
            both(&o)
        );
        assert_eq!(bytes(&env.config), before);
    }
}

/// Removing identities must clean up owned Git rules without deleting foreign entries or
/// leaving comments behind.
#[test]
fn removal_cleans_only_owned_rules_and_no_provenance_debris() {
    let env = Sandbox::new();
    env.setup_defaults();
    let key = "includeif.hasconfig:remote.*.url:https://github.com/alice/**.path";
    env.git_without_hooks(&["config", "--global", "--add", key, "/foreign.conf"]);
    let unrelated = env.state.join("identities/unrelated");
    fs::write(&unrelated, "preserve").unwrap();
    env.success(&["identity", "remove", "personal"]);
    let global = fs::read_to_string(env.home.join(".gitconfig")).unwrap();
    assert!(!global.contains("git-personal.conf"));
    assert!(global.contains("/foreign.conf"));
    assert_eq!(bytes(unrelated), b"preserve");
    assert!(
        global
            .lines()
            .all(|line| !line.trim_start().starts_with("# Generated by multigh"))
    );
}

/// Expired login can be renewed, while concurrent config creation must not be overwritten.
#[test]
fn authentication_login_expiry_and_concurrent_creation_keep_originals() {
    let env = Sandbox::new();
    env.write_identities();
    env.set_accounts(json!({
        "selected": "alice",
        "accounts": [
            {
                "login": "alice",
                "active": true,
                "state": "error",
                "error": "expired"
            }
        ]
    }));
    let o = env
        .mgh(&["identity", "edit", "personal", "--name", "Updated"])
        .env("MGH_LOGIN_ACCOUNT", "ALICE")
        .output()
        .unwrap();
    assert_ok(&o);
    assert!(
        fs::read_to_string(env.root.join("gh.log"))
            .unwrap()
            .contains("\"login\"")
    );
    let env = Sandbox::new();
    let o = env
        .mgh(&[
            "identity",
            "new",
            "x",
            "--username",
            "new-user",
            "--email",
            "n@e",
        ])
        .env("MGH_LOGIN_ACCOUNT", "new-user")
        .env("MGH_EDIT_CONFIG", &env.config)
        .output()
        .unwrap();
    assert!(!o.status.success());
    assert_eq!(bytes(&env.config), b"{\"concurrent\":true}\n");
    assert!(err(&o).contains("Configuration changed while signing in"));
}

/// Creation must report completed changes once and keep a saved identity if selection later
/// fails.
#[test]
fn creation_reports_global_changes_once_and_late_failure_keeps_saved_identity() {
    let env = Sandbox::new();
    let o = env.success(&[
        "identity",
        "new",
        "personal",
        "--username",
        "alice",
        "--email",
        "a@b",
    ]);
    assert_eq!(out(&o).matches("Global Git config updated by").count(), 1);
    let env = Sandbox::new();
    let o = env
        .mgh(&[
            "identity",
            "new",
            "personal",
            "--username",
            "alice",
            "--email",
            "a@b",
        ])
        .env("MGH_SWITCH_FAIL", "1")
        .output()
        .unwrap();
    assert!(!o.status.success());
    assert!(env.config.exists());
    assert_eq!(out(&o).matches("Global Git config updated by").count(), 1);
    assert!(err(&o).contains("Identity saved in"));
    assert!(env.state.join("hooks/pre-commit").exists());
    assert!(
        !env.native_git(&["config", "--global", "user.name"])
            .status
            .success()
    );
}

/// Changing one field must leave omitted values and unrelated formatting byte-for-byte intact.
#[test]
fn partial_edits_preserve_omitted_and_unselected_fields_byte_for_byte() {
    let env = Sandbox::new();
    let original = r#"{
  "Personal": {
    "username": " alice ",
    "commit": {"email": " Alice@Example.com ", "additional_emails": [" A@B " ]}
  }
}
"#;
    fs::write(&env.config, original).unwrap();
    env.success(&["identity", "edit", "personal", "--email", "new@example.com"]);
    assert_eq!(
        fs::read_to_string(&env.config).unwrap(),
        original.replace(" Alice@Example.com ", "new@example.com")
    );
    env.set_accounts(json!({
        "selected": "alice",
        "accounts": [
            {
                "login": "alice",
                "active": true,
                "state": "success"
            },
            {
                "login": "bob",
                "active": false,
                "state": "success"
            }
        ]
    }));
    let before = fs::read_to_string(&env.config).unwrap();
    env.success(&["identity", "edit", "personal", "--username", "bob"]);
    assert_eq!(
        fs::read_to_string(&env.config).unwrap(),
        before.replace(" alice ", "bob")
    );
    assert_eq!(env.accounts()["selected"], "alice");
    assert!(
        fs::read_to_string(env.state.join("identities/git-personal.conf"))
            .unwrap()
            .contains("bob")
    );
}
