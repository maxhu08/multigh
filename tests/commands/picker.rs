use crate::support::{Sandbox, terminal};
use std::fs;

const QUESTION: &str = "Which identities may use this repository?";

#[test]
fn checklist_selects_multiple_identities_and_reopens_with_saved_selections() {
    let sandbox = Sandbox::new();

    let global = fs::read(sandbox.path("gitconfig")).unwrap();
    let (status, output) = terminal(
        sandbox.command("mgh").args(["protections", "--repo"]),
        &[(QUESTION, b" \x1b[B \r")],
    );

    assert!(status.success(), "{output}");
    assert_eq!(
        sandbox.ok("git", &["config", "--get-all", "mgh.allowedAccount"]),
        "personal\nschool\n"
    );
    assert_eq!(fs::read(sandbox.path("gitconfig")).unwrap(), global);
    assert_eq!(
        fs::read_to_string(sandbox.path("active")).unwrap(),
        "alice\n"
    );

    let (status, output) = terminal(
        sandbox.command("mgh").args(["protections", "--repo"]),
        &[(QUESTION, b" \r")],
    );

    assert!(status.success(), "{output}");
    assert_eq!(
        sandbox.ok("git", &["config", "--get-all", "mgh.allowedAccount"]),
        "school\n"
    );
    assert_eq!(
        sandbox.ok("git", &["config", "user.email"]),
        "bob@example.edu\n"
    );
}

#[test]
fn empty_selection_is_rejected_and_cancellation_preserves_policy() {
    let sandbox = Sandbox::new();

    let (status, output) = terminal(
        sandbox.command("mgh").args(["protections", "--repo"]),
        &[(QUESTION, b"\r"), ("Input required", b" \r")],
    );

    assert!(status.success(), "{output}");

    let before = fs::read(sandbox.path("repo/.git/config")).unwrap();
    for keys in [b" \x1b".as_slice(), b" \x03".as_slice()] {
        let (status, output) = terminal(
            sandbox.command("mgh").args(["protections", "--repo"]),
            &[(QUESTION, keys)],
        );

        assert!(!status.success(), "{output}");
        if keys.ends_with(b"\x1b") {
            assert!(output.contains("Operation cancelled."), "{output}");
        }
        assert_eq!(fs::read(sandbox.path("repo/.git/config")).unwrap(), before);
    }
}

#[test]
fn checklist_scrolls_through_many_identities_and_saves_the_selected_name() {
    let sandbox = Sandbox::new();
    let mut configuration = serde_json::Map::new();

    for index in 0..12 {
        configuration.insert(
            format!("work{index:02}"),
            serde_json::json!({"username":format!("user{index}"), "commit":{"email":format!("user{index}@example.com")}}),
        );
    }

    sandbox.write(
        "config/multigh/identities.jsonc",
        &serde_json::to_string(&configuration).unwrap(),
    );

    let mut keys = b"\x1b[B".repeat(11);

    keys.extend_from_slice(b" \r");

    let (status, output) = terminal(
        sandbox.command("mgh").args(["protections", "--repo"]),
        &[(QUESTION, &keys)],
    );

    assert!(status.success(), "{output}");
    assert!(output.contains("Allowed identities saved"), "{output}");
    assert_eq!(
        sandbox.ok("git", &["config", "--get-all", "mgh.allowedAccount"]),
        "work11\n"
    );
    assert_eq!(
        sandbox.ok("git", &["config", "--local", "user.email"]),
        "user11@example.com\n"
    );
}

#[test]
fn cancelled_entry_keeps_unconfigured_repos_blocked_without_breaking_shell_startup() {
    let sandbox = Sandbox::new();

    sandbox.ok("mgh", &["setup"]);

    let (status, output) = terminal(sandbox.command("mgh").arg("enter"), &[(QUESTION, b"\x1b")]);

    assert!(status.success(), "{output}");
    assert!(output.contains("commits and pushes remain blocked"));
    assert!(
        !sandbox
            .run("git", &["config", "--get-all", "mgh.allowedAccount"])
            .status
            .success()
    );

    sandbox.blocked("mgh", &["check"], "No identities are authorized");
}

#[test]
fn interactive_clone_saves_the_selected_identities_in_the_new_repo() {
    let sandbox = Sandbox::new();

    sandbox.protect();
    sandbox.commit();

    let (status, output) = terminal(
        sandbox.command("git").args(["clone", ".", "../picked"]),
        &[(QUESTION, b" \x1b[B \r")],
    );

    assert!(status.success(), "{output}");
    assert_eq!(
        sandbox.ok(
            "git",
            &[
                "-C",
                "../picked",
                "config",
                "--get-all",
                "mgh.allowedAccount"
            ]
        ),
        "personal\nschool\n"
    );

    let output = sandbox
        .command("mgh")
        .current_dir(sandbox.path("picked"))
        .arg("check")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
