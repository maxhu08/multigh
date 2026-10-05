use crate::support::Sandbox;

#[test]
fn check_is_silent_on_success_and_still_validates_when_protections_are_off() {
    let sandbox = Sandbox::new();

    sandbox.protect();

    assert!(sandbox.ok("mgh", &["repo", "check"]).is_empty());

    sandbox.ok("mgh", &["repo", "protections", "off"]);
    sandbox.write("active", "bob\n");
    sandbox.blocked("mgh", &["repo", "check"], "not allowed in this repository");
    sandbox.write("active", "alice\n");
    sandbox.ok("git", &["config", "user.name", "Wrong Name"]);
    sandbox.blocked("mgh", &["repo", "check"], "AUTHOR commit details");
}

#[test]
fn check_rejects_missing_authorization_or_authentication_and_skips_non_repos() {
    let sandbox = Sandbox::new();

    sandbox.blocked("mgh", &["repo", "check"], "No identities are authorized");
    sandbox.protect();
    sandbox.write("fail-auth", "");
    sandbox.blocked("mgh", &["repo", "check"], "gh:");

    let outside = sandbox
        .command("mgh")
        .current_dir(sandbox.path(""))
        .args(["repo", "check"])
        .output()
        .unwrap();

    assert!(!outside.status.success());
    assert!(String::from_utf8_lossy(&outside.stderr).contains("No Git repository"));
}
