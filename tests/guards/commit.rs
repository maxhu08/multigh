use crate::support::Sandbox;
use std::fs;

#[test]
fn guards_actual_author_committer_and_authentication() {
    let sandbox = Sandbox::new();

    sandbox.executable(
        "repo/.git/hooks/pre-commit",
        "#!/bin/sh\nprintf 'original\\n' >> \"$TEST_MGH_ROOT/commits\"\n",
    );
    sandbox.protect();

    sandbox.write("active", "bob\n");
    sandbox.blocked(
        "git",
        &["commit", "--allow-empty", "-m", "Wrong login"],
        "GitHub is using bob",
    );

    sandbox.ok("mgh", &["switch", "personal", "--repo"]);
    sandbox.blocked(
        "git",
        &[
            "-c",
            "user.email=bob@example.edu",
            "commit",
            "--allow-empty",
            "-m",
            "Wrong identity",
        ],
        "commit details do not match identity",
    );
    sandbox.blocked(
        "git",
        &[
            "commit",
            "--allow-empty",
            "--author=Bob Example <bob@example.edu>",
            "-m",
            "Wrong author",
        ],
        "AUTHOR commit details",
    );

    let output = sandbox
        .command("git")
        .env("GIT_COMMITTER_EMAIL", "bob@example.edu")
        .args(["commit", "--allow-empty", "-m", "Wrong committer"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("COMMITTER commit details"));

    sandbox.commit();

    assert_eq!(
        fs::read_to_string(sandbox.path("commits")).unwrap(),
        "original\n"
    );

    sandbox.ok(
        "git",
        &[
            "-c",
            "user.email=123+alice@users.noreply.github.com",
            "commit",
            "--allow-empty",
            "-m",
            "Allowed address",
        ],
    );
    sandbox.ok("mgh", &["check"]);
}

#[test]
fn name_overrides_are_rejected_and_email_case_does_not_reject_allowed_addresses() {
    let sandbox = Sandbox::new();

    sandbox.protect();

    for kind in ["AUTHOR", "COMMITTER"] {
        let output = sandbox
            .command("git")
            .env(format!("GIT_{kind}_NAME"), "Bob Example")
            .args(["commit", "--allow-empty", "-m", "Wrong name"])
            .output()
            .unwrap();

        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(&format!("{kind} commit details"))
        );
    }

    sandbox.ok(
        "git",
        &[
            "-c",
            "user.email=123+ALICE@users.noreply.github.com",
            "commit",
            "--allow-empty",
            "-m",
            "Allowed email case",
        ],
    );
}

#[test]
fn invalid_repo_pins_and_missing_live_authentication_fail_closed() {
    let sandbox = Sandbox::new();

    sandbox.protect();
    sandbox.ok(
        "git",
        &["config", "--replace-all", "mgh.allowedAccount", "missing"],
    );
    sandbox.blocked(
        "git",
        &["commit", "--allow-empty", "-m", "Bad policy"],
        "Unknown identity",
    );
    sandbox.ok("mgh", &["protections", "--allow", "personal"]);
    sandbox.write("fail-auth", "");
    sandbox.blocked(
        "git",
        &["commit", "--allow-empty", "-m", "Cannot verify auth"],
        "gh:",
    );

    assert!(
        !sandbox
            .run("git", &["rev-parse", "--verify", "HEAD"])
            .status
            .success()
    );
}
