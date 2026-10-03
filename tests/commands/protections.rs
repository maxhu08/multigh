use crate::support::{IDENTITIES, Sandbox};

#[test]
fn multiple_allowed_identities_work_and_switching_updates_the_local_identity() {
    let sandbox = Sandbox::new();

    let mut identities: serde_json::Value = serde_json::from_str(IDENTITIES).unwrap();

    identities["work"] = serde_json::json!({"username": "carol", "commit": {"name": "Carol Example", "email": "carol@example.com"}});
    sandbox.write("config/multigh/identities.jsonc", &identities.to_string());
    sandbox.ok("mgh", &["setup"]);
    sandbox.ok(
        "mgh",
        &["protections", "--allow", "PERSONAL,school,Personal"],
    );

    assert_eq!(
        sandbox
            .ok("git", &["config", "--get-all", "mgh.allowedAccount"])
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

    let initial = sandbox.ok("mgh", &["protections"]);

    assert!(initial.contains("OFF") && initial.contains("No identities selected"));

    sandbox.protect();
    sandbox.ok(
        "mgh",
        &["protections", "--allow", "PERSONAL,School,personal"],
    );

    assert_eq!(
        sandbox
            .ok(
                "git",
                &["config", "--local", "--get-all", "mgh.allowedAccount"]
            )
            .trim(),
        "personal\nschool"
    );

    sandbox.ok("mgh", &["protections", "--allow", "SCHOOL"]);

    assert_eq!(
        sandbox
            .ok(
                "git",
                &["config", "--local", "--get-all", "mgh.allowedAccount"]
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

    let mode = sandbox.ok("mgh", &["protections"]);

    assert!(mode.contains("ON") && mode.contains("school") && mode.contains("bob"));

    sandbox.ok("mgh", &["protections", "off"]);
    sandbox.ok("mgh", &["protections", "on"]);

    assert!(sandbox.path("state/multigh/protections-enabled").is_file());
}

#[test]
fn invalid_authorization_does_not_change_the_existing_repo_policy() {
    let sandbox = Sandbox::new();

    sandbox.protect();

    let before = std::fs::read(sandbox.path("repo/.git/config")).unwrap();

    sandbox.blocked(
        "mgh",
        &["protections", "--allow", "personal,unknown"],
        "Unknown identity",
    );

    assert_eq!(
        before,
        std::fs::read(sandbox.path("repo/.git/config")).unwrap()
    );

    let outside = sandbox
        .command("mgh")
        .current_dir(sandbox.path(""))
        .args(["protections", "--allow", "personal"])
        .output()
        .unwrap();

    assert!(!outside.status.success());
    assert_eq!(
        before,
        std::fs::read(sandbox.path("repo/.git/config")).unwrap()
    );

    sandbox.blocked("mgh", &["protections", "--repo"], "interactive terminal");

    assert_eq!(
        before,
        std::fs::read(sandbox.path("repo/.git/config")).unwrap()
    );
}

#[test]
fn off_modes_work_with_missing_config_and_unconfigured_entry_still_blocks_commits() {
    let sandbox = Sandbox::new();

    sandbox.ok("mgh", &["setup"]);

    let entry = sandbox.ok("mgh", &["enter"]);

    assert!(entry.contains("No identities selected; commits and pushes remain blocked."));

    sandbox.blocked(
        "git",
        &["commit", "--allow-empty", "-m", "Blocked"],
        "No identities are authorized",
    );
    std::fs::remove_file(sandbox.path("config/multigh/identities.jsonc")).unwrap();
    sandbox.ok("mgh", &["protections", "off"]);
    sandbox.ok("mgh", &["verbose", "off"]);

    assert!(sandbox.ok("mgh", &["enter"]).is_empty());

    sandbox.commit();
    sandbox.blocked("mgh", &["protections", "on"], "Read ");

    assert!(!sandbox.path("state/multigh/protections-enabled").exists());
}
