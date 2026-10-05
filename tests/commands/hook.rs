use crate::support::Sandbox;
use std::fs;

#[test]
fn dispatcher_rejects_unknown_hooks_and_forwards_native_arguments_and_binary_input_once() {
    let sandbox = Sandbox::new();

    sandbox.ok("mgh", &["repo", "protections", "off"]);
    sandbox.blocked("mgh", &["internal", "hook", "unknown"], "Unknown Git hook");

    for name in [
        "pre-push",
        "post-rewrite",
        "reference-transaction",
        "pre-receive",
        "prepare-commit-msg",
        "pre-rebase",
        "post-merge",
    ] {
        sandbox.executable(&format!("repo/.git/hooks/{name}"), "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$TEST_MGH_ROOT/arguments\"\nprintf 'called\\n' >> \"$TEST_MGH_ROOT/calls\"\ncat > \"$TEST_MGH_ROOT/input\"\n");
        sandbox.write("calls", "");

        let output = sandbox.input(
            "mgh",
            &["internal", "hook", name, "--option", "two words", ""],
            b"first\n\xffsecond\n",
        );

        assert!(
            output.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            fs::read_to_string(sandbox.path("arguments")).unwrap(),
            "--option\ntwo words\n\n"
        );
        assert_eq!(
            fs::read(sandbox.path("input")).unwrap(),
            b"first\n\xffsecond\n"
        );
        assert_eq!(
            fs::read_to_string(sandbox.path("calls")).unwrap(),
            "called\n"
        );
    }
}

#[test]
fn merge_commit_checks_run_before_existing_hooks_and_failed_authentication_fails_closed() {
    let sandbox = Sandbox::new();

    sandbox.protect();
    sandbox.executable(
        "repo/.git/hooks/pre-merge-commit",
        "#!/bin/sh\nprintf called > \"$TEST_MGH_ROOT/merged\"\n",
    );
    sandbox.write("fail-auth", "");
    sandbox.blocked("mgh", &["internal", "hook", "pre-merge-commit"], "gh:");

    assert!(!sandbox.path("merged").exists());

    fs::remove_file(sandbox.path("fail-auth")).unwrap();
    sandbox.ok("mgh", &["internal", "hook", "pre-merge-commit"]);

    assert_eq!(
        fs::read_to_string(sandbox.path("merged")).unwrap(),
        "called"
    );
}

#[test]
fn ordinary_checkout_skips_initial_clone_selection_and_still_forwards_existing_hook() {
    let sandbox = Sandbox::new();

    sandbox.ok("mgh", &["setup"]);
    sandbox.executable(
        "repo/.git/hooks/post-checkout",
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$TEST_MGH_ROOT/checkout\"\n",
    );

    let output = sandbox.ok(
        "mgh",
        &[
            "internal",
            "hook",
            "post-checkout",
            &"a".repeat(40),
            &"b".repeat(40),
            "1",
        ],
    );

    assert!(output.is_empty());
    assert_eq!(
        fs::read_to_string(sandbox.path("checkout")).unwrap(),
        format!("{}\n{}\n1\n", "a".repeat(40), "b".repeat(40))
    );
    assert!(
        !sandbox
            .run("git", &["config", "--get-all", "mgh.allowed-identity"])
            .status
            .success()
    );
}
