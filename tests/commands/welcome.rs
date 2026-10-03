use crate::support::Sandbox;
use std::fs;

#[test]
fn welcome_preferences_work_without_config_and_do_not_modify_git_or_auth() {
    let sandbox = Sandbox::new();

    let before = fs::read(sandbox.path("gitconfig")).unwrap();

    fs::remove_file(sandbox.path("config/multigh/accounts.conf")).unwrap();

    assert!(sandbox.ok("mgh", &["welcome"]).is_empty());

    assert_eq!(
        sandbox.ok("mgh", &["welcome", "on"]),
        "\n  ✓ Identity welcome enabled\n\n"
    );

    assert!(sandbox.path("state/multigh/welcome-enabled").is_file());
    assert!(!sandbox.path("gh-calls").exists());

    assert_eq!(
        sandbox.ok("mgh", &["welcome", "off"]),
        "\n  ✓ Identity welcome disabled\n\n"
    );
    sandbox.ok("mgh", &["welcome", "off"]);

    assert!(sandbox.ok("mgh", &["welcome"]).is_empty());
    assert_eq!(before, fs::read(sandbox.path("gitconfig")).unwrap());
}

#[test]
fn welcome_uses_local_selection_and_reports_repo_or_identity_mismatches() {
    let sandbox = Sandbox::new();

    sandbox.protect();
    sandbox.ok("mgh", &["welcome", "on"]);
    sandbox.write("selected", "bob\n");
    fs::remove_file(sandbox.path("gh-calls")).unwrap();

    let text = sandbox.ok("mgh", &["welcome"]);

    assert!(text.starts_with("\n  Identity\n"));
    assert!(!text.contains("GitHub"));
    assert!(text.contains("Selected account bob"));
    assert!(text.contains("mgh switch personal"));
    assert!(!text.contains("~~~"));

    let calls = fs::read_to_string(sandbox.path("gh-calls")).unwrap();

    assert!(calls.contains("config get") && !calls.contains("auth status"));

    sandbox.write("selected", "alice\n");
    sandbox.ok("git", &["config", "--local", "user.name", "Wrong Name"]);

    assert!(
        sandbox
            .ok("mgh", &["welcome"])
            .contains("Commit identity needs: mgh switch personal")
    );

    sandbox.ok("git", &["config", "--local", "user.name", "Alice Example"]);
    sandbox.ok(
        "git",
        &[
            "config",
            "--local",
            "user.email",
            "123+ALICE@users.noreply.github.com",
        ],
    );

    assert!(
        !sandbox
            .ok("mgh", &["welcome"])
            .contains("Commit identity needs:")
    );
}

#[test]
fn welcome_handles_unavailable_selection_and_missing_config_without_breaking_startup() {
    let sandbox = Sandbox::new();

    sandbox.protect();
    sandbox.ok("mgh", &["welcome", "on"]);
    sandbox.write("fail-selected", "");

    assert!(sandbox.ok("mgh", &["welcome"]).contains("unavailable"));

    fs::remove_file(sandbox.path("config/multigh/accounts.conf")).unwrap();

    let welcome = sandbox.ok("mgh", &["welcome"]);

    assert!(welcome.contains("Selected account") && welcome.contains("Read "));
}
