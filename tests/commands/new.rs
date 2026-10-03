use crate::support::{ACCOUNTS, Sandbox, terminal};
use std::{fs, os::unix::fs::PermissionsExt};

const LOGIN: &str = r#"[{"login":"carol","active":true,"state":"success"}]"#;
const ARGS: &[&str] = &[
    "new",
    "WORK",
    "--username",
    "carol",
    "--email",
    "carol@example.com",
];

#[test]
fn new_authenticates_appends_preserves_existing_text_and_sets_up_the_identity() {
    let sandbox = Sandbox::new();
    let original = format!("# My accounts\n{ACCOUNTS}");

    sandbox.write("config/multigh/accounts.conf", &original);
    sandbox.write("login-accounts-json", LOGIN);

    let output = sandbox.ok("mgh", ARGS);
    let accounts = fs::read_to_string(sandbox.path("config/multigh/accounts.conf")).unwrap();
    let calls = fs::read_to_string(sandbox.path("gh-calls")).unwrap();

    assert!(accounts.starts_with(&original));
    assert!(accounts.contains("[work]\nusername = carol\nname = carol\nemail = carol@example.com"));
    assert_eq!(
        calls
            .matches("auth login --hostname github.com --web")
            .count(),
        1
    );
    assert!(calls.contains("auth switch --hostname github.com --user carol"));
    assert!(output.contains("GitHub browser login") && output.contains("Account added · WORK"));
    assert!(
        output.contains(
            sandbox
                .path("config/multigh/accounts.conf")
                .to_str()
                .unwrap()
        )
    );
    assert_eq!(
        sandbox.ok("git", &["config", "--global", "user.email"]),
        "carol@example.com\n"
    );
    assert_eq!(
        sandbox.ok("git", &["config", "--global", "user.name"]),
        "carol\n"
    );
    assert!(
        sandbox
            .ok("git", &["config", "--global", "--list"])
            .contains("carol/**")
    );
    assert!(
        sandbox
            .path("state/multigh/identities/git-work.conf")
            .is_file()
    );
    assert_eq!(
        fs::metadata(sandbox.path("config/multigh/accounts.conf"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );

    for mode in ["protections", "verbose"] {
        assert!(
            sandbox
                .path(&format!("state/multigh/{mode}-enabled"))
                .is_file()
        );
    }

    assert!(
        !sandbox
            .run("git", &["config", "--get-all", "mgh.allowedAccount"])
            .status
            .success()
    );
}

#[test]
fn new_reuses_an_existing_login_and_optionally_authorizes_the_repository() {
    let sandbox = Sandbox::new();

    sandbox.protect();
    sandbox.write("accounts-json", LOGIN);
    sandbox.ok(
        "mgh",
        &[
            "new",
            "work",
            "--username",
            "CAROL",
            "--email",
            "carol@example.com",
            "--name",
            "Carol Example",
            "--repo",
        ],
    );

    assert!(
        !fs::read_to_string(sandbox.path("gh-calls"))
            .unwrap()
            .contains("auth login")
    );
    assert_eq!(
        sandbox.ok("git", &["config", "--get-all", "mgh.allowedAccount"]),
        "personal\nwork\n"
    );
    assert_eq!(
        sandbox.ok("git", &["config", "--local", "user.name"]),
        "Carol Example\n"
    );
    assert_eq!(
        sandbox.ok("git", &["config", "--local", "user.email"]),
        "carol@example.com\n"
    );
    sandbox.ok("mgh", &["check"]);
}

#[test]
fn new_creates_a_first_account_in_the_default_or_explicit_config_location() {
    for explicit in [false, true] {
        let sandbox = Sandbox::new();

        fs::remove_file(sandbox.path("config/multigh/accounts.conf")).unwrap();
        sandbox.write("accounts-json", LOGIN);

        let mut args = Vec::new();

        if explicit {
            args.extend(["--config", "../custom/accounts.conf"]);
        }

        args.extend_from_slice(ARGS);

        let output = sandbox.ok("mgh", &args);
        let path = sandbox.path(if explicit {
            "custom/accounts.conf"
        } else {
            "config/multigh/accounts.conf"
        });

        assert!(path.is_file());
        assert!(output.contains(if explicit {
            "custom/accounts.conf"
        } else {
            "config/multigh/accounts.conf"
        }));
        assert!(!fs::read_to_string(path).unwrap().contains("[personal]"));
        sandbox.ok(
            "mgh",
            &if explicit {
                vec!["--config", "../custom/accounts.conf", "setup"]
            } else {
                vec!["setup"]
            },
        );

        if explicit {
            assert!(!sandbox.path("config/multigh/accounts.conf").exists());
            assert_eq!(
                fs::metadata(sandbox.path("custom"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o700
            );
        }
    }
}

#[test]
fn new_prompts_for_missing_fields_and_uses_the_default_commit_name() {
    let sandbox = Sandbox::new();

    sandbox.write("login-accounts-json", LOGIN);

    let (status, output) = terminal(
        sandbox.command("mgh").arg("new"),
        &[
            ("Account alias: ", b"Work\r"),
            ("GitHub username: ", b"carol\r"),
            ("Commit name (default: carol): ", b"\r"),
            ("Commit email: ", b"carol@example.com\r"),
        ],
    );

    assert!(status.success(), "{output}");
    assert!(output.contains("Add new identity"), "{output}");
    assert!(!output.contains("New GitHub account"), "{output}");
    assert!(output.contains("Identity details entered"), "{output}");
    assert!(output.contains("carol (default)"), "{output}");
    assert!(
        fs::read_to_string(sandbox.path("config/multigh/accounts.conf"))
            .unwrap()
            .contains("[work]\nusername = carol\nname = carol")
    );
}

#[test]
fn new_commit_name_prompt_accepts_an_override_and_shows_the_username_default() {
    let sandbox = Sandbox::new();

    sandbox.write("accounts-json", LOGIN);

    let (status, output) = terminal(
        sandbox.command("mgh").args([
            "new",
            "work",
            "--username",
            "carol",
            "--email",
            "carol@example.com",
        ]),
        &[("Commit name (default: carol): ", b"Carol Example\r")],
    );

    assert!(status.success(), "{output}");
    assert!(output.contains("Identity details entered"), "{output}");
    assert_eq!(
        sandbox.ok("git", &["config", "--global", "user.name"]),
        "Carol Example\n"
    );
}

#[test]
fn new_cancelled_prompts_and_missing_noninteractive_fields_leave_settings_intact() {
    let sandbox = Sandbox::new();
    let original = fs::read(sandbox.path("config/multigh/accounts.conf")).unwrap();
    let global = fs::read(sandbox.path("gitconfig")).unwrap();
    for keys in [b"\x1b".as_slice(), b"\x03".as_slice()] {
        let (status, output) = terminal(
            sandbox.command("mgh").arg("new"),
            &[("Account alias", keys)],
        );

        assert!(!status.success(), "{output}");
        if keys.ends_with(b"\x1b") {
            assert!(output.contains("Operation cancelled."), "{output}");
        }
    }
    sandbox.blocked("mgh", &["new"], "requires a terminal");
    sandbox.blocked(
        "mgh",
        &["new", "work", "--username", "carol"],
        "requires a terminal",
    );
    assert_eq!(
        original,
        fs::read(sandbox.path("config/multigh/accounts.conf")).unwrap()
    );
    assert_eq!(global, fs::read(sandbox.path("gitconfig")).unwrap());
    assert!(!sandbox.path("gh-calls").exists());
}

#[test]
fn new_rejects_duplicates_invalid_fields_and_injected_sections_before_authentication() {
    let cases = [
        (
            "PERSONAL",
            "carol",
            "carol@example.com",
            "Carol",
            "already exists",
        ),
        (
            "work",
            "ALICE",
            "carol@example.com",
            "Carol",
            "cannot share",
        ),
        (
            "work",
            "carol",
            "ALICE@example.com",
            "Carol",
            "cannot share",
        ),
        (
            "work",
            "carol",
            "123+alice@users.noreply.github.com",
            "Carol",
            "cannot share",
        ),
        (
            "bad alias",
            "carol",
            "carol@example.com",
            "Carol",
            "Invalid account",
        ),
        (
            "work",
            "bad/user",
            "carol@example.com",
            "Carol",
            "Invalid username",
        ),
        ("work", "carol", "bad-email", "Carol", "Invalid email"),
        (
            "work",
            "carol",
            "carol@example.com",
            "<Carol>",
            "Invalid commit name",
        ),
        (
            "work",
            "carol",
            "carol@example.com",
            "Carol\n[injected]",
            "single-line",
        ),
    ];

    for (alias, username, email, name, error) in cases {
        let sandbox = Sandbox::new();
        let original = fs::read(sandbox.path("config/multigh/accounts.conf")).unwrap();
        let global = fs::read(sandbox.path("gitconfig")).unwrap();

        sandbox.blocked(
            "mgh",
            &[
                "new",
                alias,
                "--username",
                username,
                "--email",
                email,
                "--name",
                name,
            ],
            error,
        );

        assert_eq!(
            original,
            fs::read(sandbox.path("config/multigh/accounts.conf")).unwrap()
        );
        assert_eq!(global, fs::read(sandbox.path("gitconfig")).unwrap());
        assert!(!sandbox.path("gh-calls").exists());
    }
}

#[test]
fn new_failed_login_wrong_account_and_expired_login_do_not_save_or_change_git() {
    for failure in ["login", "wrong-account", "expired"] {
        let sandbox = Sandbox::new();
        let original = fs::read(sandbox.path("config/multigh/accounts.conf")).unwrap();
        let global = fs::read(sandbox.path("gitconfig")).unwrap();

        if failure == "login" {
            sandbox.write("fail-login", "");
        } else if failure == "expired" {
            sandbox.write("accounts-json", r#"[{"login":"carol","state":"error"}]"#);
        }

        sandbox.blocked(
            "mgh",
            ARGS,
            if failure == "login" {
                "command failed"
            } else {
                "not signed in"
            },
        );

        assert_eq!(
            original,
            fs::read(sandbox.path("config/multigh/accounts.conf")).unwrap()
        );
        assert_eq!(global, fs::read(sandbox.path("gitconfig")).unwrap());
        assert!(
            !fs::read_to_string(sandbox.path("gh-calls"))
                .unwrap()
                .contains("auth switch")
        );
    }
}

#[test]
fn new_detects_accounts_edited_during_browser_login_and_preserves_the_edits() {
    let sandbox = Sandbox::new();
    let modified = format!("# Edited during login\n{ACCOUNTS}");

    sandbox.write("login-accounts-json", LOGIN);
    sandbox.write("login-accounts-conf", &modified);
    sandbox.blocked("mgh", ARGS, "Accounts changed while signing in");

    assert_eq!(
        modified,
        fs::read_to_string(sandbox.path("config/multigh/accounts.conf")).unwrap()
    );
    assert!(
        !sandbox
            .path("state/multigh/identities/git-work.conf")
            .exists()
    );
}

#[test]
fn new_keeps_the_account_and_reports_recovery_when_setup_or_switch_fails() {
    for failure in ["setup", "switch"] {
        let sandbox = Sandbox::new();

        sandbox.write("accounts-json", LOGIN);

        if failure == "setup" {
            sandbox.ok(
                "git",
                &["config", "--global", "core.hooksPath", "/custom/hooks"],
            );
        } else {
            sandbox.write("fail-switch", "");
        }

        sandbox.blocked("mgh", ARGS, "Account saved in");

        assert!(
            fs::read_to_string(sandbox.path("config/multigh/accounts.conf"))
                .unwrap()
                .contains("[work]")
        );
        assert_eq!(
            sandbox.ok("git", &["config", "--global", "user.email"]),
            "bob@example.edu\n"
        );
    }
}

#[test]
fn new_rejects_invalid_existing_configs_symlinks_and_repo_option_outside_a_repository() {
    let sandbox = Sandbox::new();

    sandbox.write("config/multigh/accounts.conf", "invalid = account\n");
    sandbox.blocked("mgh", ARGS, "Account fields must be inside");

    fs::remove_file(sandbox.path("config/multigh/accounts.conf")).unwrap();
    sandbox.write("target.conf", ACCOUNTS);
    std::os::unix::fs::symlink(
        sandbox.path("target.conf"),
        sandbox.path("config/multigh/accounts.conf"),
    )
    .unwrap();
    sandbox.blocked("mgh", ARGS, "Accounts file is a symlink");

    assert_eq!(
        fs::read_to_string(sandbox.path("target.conf")).unwrap(),
        ACCOUNTS
    );

    let output = sandbox
        .command("mgh")
        .current_dir(sandbox.path("home"))
        .args(ARGS)
        .arg("--repo")
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--repo must be run inside"));
    assert!(!sandbox.path("gh-calls").exists());
}

#[test]
fn new_does_not_create_a_config_after_failed_login_and_keeps_custom_recovery_paths() {
    let sandbox = Sandbox::new();

    sandbox.write("fail-login", "");
    sandbox.blocked(
        "mgh",
        &[
            "--config",
            "../custom/accounts.conf",
            "new",
            "work",
            "--username",
            "carol",
            "--email",
            "carol@example.com",
        ],
        "command failed",
    );

    assert!(!sandbox.path("custom/accounts.conf").exists());
    assert_eq!(fs::read_dir(sandbox.path("custom")).unwrap().count(), 0);

    fs::remove_file(sandbox.path("fail-login")).unwrap();
    sandbox.write("accounts-json", LOGIN);
    sandbox.write("fail-switch", "");

    let output = sandbox.run(
        "mgh",
        &[
            "--config",
            "../custom/accounts.conf",
            "new",
            "work",
            "--username",
            "carol",
            "--email",
            "carol@example.com",
            "--repo",
        ],
    );
    let error = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(
        error.contains("--config")
            && error.contains("custom/accounts.conf")
            && error.contains("switch work --repo"),
        "{error}"
    );
    assert!(sandbox.path("custom/accounts.conf").is_file());
}
