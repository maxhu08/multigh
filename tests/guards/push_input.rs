use crate::support::Sandbox;

#[test]
fn malformed_updates_and_invalid_encoding_block_before_forwarding() {
    let sandbox = Sandbox::new();

    sandbox.protect();
    sandbox.executable(
        "repo/.git/hooks/pre-push",
        "#!/bin/sh\ntouch \"$TEST_MGH_ROOT/forwarded\"\n",
    );

    for (input, message) in [
        (b"missing fields\n".as_slice(), "Invalid push-hook input"),
        (b"local xyz remote 000\n".as_slice(), "Invalid commit ID"),
        (
            b"local -bad remote 0000000000000000000000000000000000000000\n".as_slice(),
            "Invalid commit ID",
        ),
        (b"\xff".as_slice(), "invalid utf-8"),
    ] {
        let output = sandbox.input(
            "mgh",
            &["internal", "hook", "pre-push", "origin", "../remote"],
            input,
        );

        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(message),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!sandbox.path("forwarded").exists());
    }
}

#[test]
fn deleting_refs_requires_allowed_authentication_but_does_not_scan_deleted_history() {
    let sandbox = Sandbox::new();

    sandbox.protect();

    for length in [40, 64] {
        let input = format!(
            "(delete) {} refs/heads/old {}\n",
            "0".repeat(length),
            "a".repeat(length)
        );
        let output = sandbox.input("mgh", &["internal", "hook", "pre-push"], input.as_bytes());

        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    sandbox.write("active", "bob\n");

    let input = format!(
        "(delete) {} refs/heads/old {}\n",
        "0".repeat(40),
        "a".repeat(40)
    );
    let output = sandbox.input("mgh", &["internal", "hook", "pre-push"], input.as_bytes());

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("GitHub is using bob"));
}

#[test]
fn missing_remote_baselines_require_fetching_before_a_push() {
    let sandbox = Sandbox::new();

    sandbox.protect();
    sandbox.commit();

    let head = sandbox.ok("git", &["rev-parse", "HEAD"]);
    let input = format!(
        "refs/heads/main {} refs/heads/main {}\n",
        head.trim(),
        "a".repeat(40)
    );
    let output = sandbox.input("mgh", &["internal", "hook", "pre-push"], input.as_bytes());

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Fetch the remote and try again"));
}
