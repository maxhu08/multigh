use crate::support::Sandbox;
use std::fs;

#[test]
fn switch_updates_authentication_and_global_identity_without_authorizing_the_repo() {
    let sandbox = Sandbox::new();

    let text = sandbox.ok("mgh", &["switch", "SCHOOL"]);

    assert_eq!(
        fs::read_to_string(sandbox.path("active")).unwrap().trim(),
        "bob"
    );
    assert_eq!(
        sandbox
            .ok("git", &["config", "--global", "user.name"])
            .trim(),
        "Bob Example"
    );
    assert_eq!(
        sandbox
            .ok("git", &["config", "--global", "user.email"])
            .trim(),
        "bob@example.edu"
    );
    assert!(
        !sandbox
            .run("git", &["config", "--local", "--get", "mgh.allowedAccount"])
            .status
            .success()
    );
    assert!(text.contains("→ bob"));
    assert!(!text.contains("Account file") && !text.contains("accounts.conf"));

    let outside = sandbox
        .command("mgh")
        .current_dir(sandbox.path(""))
        .args(["switch", "personal"])
        .output()
        .unwrap();

    assert!(outside.status.success());
    assert_eq!(
        sandbox
            .ok("git", &["config", "--global", "user.email"])
            .trim(),
        "alice@example.com"
    );
}

#[test]
fn switching_authorizes_only_when_requested_and_updates_allowed_local_identity() {
    let sandbox = Sandbox::new();

    sandbox.protect();
    sandbox.ok("mgh", &["switch", "school"]);

    assert_eq!(
        sandbox
            .ok("git", &["config", "--local", "user.email"])
            .trim(),
        "alice@example.com"
    );

    sandbox.ok("mgh", &["switch", "school", "--repo"]);

    assert_eq!(
        sandbox
            .ok(
                "git",
                &["config", "--local", "--get-all", "mgh.allowedAccount"]
            )
            .trim(),
        "personal\nschool"
    );
    assert_eq!(
        sandbox
            .ok("git", &["config", "--local", "user.email"])
            .trim(),
        "bob@example.edu"
    );

    sandbox.ok("mgh", &["switch", "personal"]);

    assert_eq!(
        sandbox
            .ok("git", &["config", "--local", "user.email"])
            .trim(),
        "alice@example.com"
    );
    assert!(!sandbox.ok("mgh", &["switch", "personal"]).contains("→"));
}

#[test]
fn failed_or_invalid_switches_do_not_change_authentication_or_commit_defaults() {
    let sandbox = Sandbox::new();

    let original = fs::read(sandbox.path("gitconfig")).unwrap();

    sandbox.blocked("mgh", &["switch", "missing"], "Unknown account");

    assert_eq!(original, fs::read(sandbox.path("gitconfig")).unwrap());

    let outside = sandbox
        .command("mgh")
        .current_dir(sandbox.path(""))
        .args(["switch", "personal", "--repo"])
        .output()
        .unwrap();

    assert!(!outside.status.success());
    assert_eq!(original, fs::read(sandbox.path("gitconfig")).unwrap());

    sandbox.write("fail-switch", "");
    sandbox.blocked("mgh", &["switch", "personal"], "gh:");

    assert_eq!(
        sandbox
            .ok("git", &["config", "--global", "user.email"])
            .trim(),
        "bob@example.edu"
    );
    assert_eq!(
        fs::read_to_string(sandbox.path("active")).unwrap().trim(),
        "alice"
    );
}
