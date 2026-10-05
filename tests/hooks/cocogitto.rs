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

    sandbox.ok("mgh", &["repo", "protections", "off"]);
    sandbox.blocked(
        "git",
        &["commit", "--allow-empty", "-m", "update project"],
        "Existing commit-msg hook rejected",
    );
    assert_eq!(sandbox.ok("git", &["rev-parse", "HEAD"]), head);
}
