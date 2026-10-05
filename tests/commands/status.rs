use crate::support::Sandbox;
use std::fs;

#[test]
fn status_uses_identity_names_even_when_github_logins_differ_or_are_not_signed_in() {
    let sandbox = Sandbox::new();

    sandbox.write(
        "config/multigh/identities.jsonc",
        r#"{"PeRsOnAl": {"username": "alice", "commit": {"email": "alice@example.com"}}, "WoRk": {"username": "carol", "commit": {"email": "carol@example.com"}}}"#,
    );
    sandbox.write(
        "accounts-json",
        r#"[{"login":"ALICE","active":true,"state":"success"},{"login":"unmapped","active":false,"state":"success"}]"#,
    );

    let output = sandbox.ok("mgh", &["status"]);

    assert!(
        output.contains("personal (Active)\n    GitHub username           alice\n    Commit name               alice\n    Commit email              alice@example.com"),
        "{output}"
    );
    assert!(
        output.contains("work (Not signed in)\n    GitHub username           carol\n    Commit name               carol\n    Commit email              carol@example.com"),
        "{output}"
    );
    assert!(output.contains("git-personal.conf") && output.contains("git-work.conf"));
    assert!(
        output.contains("Unconfigured GitHub accounts\n  unmapped"),
        "{output}"
    );
    assert!(!output.contains("ALICE alice@example.com"));
}

#[test]
fn status_reports_live_authentication_identity_and_protection_without_mutation() {
    let sandbox = Sandbox::new();

    sandbox.protect();
    sandbox.write("selected", "bob\n");

    let before = fs::read(sandbox.path("gitconfig")).unwrap();
    let local = fs::read(sandbox.path("repo/.git/config")).unwrap();
    let status = sandbox.ok("mgh", &["status"]);

    assert!(status.contains(&format!(
        "  personal (Active)\n    GitHub username           alice\n    Commit name               Alice Example\n    Commit email              alice@example.com\n    Identity file             {}",
        sandbox.path("state/multigh/identities/git-personal.conf").display()
    )));
    assert!(status.contains(&format!(
        "  school\n    GitHub username           bob\n    Commit name               Bob Example\n    Commit email              bob@example.edu\n    Identity file             {}",
        sandbox.path("state/multigh/identities/git-school.conf").display()
    )));
    assert!(status.contains("Identities"));
    assert_eq!(status.matches("(Active)").count(), 1);
    assert!(status.contains(&format!(
        "  Accounts\n  {}",
        sandbox.path("config/multigh/identities.jsonc").display()
    )));
    assert!(!status.contains("Account file") && !status.contains("Global commit defaults"));
    assert!(status.contains("Allowed identities          personal"));
    assert!(
        status.contains("alice@example.com")
            && status.contains("Identity and commit details match")
    );
    assert!(status.contains("Commit + push checks enabled"));
    assert!(status.contains("Active identity             personal"));
    assert!(status.contains("Autoswitch (global)         OFF"));
    assert_eq!(before, fs::read(sandbox.path("gitconfig")).unwrap());
    assert_eq!(local, fs::read(sandbox.path("repo/.git/config")).unwrap());

    let full = sandbox.ok("mgh", &["status", "--full"]);

    assert!(full.contains("GitHub authentication details"));
}

#[test]
fn repository_status_reports_local_preferences_and_works_in_bare_repositories() {
    let sandbox = Sandbox::new();
    sandbox.protect();
    sandbox.ok("mgh", &["settings", "autoswitch", "on"]);
    assert!(
        sandbox
            .ok("mgh", &["repo", "status"])
            .contains("Autoswitch (global)         ON")
    );
    sandbox.ok("git", &["init", "--bare", "../bare.git"]);
    let output = sandbox
        .command("mgh")
        .current_dir(sandbox.path("bare.git"))
        .args(["repo", "status"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("Current repository          bare.git"));
    assert!(text.contains("No identities are authorized"));
    assert!(text.contains("Autoswitch (global)         ON"));
    assert!(text.contains(&format!(
        "Repo config                 {}",
        sandbox.path("bare.git/config").canonicalize().unwrap().display()
    )));
}

#[test]
fn linked_worktree_status_shows_the_common_repository_config_path_without_changes() {
    let sandbox = Sandbox::new();
    sandbox.protect();
    sandbox.commit();
    sandbox.ok("git", &["worktree", "add", "-b", "linked", "../linked"]);
    let config = sandbox.path("repo/.git/config");
    let original = fs::read(&config).unwrap();
    let output = sandbox
        .command("mgh")
        .current_dir(sandbox.path("linked"))
        .args(["repo", "status"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.contains(&format!(
            "Repo config                 {}",
            config.canonicalize().unwrap().display()
        )),
        "{text}"
    );
    assert_eq!(fs::read(config).unwrap(), original);
}

#[test]
fn status_explains_unconfigured_disallowed_and_mismatched_repositories() {
    let sandbox = Sandbox::new();

    assert!(
        sandbox
            .ok("mgh", &["status"])
            .contains("No identities are authorized")
    );

    sandbox.protect();
    sandbox.write("active", "bob\n");

    assert!(
        sandbox
            .ok("mgh", &["status"])
            .contains("not allowed in this repository")
    );

    sandbox.write("active", "alice\n");
    sandbox.ok(
        "git",
        &["config", "--local", "user.email", "bob@example.edu"],
    );

    assert!(
        sandbox
            .ok("mgh", &["status"])
            .contains("commit details do not match identity")
    );

    sandbox.ok("mgh", &["repo", "protections", "off"]);

    assert!(
        sandbox
            .ok("mgh", &["status"])
            .contains("Protection                  OFF")
    );
}

#[test]
fn status_handles_authentication_failures_and_invalid_identity_config() {
    let sandbox = Sandbox::new();

    sandbox.write("accounts-json", "[]\n");
    sandbox.blocked("mgh", &["status"], "No active GitHub account");
    sandbox.write("accounts-json", "invalid json\n");

    assert!(!sandbox.run("mgh", &["status"]).status.success());

    sandbox.write(
        "accounts-json",
        "[{\"login\":\"alice\",\"active\":true,\"state\":\"expired\"}]\n",
    );

    assert!(sandbox.ok("mgh", &["status"]).contains("expired"));

    sandbox.write("fail-full", "");
    sandbox.blocked(
        "mgh",
        &["status", "--full"],
        "GitHub authentication check failed",
    );
    fs::remove_file(sandbox.path("config/multigh/identities.jsonc")).unwrap();

    assert!(sandbox.ok("mgh", &["status"]).contains("Read "));

    sandbox.write("fail-auth", "");
    sandbox.blocked("mgh", &["status"], "gh:");
}

#[test]
fn status_outside_a_repository_reports_identity_entries_without_global_defaults() {
    let sandbox = Sandbox::new();

    let output = sandbox
        .command("mgh")
        .current_dir(sandbox.path(""))
        .arg("status")
        .output()
        .unwrap();

    assert!(output.status.success());

    let text = String::from_utf8_lossy(&output.stdout);

    assert!(text.contains("Identities"));
    assert!(text.contains("personal (Active)\n    GitHub username           alice\n    Commit name               Alice Example\n    Commit email              alice@example.com"));
    assert!(text.contains("school\n    GitHub username           bob\n    Commit name               Bob Example\n    Commit email              bob@example.edu"));
    assert!(!text.contains("GitHub accounts") && !text.contains("Account file"));
    assert!(!text.contains("Global commit defaults"));
    assert!(!text.contains("Current repository"));
}
