use crate::support::{Sandbox, terminal};
use std::fs;

#[test]
fn identity_edit_and_remove_preserve_comments_permissions_and_other_identities() {
    let sandbox = Sandbox::new();
    sandbox.protect();
    let original = fs::read_to_string(sandbox.path("config/multigh/identities.jsonc")).unwrap();
    sandbox.write(
        "config/multigh/identities.jsonc",
        &format!("// My identities\n{original}"),
    );
    let local = fs::read(sandbox.path("repo/.git/config")).unwrap();
    sandbox.ok(
        "mgh",
        &["identity", "edit", "PERSONAL", "--email", "new@example.com"],
    );
    let updated = fs::read_to_string(sandbox.path("config/multigh/identities.jsonc")).unwrap();
    assert!(updated.starts_with("// My identities"));
    assert!(updated.contains("new@example.com") && updated.contains("bob@example.edu"));
    assert_eq!(fs::read(sandbox.path("repo/.git/config")).unwrap(), local);
    sandbox.blocked(
        "mgh",
        &["identity", "edit", "personal", "--email", "bad email"],
        "Invalid email",
    );
    assert_eq!(
        fs::read_to_string(sandbox.path("config/multigh/identities.jsonc")).unwrap(),
        updated
    );
    sandbox.ok("mgh", &["identity", "remove", "school"]);
    assert!(
        !fs::read_to_string(sandbox.path("config/multigh/identities.jsonc"))
            .unwrap()
            .contains("bob@example.edu")
    );
    assert!(
        !sandbox
            .ok("git", &["config", "--global", "--list"])
            .contains("bob/**")
    );
    assert_eq!(
        sandbox.ok("mgh", &["identity"]),
        sandbox.ok("mgh", &["identity", "list"])
    );
    sandbox.ok("mgh", &["identity", "remove", "personal"]);
    sandbox.blocked(
        "git",
        &["commit", "--allow-empty", "-m", "Blocked"],
        "Unknown identity",
    );
    sandbox.ok("mgh", &["repo", "allowed", "remove", "personal"]);
    assert!(
        sandbox
            .ok("mgh", &["repo", "allowed"])
            .contains("No identities selected")
    );
}

#[test]
fn interactive_edit_uses_existing_defaults_and_cancellation_preserves_state() {
    let sandbox = Sandbox::new();
    let original = fs::read(sandbox.path("config/multigh/identities.jsonc")).unwrap();
    let global = fs::read(sandbox.path("gitconfig")).unwrap();

    for keys in [b"\x1b".as_slice(), b"\x03".as_slice()] {
        let (status, _) = terminal(
            sandbox
                .command("mgh")
                .args(["identity", "edit", "personal"]),
            &[("GitHub username", keys)],
        );

        assert!(!status.success());
        assert_eq!(
            fs::read(sandbox.path("config/multigh/identities.jsonc")).unwrap(),
            original
        );
        assert_eq!(fs::read(sandbox.path("gitconfig")).unwrap(), global);
    }

    let (status, output) = terminal(
        sandbox
            .command("mgh")
            .args(["identity", "edit", "PERSONAL"]),
        &[
            ("GitHub username", b"\r"),
            ("Commit email", b"\r"),
            ("Commit name", b"Updated Name\r"),
        ],
    );

    assert!(status.success(), "{output}");
    let saved: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(sandbox.path("config/multigh/identities.jsonc")).unwrap(),
    )
    .unwrap();

    assert_eq!(saved["personal"]["username"], "alice");
    assert_eq!(saved["personal"]["commit"]["email"], "alice@example.com");
    assert_eq!(saved["personal"]["commit"]["name"], "Updated Name");
    assert_eq!(
        saved["personal"]["commit"]["additional_emails"][0],
        "123+alice@users.noreply.github.com"
    );
}

#[test]
fn edit_authentication_failures_and_concurrent_changes_do_not_replace_configuration() {
    for failure in ["login", "wrong-account", "concurrent"] {
        let sandbox = Sandbox::new();
        let original = fs::read_to_string(sandbox.path("config/multigh/identities.jsonc")).unwrap();
        let global = fs::read(sandbox.path("gitconfig")).unwrap();
        let changed = format!("// Edited during login\n{original}");

        if failure == "login" {
            sandbox.write("fail-login", "");
        } else if failure == "concurrent" {
            sandbox.write(
                "login-accounts-json",
                r#"[{"login":"carol","active":true,"state":"success"}]"#,
            );
            sandbox.write("login-identities-json", &changed);
        }

        sandbox.blocked(
            "mgh",
            &["identity", "edit", "personal", "--username", "carol"],
            match failure {
                "login" => "command failed",
                "wrong-account" => "not signed in",
                _ => "Configuration changed during editing",
            },
        );

        assert_eq!(fs::read(sandbox.path("gitconfig")).unwrap(), global);
        assert_eq!(
            fs::read_to_string(sandbox.path("config/multigh/identities.jsonc")).unwrap(),
            if failure == "concurrent" {
                changed
            } else {
                original
            }
        );
    }
}

#[test]
fn edit_and_remove_reject_unknown_identities_and_symlinks_without_mutations() {
    let sandbox = Sandbox::new();
    let original = fs::read(sandbox.path("config/multigh/identities.jsonc")).unwrap();
    let global = fs::read(sandbox.path("gitconfig")).unwrap();

    sandbox.blocked(
        "mgh",
        &["identity", "edit", "missing", "--name", "Name"],
        "Unknown identity",
    );
    sandbox.blocked(
        "mgh",
        &["identity", "remove", "missing"],
        "Unknown identity",
    );
    fs::rename(
        sandbox.path("config/multigh/identities.jsonc"),
        sandbox.path("target.jsonc"),
    )
    .unwrap();
    std::os::unix::fs::symlink(
        sandbox.path("target.jsonc"),
        sandbox.path("config/multigh/identities.jsonc"),
    )
    .unwrap();

    for command in ["edit", "remove"] {
        sandbox.blocked(
            "mgh",
            &["identity", command, "personal"],
            "Configuration file is a symlink",
        );
    }

    assert_eq!(fs::read(sandbox.path("target.jsonc")).unwrap(), original);
    assert_eq!(fs::read(sandbox.path("gitconfig")).unwrap(), global);
    assert!(!sandbox.path("gh-calls").exists());
}
