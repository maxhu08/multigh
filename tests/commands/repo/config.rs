use crate::support::Sandbox;
use std::fs;

fn legacy(sandbox: &Sandbox, values: &[&str]) {
    sandbox.ok(
        "git",
        &["config", "--local", "--unset-all", "mgh.allowed-identity"],
    );
    sandbox.ok(
        "git",
        &["config", "--local", "--unset-all", "mgh.current-identity"],
    );

    for value in values {
        sandbox.ok(
            "git",
            &["config", "--local", "--add", "mgh.allowedAccount", value],
        );
    }

    sandbox.ok("git", &["config", "--local", "mgh.account", "personal"]);
}

fn old_keys_removed(sandbox: &Sandbox) {
    for key in ["mgh.allowedAccount", "mgh.account"] {
        assert!(
            !sandbox
                .run("git", &["config", "--local", "--get-all", key])
                .status
                .success()
        );
    }
}

#[test]
fn legacy_permissions_work_read_only_then_migrate_on_setup_or_entry_without_expansion() {
    for command in [&["setup"][..], &["internal", "enter"][..]] {
        let sandbox = Sandbox::new();
        sandbox.protect();
        legacy(&sandbox, &["PeRsOnAl", "SCHOOL"]);
        let config = sandbox.path("repo/.git/config");
        let before = fs::read(&config).unwrap();

        sandbox.ok("mgh", &["repo", "status"]);
        sandbox.ok("mgh", &["repo", "check"]);
        sandbox.ok("mgh", &["settings", "welcome", "on"]);
        sandbox.ok("mgh", &["internal", "welcome"]);
        assert_eq!(fs::read(&config).unwrap(), before);

        sandbox.ok("mgh", command);
        old_keys_removed(&sandbox);
        assert_eq!(
            sandbox.ok("git", &["config", "--get-all", "mgh.allowed-identity"]),
            "PeRsOnAl\nSCHOOL\n"
        );
        assert_eq!(
            sandbox.ok("git", &["config", "mgh.current-identity"]),
            "personal\n"
        );
        assert_eq!(
            sandbox.ok("git", &["remote", "get-url", "origin"]),
            "https://github.com/alice/project.git\n"
        );
        sandbox.ok("mgh", &["repo", "check"]);
        sandbox.write("active", "outsider\n");
        sandbox.blocked(
            "git",
            &["commit", "--allow-empty", "-m", "Wrong identity"],
            "not allowed",
        );
    }
}

#[test]
fn empty_legacy_permissions_stay_blocked_and_new_permissions_override_stale_keys() {
    for canonical in [None, Some("")] {
        let sandbox = Sandbox::new();
        sandbox.protect();
        legacy(&sandbox, &[if canonical.is_some() { "school" } else { "" }]);

        if let Some(value) = canonical {
            sandbox.ok("git", &["config", "--local", "mgh.allowed-identity", value]);
            sandbox.ok(
                "git",
                &["config", "--local", "mgh.current-identity", "school"],
            );
        }

        sandbox.ok("mgh", &["setup"]);
        old_keys_removed(&sandbox);
        assert_eq!(
            sandbox.ok("git", &["config", "--get-all", "mgh.allowed-identity"]),
            "\n"
        );
        assert_eq!(
            sandbox.ok("git", &["config", "mgh.current-identity"]),
            if canonical.is_some() {
                "school\n"
            } else {
                "personal\n"
            }
        );
        sandbox.blocked(
            "git",
            &["commit", "--allow-empty", "-m", "Still blocked"],
            "No identities are authorized",
        );
    }
}

#[test]
fn permission_changes_and_switches_write_only_new_keys_and_keep_local_exceptions() {
    for command in [
        &["repo", "allowed", "remove", "school"][..],
        &["switch", "personal"][..],
    ] {
        let sandbox = Sandbox::new();
        sandbox.protect();
        legacy(&sandbox, &["personal", "school"]);
        sandbox.ok("git", &["config", "--local", "mgh.protections", "false"]);
        sandbox.ok("mgh", command);

        old_keys_removed(&sandbox);
        assert_eq!(sandbox.ok("git", &["config", "mgh.protections"]), "false\n");
        assert_eq!(
            sandbox.ok("git", &["config", "mgh.current-identity"]),
            "personal\n"
        );
        assert_eq!(
            sandbox.ok("git", &["config", "--get-all", "mgh.allowed-identity"]),
            if command[0] == "switch" {
                "personal\nschool\n"
            } else {
                "personal\n"
            }
        );
    }
}

#[test]
fn migrated_empty_permissions_and_stale_pins_do_not_restore_removed_access_in_welcome() {
    let sandbox = Sandbox::new();
    sandbox.protect();
    sandbox.ok("mgh", &["settings", "welcome", "on"]);
    sandbox.ok(
        "git",
        &["config", "--local", "mgh.allowedAccount", "personal"],
    );
    sandbox.ok("git", &["config", "--local", "mgh.account", "personal"]);
    sandbox.ok("mgh", &["repo", "allowed", "remove", "personal"]);

    old_keys_removed(&sandbox);
    sandbox.ok(
        "git",
        &["config", "--local", "user.email", "bob@example.edu"],
    );
    let welcome = sandbox.ok("mgh", &["internal", "welcome"]);

    assert!(
        !welcome.contains("Commit details need") && !welcome.contains("This repository needs"),
        "{welcome}"
    );
    sandbox.blocked("mgh", &["repo", "check"], "No identities are authorized");
}
