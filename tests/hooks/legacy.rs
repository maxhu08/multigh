use crate::support::Sandbox;
use std::fs;

#[test]
fn legacy_guards_migrate_and_missing_config_fails_closed() {
    let sandbox = Sandbox::new();

    sandbox.ok("git", &["config", "ghguard.account", "personal"]);
    sandbox.executable(
        "repo/.git/hooks/pre-commit",
        "#!/bin/sh\n# Managed by ghguard; existing hooks are preserved.\nexit 1\n",
    );
    sandbox.executable(
        "repo/.git/hooks/pre-commit.before-ghguard",
        "#!/bin/sh\nprintf 'legacy\\n' >> \"$TEST_MGH_ROOT/commits\"\n",
    );
    sandbox.protect();

    assert!(
        !sandbox
            .run("git", &["config", "ghguard.account"])
            .status
            .success()
    );
    assert_eq!(
        sandbox.ok("git", &["config", "mgh.account"]).trim(),
        "personal"
    );

    sandbox.commit();

    assert_eq!(
        fs::read_to_string(sandbox.path("commits")).unwrap(),
        "legacy\n"
    );

    fs::remove_file(sandbox.path("config/multigh/identities.jsonc")).unwrap();
    sandbox.blocked(
        "git",
        &["commit", "--allow-empty", "-m", "Missing config"],
        "identities.jsonc",
    );
    sandbox.ok("mgh", &["--help"]);
    sandbox.ok("mgh", &["completions", "fish"]);
}
