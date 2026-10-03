use crate::support::Sandbox;

#[test]
fn check_is_silent_on_success_and_still_validates_when_protections_are_off() {
    let sandbox = Sandbox::new();

    sandbox.protect();

    assert!(sandbox.ok("mgh", &["check"]).is_empty());

    sandbox.ok("mgh", &["protections", "off"]);
    sandbox.write("active", "bob\n");
    sandbox.blocked("mgh", &["check"], "not allowed in this repository");
    sandbox.write("active", "alice\n");
    sandbox.ok("git", &["config", "user.name", "Wrong Name"]);
    sandbox.blocked("mgh", &["check"], "AUTHOR identity");
}

#[test]
fn check_rejects_missing_authorization_or_authentication_and_skips_non_repos() {
    let sandbox = Sandbox::new();

    sandbox.blocked("mgh", &["check"], "No accounts are authorized");
    sandbox.protect();
    sandbox.write("fail-auth", "");
    sandbox.blocked("mgh", &["check"], "gh:");

    let outside = sandbox
        .command("mgh")
        .current_dir(sandbox.path(""))
        .arg("check")
        .output()
        .unwrap();

    assert!(outside.status.success() && outside.stdout.is_empty());
}
