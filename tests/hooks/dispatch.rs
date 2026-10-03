use crate::support::Sandbox;
use std::fs;

#[test]
fn every_installed_launcher_forwards_existing_hooks_when_protections_are_off() {
    let sandbox = Sandbox::new();

    sandbox.protect();
    sandbox.ok("mgh", &["protections", "off"]);
    fs::remove_file(sandbox.path("config/multigh/accounts.conf")).unwrap();

    for name in [
        "applypatch-msg",
        "pre-applypatch",
        "post-applypatch",
        "pre-commit",
        "pre-merge-commit",
        "prepare-commit-msg",
        "commit-msg",
        "post-commit",
        "pre-rebase",
        "pre-push",
        "post-checkout",
        "post-merge",
        "pre-receive",
        "update",
        "proc-receive",
        "post-receive",
        "post-update",
        "reference-transaction",
        "push-to-checkout",
        "pre-auto-gc",
        "post-rewrite",
        "sendemail-validate",
        "fsmonitor-watchman",
    ] {
        sandbox.executable(&format!("repo/.git/hooks/{name}"), "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$TEST_MGH_ROOT/arguments\"\ncat > \"$TEST_MGH_ROOT/input\"\n");

        let launcher = sandbox.path(&format!("state/multigh/hooks/{name}"));
        let output = sandbox.input(
            launcher.to_str().unwrap(),
            &["one argument", "--second"],
            b"standard hook input\n",
        );

        assert!(
            output.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            fs::read_to_string(sandbox.path("arguments")).unwrap(),
            "one argument\n--second\n",
            "{name}"
        );
        assert_eq!(
            fs::read(sandbox.path("input")).unwrap(),
            b"standard hook input\n",
            "{name}"
        );
    }
}
