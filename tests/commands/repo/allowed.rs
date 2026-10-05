use crate::support::Sandbox;
use std::fs;

#[test]
fn allowed_add_remove_and_bare_defaults_preserve_authentication_and_block_an_empty_list() {
    let sandbox = Sandbox::new();
    sandbox.protect();
    sandbox.ok("mgh", &["repo", "allowed", "add", "SCHOOL"]);
    sandbox.ok("mgh", &["repo", "allowed", "add", "school"]);
    assert_eq!(
        sandbox.ok("git", &["config", "--get-all", "mgh.allowed-identity"]),
        "personal\nschool\n"
    );
    assert_eq!(
        sandbox.ok("mgh", &["repo", "allowed"]),
        sandbox.ok("mgh", &["repo", "allowed", "list"])
    );
    assert_eq!(
        sandbox.ok("mgh", &["repo"]),
        sandbox.ok("mgh", &["repo", "status"])
    );
    sandbox.ok("mgh", &["repo", "allowed", "remove", "SCHOOL"]);
    sandbox.ok("mgh", &["repo", "allowed", "remove", "personal"]);
    assert!(
        sandbox
            .ok("mgh", &["repo", "allowed"])
            .contains("No identities selected")
    );
    assert_eq!(
        fs::read_to_string(sandbox.path("active")).unwrap(),
        "alice\n"
    );
    sandbox.blocked(
        "git",
        &["commit", "--allow-empty", "-m", "Blocked"],
        "No identities are authorized",
    );
    sandbox.ok("mgh", &["repo", "allowed", "add", "personal"]);
    sandbox.commit();
}

#[test]
fn removed_global_identities_can_be_revoked_individually_or_replaced_with_the_picker() {
    for picker in [false, true] {
        let sandbox = Sandbox::new();
        sandbox.protect();
        sandbox.ok("mgh", &["repo", "allowed", "add", "school"]);
        sandbox.ok("mgh", &["identity", "remove", "school"]);
        assert!(
            sandbox
                .ok("mgh", &["repo", "allowed"])
                .contains("Not configured")
        );

        if picker {
            let (status, output) = crate::support::terminal(
                sandbox.command("mgh").args(["repo", "allowed", "update"]),
                &[("Which identities may use this repository?", b"\r")],
            );
            assert!(status.success(), "{output}");
            assert_eq!(
                sandbox.ok("git", &["config", "--get-all", "mgh.allowed-identity"]),
                "personal\n"
            );
            sandbox.commit();
        } else {
            sandbox.ok("mgh", &["identity", "remove", "personal"]);
            sandbox.ok("mgh", &["repo", "allowed", "remove", "PERSONAL"]);
            sandbox.blocked(
                "git",
                &["commit", "--allow-empty", "-m", "Blocked"],
                "Unknown identity",
            );
            sandbox.ok("mgh", &["repo", "allowed", "remove", "school"]);
            assert!(
                sandbox
                    .ok("mgh", &["repo", "allowed"])
                    .contains("No identities selected")
            );
        }
    }
}
