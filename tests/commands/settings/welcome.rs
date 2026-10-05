use crate::support::{Sandbox, terminal};
use std::fs;

#[test]
fn welcome_aligns_identity_with_values_without_a_separator() {
    let sandbox = Sandbox::new();

    sandbox.write(
        "config/multigh/identities.jsonc",
        r#"{"PeRsOnAl": {"username": "alice", "commit": {"email": "alice@example.com"}}}"#,
    );
    sandbox.write("selected", "ALICE\n");
    sandbox.ok("mgh", &["settings", "welcome", "on"]);

    let output = sandbox.ok("mgh", &["internal", "welcome"]);

    assert!(
        output
            .contains("Identity                    personal\n  GitHub username             ALICE"),
        "{output}"
    );
    let heading = output
        .lines()
        .find(|line| line.contains("Identity"))
        .unwrap();
    let username = output
        .lines()
        .find(|line| line.contains("GitHub username"))
        .unwrap();
    let email = output
        .lines()
        .find(|line| line.contains("Commit email"))
        .unwrap();

    assert_eq!(heading.find("personal"), username.find("ALICE"));
    assert_eq!(heading.find("personal"), email.find("bob@example.edu"));
    assert!(!output.contains("\x1b["));
    assert!(
        !output.contains("Selected identity")
            && !output.contains("·")
            && !output.contains("(personal)"),
        "{output}"
    );
}

#[test]
fn welcome_preferences_work_without_config_and_do_not_modify_git_or_auth() {
    let sandbox = Sandbox::new();

    let before = fs::read(sandbox.path("gitconfig")).unwrap();

    fs::remove_file(sandbox.path("config/multigh/identities.jsonc")).unwrap();

    assert!(sandbox.ok("mgh", &["internal", "welcome"]).is_empty());

    assert_eq!(
        sandbox.ok("mgh", &["settings", "welcome", "on"]),
        "  Welcome                     ON\n  Show the active identity in the terminal greeting.\n\n"
    );

    assert!(sandbox.path("state/multigh/welcome-enabled").is_file());
    assert!(!sandbox.path("gh-calls").exists());

    assert_eq!(
        sandbox.ok("mgh", &["settings", "welcome", "off"]),
        "  Welcome                     OFF\n  Show the active identity in the terminal greeting.\n\n"
    );
    sandbox.ok("mgh", &["settings", "welcome", "off"]);

    assert!(sandbox.ok("mgh", &["internal", "welcome"]).is_empty());
    assert_eq!(before, fs::read(sandbox.path("gitconfig")).unwrap());
}

#[test]
fn welcome_uses_local_selection_and_reports_repo_or_identity_mismatches() {
    let sandbox = Sandbox::new();

    sandbox.protect();
    sandbox.ok("mgh", &["settings", "welcome", "on"]);
    sandbox.write("selected", "bob\n");
    fs::remove_file(sandbox.path("gh-calls")).unwrap();

    let text = sandbox.ok("mgh", &["internal", "welcome"]);

    assert!(text.starts_with("\n  Identity                    school\n"));
    assert!(text.contains("GitHub username             bob"));
    assert!(text.contains("mgh switch personal"));
    assert!(!text.contains("~~~"));

    let calls = fs::read_to_string(sandbox.path("gh-calls")).unwrap();

    assert!(calls.contains("config get") && !calls.contains("auth status"));

    sandbox.write("selected", "alice\n");
    sandbox.ok("git", &["config", "--local", "user.name", "Wrong Name"]);

    assert!(
        sandbox
            .ok("mgh", &["internal", "welcome"])
            .contains("Commit details need: mgh switch personal")
    );

    sandbox.ok("git", &["config", "--local", "user.name", "Alice Example"]);
    sandbox.ok(
        "git",
        &[
            "config",
            "--local",
            "user.email",
            "123+ALICE@users.noreply.github.com",
        ],
    );

    assert!(
        !sandbox
            .ok("mgh", &["internal", "welcome"])
            .contains("Commit details need:")
    );
}

#[test]
fn welcome_handles_unavailable_selection_and_missing_config_without_breaking_startup() {
    let sandbox = Sandbox::new();

    sandbox.protect();
    sandbox.ok("mgh", &["settings", "welcome", "on"]);
    sandbox.write("fail-selected", "");

    assert!(
        sandbox
            .ok("mgh", &["internal", "welcome"])
            .contains("unavailable")
    );

    fs::remove_file(sandbox.path("config/multigh/identities.jsonc")).unwrap();

    let welcome = sandbox.ok("mgh", &["internal", "welcome"]);

    assert!(
        welcome.contains("GitHub username             unavailable") && welcome.contains("Read ")
    );
}

#[test]
fn welcome_identity_is_green_and_label_is_purple_only_on_color_capable_terminals() {
    let sandbox = Sandbox::new();

    sandbox.ok("mgh", &["settings", "welcome", "on"]);

    for mode in ["color", "no-color", "dumb"] {
        let mut command = sandbox.command("mgh");

        command.env("TERM", "xterm-256color");

        if mode != "no-color" {
            command.env_remove("NO_COLOR");
        }

        if mode == "dumb" {
            command.env("TERM", "dumb");
        }

        let (status, output) = terminal(command.args(["internal", "welcome"]), &[]);

        assert!(status.success(), "{output}");

        if mode == "color" {
            assert!(
                output.contains("\x1b[1;38;2;192;132;252mIdentity\x1b[0m"),
                "{output}"
            );
            assert!(
                output.contains("\x1b[1;38;2;134;239;172mpersonal\x1b[0m"),
                "{output}"
            );
        } else {
            assert!(
                output.contains("Identity                    personal\r\n"),
                "{output}"
            );
            assert!(!output.contains("\x1b["), "{output}");
        }
    }
}
