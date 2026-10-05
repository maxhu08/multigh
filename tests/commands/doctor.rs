use crate::support::Sandbox;
use std::fs;

#[test]
fn doctor_reports_problems_and_is_read_only() {
    let sandbox = Sandbox::new();
    sandbox.blocked("mgh", &["doctor"], "Resolve the reported problems");
    sandbox.protect();
    let global = fs::read(sandbox.path("gitconfig")).unwrap();
    let local = fs::read(sandbox.path("repo/.git/config")).unwrap();
    assert!(
        sandbox
            .ok("mgh", &["doctor"])
            .contains("Installation checks passed")
    );
    sandbox.write("fail-auth", "");
    sandbox.blocked("mgh", &["doctor"], "Resolve the reported problems");
    assert_eq!(fs::read(sandbox.path("gitconfig")).unwrap(), global);
    assert_eq!(fs::read(sandbox.path("repo/.git/config")).unwrap(), local);
}

#[test]
fn doctor_reports_tool_and_configuration_failures_without_repairing_them() {
    let sandbox = Sandbox::new();
    sandbox.protect();
    let global = fs::read(sandbox.path("gitconfig")).unwrap();
    sandbox.executable(
        "bin/gh",
        "#!/bin/sh\nprintf 'CLI unavailable\\n' >&2\nexit 1\n",
    );
    sandbox.write("config/multigh/identities.jsonc", "invalid");
    let result = sandbox.run("mgh", &["doctor"]);
    let output = String::from_utf8_lossy(&result.stdout);

    assert!(!result.status.success());
    assert!(output.contains("CLI unavailable"));
    assert!(output.contains("invalid identity configuration"));
    assert_eq!(fs::read(sandbox.path("gitconfig")).unwrap(), global);
    assert_eq!(
        fs::read_to_string(sandbox.path("config/multigh/identities.jsonc")).unwrap(),
        "invalid"
    );
}
