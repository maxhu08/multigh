use crate::support::Sandbox;
use std::fs;

#[test]
fn welcome_distinguishes_a_login_named_unavailable_from_no_selection() {
    let sandbox = Sandbox::new();
    sandbox.write(
        "config/multigh/identities.jsonc",
        r#"{"personal":{"username":"unavailable","commit":{"email":"personal@example.com"}}}"#,
    );
    sandbox.ok("mgh", &["settings", "welcome", "on"]);
    sandbox.ok("mgh", &["repo", "allowed", "add", "personal"]);
    sandbox.write("selected", "unavailable\n");

    let selected = sandbox.ok("mgh", &["internal", "welcome"]);
    assert!(
        selected
            .lines()
            .any(|line| line.trim_start().starts_with("Identity")
                && line.trim_end().ends_with("personal")),
        "{selected}"
    );

    sandbox.write("fail-selected", "");
    let missing = sandbox.ok("mgh", &["internal", "welcome"]);
    assert!(
        missing
            .lines()
            .any(|line| line.trim_start().starts_with("Identity")
                && line.trim_end().ends_with("not configured")),
        "{missing}"
    );
    assert!(
        missing.contains("This repository needs: mgh switch personal"),
        "{missing}"
    );
}

#[test]
fn welcome_reports_conflicting_legacy_policy_without_rewriting_it() {
    let sandbox = Sandbox::new();
    sandbox.ok("mgh", &["settings", "welcome", "on"]);
    sandbox.ok(
        "git",
        &["config", "--local", "mgh.current-identity", "personal"],
    );
    sandbox.ok("git", &["config", "--local", "ghguard.account", "school"]);
    let original = fs::read(sandbox.path("repo/.git/config")).unwrap();

    let welcome = sandbox.ok("mgh", &["internal", "welcome"]);
    assert!(
        welcome.contains("Repository identity settings conflict"),
        "{welcome}"
    );
    sandbox.blocked(
        "mgh",
        &["repo", "check"],
        "Repository identity settings conflict",
    );
    assert_eq!(
        fs::read(sandbox.path("repo/.git/config")).unwrap(),
        original
    );
}

#[test]
fn unexpected_selected_account_failure_is_reported() {
    let sandbox = Sandbox::new();
    sandbox.ok("mgh", &["settings", "welcome", "on"]);
    sandbox.executable(
        "bin/gh",
        "#!/bin/sh\nprintf 'unable to read account' >&2\nexit 2\n",
    );

    sandbox.blocked(
        "mgh",
        &["internal", "welcome"],
        "gh: unable to read account",
    );
}

#[test]
fn welcome_preferences_work_without_config_and_do_not_modify_git_or_auth() {
    let sandbox = Sandbox::new();

    let before = fs::read(sandbox.path("gitconfig")).unwrap();

    fs::remove_file(sandbox.path("config/multigh/identities.jsonc")).unwrap();

    assert!(sandbox.ok("mgh", &["internal", "welcome"]).is_empty());

    sandbox.ok("mgh", &["settings", "welcome", "on"]);

    assert!(sandbox.path("state/multigh/welcome-enabled").is_file());
    assert!(!sandbox.path("gh-calls").exists());

    sandbox.ok("mgh", &["settings", "welcome", "off"]);
    sandbox.ok("mgh", &["settings", "welcome", "off"]);

    assert!(!sandbox.path("state/multigh/welcome-enabled").exists());
    assert!(sandbox.ok("mgh", &["internal", "welcome"]).is_empty());
    assert_eq!(before, fs::read(sandbox.path("gitconfig")).unwrap());
}

#[test]
fn welcome_uses_local_selection_and_reports_repo_or_identity_mismatches() {
    let sandbox = Sandbox::new();

    sandbox.protect();
    sandbox.ok("mgh", &["settings", "welcome", "on"]);
    sandbox.write("selected", "bob\n");
    fs::remove_file(sandbox.path("gh-calls")).unwrap();

    let text = sandbox.ok("mgh", &["internal", "welcome"]);

    assert!(text.lines().any(
        |line| line.trim_start().starts_with("Identity") && line.trim_end().ends_with("school")
    ));
    assert!(
        text.lines()
            .any(|line| line.trim_start().starts_with("GitHub username")
                && line.trim_end().ends_with("bob"))
    );
    assert!(text.contains("mgh switch personal"));

    let calls = fs::read_to_string(sandbox.path("gh-calls")).unwrap();

    assert!(calls.contains("config get") && !calls.contains("auth status"));

    sandbox.write("selected", "alice\n");
    sandbox.ok("git", &["config", "--local", "user.name", "Wrong Name"]);

    assert!(
        sandbox
            .ok("mgh", &["internal", "welcome"])
            .contains("Commit details need: mgh switch personal")
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
            .ok("mgh", &["internal", "welcome"])
            .contains("Commit details need:")
    );
}

#[test]
fn welcome_handles_unavailable_selection_and_missing_config_without_breaking_startup() {
    let sandbox = Sandbox::new();

    sandbox.protect();
    sandbox.ok("mgh", &["settings", "welcome", "on"]);
    sandbox.write("fail-selected", "");

    assert!(
        sandbox
            .ok("mgh", &["internal", "welcome"])
            .contains("unavailable")
    );

    fs::remove_file(sandbox.path("config/multigh/identities.jsonc")).unwrap();

    let welcome = sandbox.ok("mgh", &["internal", "welcome"]);

    assert!(welcome.contains("unavailable") && welcome.contains("Read "));
}
