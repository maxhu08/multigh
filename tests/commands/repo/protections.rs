use crate::support::{IDENTITIES, Sandbox};

#[test]
fn multiple_allowed_identities_work_and_switching_updates_the_local_identity() {
    let sandbox = Sandbox::new();

    let mut identities: serde_json::Value = serde_json::from_str(IDENTITIES).unwrap();

    identities["work"] = serde_json::json!({"username": "carol", "commit": {"name": "Carol Example", "email": "carol@example.com"}});
    sandbox.write("config/multigh/identities.jsonc", &identities.to_string());
    sandbox.ok("mgh", &["setup"]);
    sandbox.allow("PERSONAL,school,Personal");

    assert_eq!(
        sandbox
            .ok("git", &["config", "--get-all", "mgh.allowed-identity"])
            .trim(),
        "personal\nschool"
    );

    sandbox.commit();
    sandbox.ok("mgh", &["switch", "SCHOOL"]);

    assert_eq!(
        sandbox.ok("git", &["config", "user.email"]).trim(),
        "bob@example.edu"
    );

    sandbox.commit();

    sandbox.ok("git", &["init", "--bare", "../remote.git"]);
    sandbox.ok("git", &["remote", "set-url", "origin", "../remote.git"]);
    sandbox.ok("git", &["push", "origin", "main"]);
    sandbox.ok(
        "git",
        &[
            "commit",
            "--no-verify",
            "--allow-empty",
            "--author=Carol Example <carol@example.com>",
            "-m",
            "Other account",
        ],
    );
    sandbox.blocked(
        "git",
        &["push", "origin", "main"],
        "contains your work identity",
    );

    sandbox.write("active", "carol\n");
    sandbox.blocked(
        "git",
        &["commit", "--allow-empty", "-m", "Disallowed login"],
        "not allowed in this repository",
    );
}

#[test]
fn protections_reports_status_and_replaces_allowed_identities_without_duplicates() {
    let sandbox = Sandbox::new();

    let initial = sandbox.ok("mgh", &["repo", "protections"]);

    assert!(initial.contains("ON"));

    sandbox.protect();
    sandbox.allow("PERSONAL,School,personal");

    assert_eq!(
        sandbox
            .ok(
                "git",
                &["config", "--local", "--get-all", "mgh.allowed-identity"]
            )
            .trim(),
        "personal\nschool"
    );

    sandbox.allow("SCHOOL");

    assert_eq!(
        sandbox
            .ok(
                "git",
                &["config", "--local", "--get-all", "mgh.allowed-identity"]
            )
            .trim(),
        "school"
    );
    assert_eq!(
        sandbox
            .ok("git", &["config", "--local", "user.email"])
            .trim(),
        "bob@example.edu"
    );

    let mode = sandbox.ok("mgh", &["repo", "protections"]);

    assert!(mode.contains("ON"));
    assert!(sandbox.ok("mgh", &["repo", "allowed"]).contains("school"));

    sandbox.ok("mgh", &["repo", "protections", "off"]);
    sandbox.ok("mgh", &["repo", "protections", "on"]);

    assert_eq!(
        sandbox
            .ok("git", &["config", "--local", "mgh.protections"])
            .trim(),
        "true"
    );
}

#[test]
fn invalid_authorization_does_not_change_the_existing_repo_policy() {
    let sandbox = Sandbox::new();

    sandbox.protect();

    let before = std::fs::read(sandbox.path("repo/.git/config")).unwrap();

    sandbox.blocked(
        "mgh",
        &["repo", "allowed", "add", "unknown"],
        "Unknown identity",
    );

    assert_eq!(
        before,
        std::fs::read(sandbox.path("repo/.git/config")).unwrap()
    );

    let outside = sandbox
        .command("mgh")
        .current_dir(sandbox.path(""))
        .args(["repo", "allowed", "add", "personal"])
        .output()
        .unwrap();

    assert!(!outside.status.success());
    assert_eq!(
        before,
        std::fs::read(sandbox.path("repo/.git/config")).unwrap()
    );

    sandbox.blocked(
        "mgh",
        &["repo", "allowed", "update"],
        "interactive terminal",
    );

    assert_eq!(
        before,
        std::fs::read(sandbox.path("repo/.git/config")).unwrap()
    );
}

#[test]
fn off_modes_work_with_missing_config_and_unconfigured_entry_still_blocks_commits() {
    let sandbox = Sandbox::new();

    sandbox.ok("mgh", &["setup"]);

    let entry = sandbox.ok("mgh", &["internal", "enter"]);

    assert!(entry.contains("No identities selected; commits and pushes remain blocked."));

    sandbox.blocked(
        "git",
        &["commit", "--allow-empty", "-m", "Blocked"],
        "No identities are authorized",
    );
    std::fs::remove_file(sandbox.path("config/multigh/identities.jsonc")).unwrap();
    sandbox.ok("mgh", &["repo", "protections", "off"]);
    sandbox.ok("mgh", &["settings", "verbose", "off"]);

    assert!(sandbox.ok("mgh", &["internal", "enter"]).is_empty());

    sandbox.commit();
    sandbox.blocked("mgh", &["repo", "protections", "on"], "Read ");

    assert!(!sandbox.path("state/multigh/protections-enabled").exists());
}

#[test]
fn repo_protections_default_on_and_off_is_local_and_survives_setup() {
    let sandbox = Sandbox::new();
    assert!(sandbox.ok("mgh", &["repo", "protections"]).contains("ON"));
    sandbox.protect();
    sandbox.ok("mgh", &["repo", "protections", "off"]);
    sandbox.ok("mgh", &["settings", "verbose", "off"]);
    sandbox.ok("mgh", &["setup"]);
    assert!(sandbox.ok("mgh", &["repo", "protections"]).contains("OFF"));
    assert!(sandbox.ok("mgh", &["settings", "verbose"]).contains("OFF"));
    sandbox.write("active", "bob\n");
    sandbox.commit();
    sandbox.ok("git", &["init", "../other"]);
    let other = sandbox
        .command("mgh")
        .current_dir(sandbox.path("other"))
        .args(["repo", "protections"])
        .output()
        .unwrap();
    assert!(other.status.success());
    assert!(String::from_utf8_lossy(&other.stdout).contains("ON"));
    sandbox.ok("mgh", &["repo", "protections", "on"]);
    sandbox.blocked(
        "git",
        &["commit", "--allow-empty", "-m", "Blocked"],
        "GitHub is using bob",
    );
}
