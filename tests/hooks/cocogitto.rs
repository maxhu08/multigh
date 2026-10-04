use crate::support::Sandbox;
use std::fs;

fn install(sandbox: &Sandbox) {
    sandbox.write("repo/cog.toml", include_str!("../../cog.toml"));
    sandbox.write(
        "home/.gitconfig",
        &fs::read_to_string(sandbox.path("gitconfig")).unwrap(),
    );

    let output = sandbox
        .command("cog")
        .args(["install-hook", "--all"])
        .output()
        .expect("Install Cocogitto before running the hook tests");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(sandbox.path("repo/.git/hooks/commit-msg").is_file());
}

#[test]
fn installed_cocogitto_hook_accepts_conventional_and_merge_messages() {
    let sandbox = Sandbox::new();

    install(&sandbox);

    for message in [
        "feat: add identity switching",
        "fix(config): preserve comments",
        "feat!: change identity configuration",
        "fix(config)!: change identity configuration",
        "feat: change configuration\n\nBREAKING CHANGE: new schema",
        "Merge branch 'topic'",
    ] {
        sandbox.write("repo/message with spaces", message);
        sandbox.ok(
            "git",
            &["hook", "run", "commit-msg", "--", "message with spaces"],
        );
    }

    for kind in [
        "build", "chore", "ci", "docs", "perf", "refactor", "revert", "style", "test",
    ] {
        sandbox.write("repo/message", &format!("{kind}: update project"));
        sandbox.ok("git", &["hook", "run", "commit-msg", "--", "message"]);
    }
}

#[test]
fn installed_cocogitto_hook_rejects_invalid_messages_without_creating_commits() {
    let sandbox = Sandbox::new();

    install(&sandbox);

    for message in ["update project", "unknown: update project", "feat:", ""] {
        sandbox.blocked("git", &["commit", "--allow-empty", "-m", message], "Error");
        assert!(
            !sandbox
                .run("git", &["rev-parse", "--verify", "HEAD"])
                .status
                .success()
        );
    }
}

#[test]
fn cocogitto_installation_preserves_existing_hooks_when_replacement_is_declined() {
    let sandbox = Sandbox::new();

    install(&sandbox);
    sandbox.executable("repo/.git/hooks/commit-msg", "#!/bin/sh\nexit 42\n");

    let output = sandbox.input("cog", &["install-hook", "--all"], b"n\n");

    assert!(output.status.success());
    assert_eq!(
        fs::read_to_string(sandbox.path("repo/.git/hooks/commit-msg")).unwrap(),
        "#!/bin/sh\nexit 42\n"
    );
}

#[test]
fn cocogitto_and_mgh_enforce_messages_and_identity_without_replacing_shared_hooks() {
    let sandbox = Sandbox::new();

    sandbox.protect();

    let shared = sandbox.path("state/multigh/hooks/commit-msg");
    let original = fs::read(&shared).unwrap();
    let hooks_path = sandbox.ok("git", &["config", "--get", "core.hooksPath"]);

    install(&sandbox);

    assert_eq!(fs::read(&shared).unwrap(), original);
    assert_eq!(
        sandbox.ok("git", &["config", "--get", "core.hooksPath"]),
        hooks_path
    );

    sandbox.ok(
        "git",
        &["commit", "--allow-empty", "-m", "feat: add an identity"],
    );

    let head = sandbox.ok("git", &["rev-parse", "HEAD"]);

    sandbox.blocked(
        "git",
        &["commit", "--allow-empty", "-m", "update project"],
        "Existing commit-msg hook rejected",
    );
    sandbox.write("active", "bob\n");
    sandbox.blocked(
        "git",
        &["commit", "--allow-empty", "-m", "fix: update project"],
        "GitHub is using bob",
    );

    sandbox.ok("mgh", &["protections", "off"]);
    sandbox.blocked(
        "git",
        &["commit", "--allow-empty", "-m", "update project"],
        "Existing commit-msg hook rejected",
    );
    assert_eq!(sandbox.ok("git", &["rev-parse", "HEAD"]), head);
}
