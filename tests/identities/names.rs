use crate::support::{IDENTITIES, Sandbox};
use std::fs;

#[test]
fn mixed_case_identity_names_work_in_commands_and_repository_pins() {
    let sandbox = Sandbox::new();

    sandbox.write(
        "config/multigh/identities.jsonc",
        &IDENTITIES
            .replace("\"personal\"", "\"PeRsOnAl\"")
            .replace("\"school\"", "\"ScHoOl\""),
    );

    sandbox.ok("mgh", &["setup"]);

    assert!(
        sandbox
            .path("state/multigh/identities/git-personal.conf")
            .is_file()
    );

    sandbox.ok("mgh", &["protections", "--allow", "PERSONAL"]);

    assert_eq!(
        sandbox.ok("git", &["config", "mgh.account"]).trim(),
        "personal"
    );

    sandbox.ok("mgh", &["switch", "sChOoL"]);
    sandbox.blocked(
        "git",
        &["commit", "--allow-empty", "-m", "Wrong account"],
        "GitHub is using bob",
    );
    sandbox.ok("mgh", &["switch", "pErSoNaL", "--repo"]);
    sandbox.commit();

    sandbox.ok("git", &["config", "mgh.account", "Personal"]);
    sandbox.ok("git", &["config", "ghguard.account", "PERSONAL"]);
    sandbox.ok("mgh", &["check"]);

    assert!(
        sandbox
            .ok("mgh", &["status"])
            .contains("Identity and commit details match")
    );

    sandbox.ok("mgh", &["welcome", "on"]);

    let welcome = sandbox.ok("mgh", &["welcome"]);

    assert!(!welcome.contains("conflict") && !welcome.contains("needs:"));

    sandbox.write("active", "bob\n");

    assert!(
        sandbox
            .ok("mgh", &["welcome"])
            .to_ascii_lowercase()
            .contains("mgh switch personal")
    );

    sandbox.write("active", "alice\n");

    sandbox.ok("git", &["config", "--unset-all", "mgh.allowedAccount"]);
    sandbox.ok("git", &["config", "ghguard.account", "SCHOOL"]);
    sandbox.blocked("mgh", &["check"], "settings conflict");
    sandbox.ok("git", &["config", "ghguard.account", "PERSONAL"]);
    sandbox.ok("git", &["config", "--unset", "mgh.account"]);
    sandbox.ok("mgh", &["check"]);
    sandbox.ok("mgh", &["protections", "--allow", "PeRsOnAl"]);

    assert!(
        !sandbox
            .run("git", &["config", "ghguard.account"])
            .status
            .success()
    );

    sandbox.commit();
}

#[test]
fn case_colliding_identity_names_are_rejected_before_changing_git() {
    let sandbox = Sandbox::new();

    let before = fs::read(sandbox.path("gitconfig")).unwrap();

    sandbox.write(
        "config/multigh/identities.jsonc",
        &IDENTITIES.replacen(
            "{",
            r#"{"PERSONAL": {"username": "charlie", "commit": {"email": "charlie@example.com"}},"#,
            1,
        ),
    );

    sandbox.blocked("mgh", &["setup"], "Duplicate identity name: personal");

    assert_eq!(before, fs::read(sandbox.path("gitconfig")).unwrap());
    assert!(!sandbox.path("state/multigh/identities").exists());
}
