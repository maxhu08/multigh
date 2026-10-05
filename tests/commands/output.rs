use crate::support::{Sandbox, terminal};

#[test]
fn changes_are_spaced_and_highlighted_only_on_color_capable_terminals() {
    let sandbox = Sandbox::new();

    let (status, output) = terminal(
        sandbox
            .command("mgh")
            .env_remove("NO_COLOR")
            .args(["switch", "personal"]),
        &[],
    );

    assert!(status.success(), "{output}");
    assert!(
        output.contains("→") && output.contains("\x1b[1;38;2;134;239;172m"),
        "{output}"
    );
    assert!(output.contains("\r\n\r\n"), "{output}");

    for no_color in [true, false] {
        let mut command = sandbox.command("mgh");

        if !no_color {
            command.env_remove("NO_COLOR").env("TERM", "dumb");
        }

        let (status, output) = terminal(command.args(["settings", "verbose", "on"]), &[]);

        assert!(status.success(), "{output}");
        assert!(output.contains("Verbose") && output.contains("ON"));
        assert!(!output.contains("\x1b["), "{output}");
    }

    let output = sandbox
        .command("mgh")
        .env_remove("NO_COLOR")
        .env("TERM", "xterm-256color")
        .args(["settings", "verbose", "on"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("\x1b["));
}

#[test]
fn errors_are_spaced_on_stderr_and_terminal_warnings_are_highlighted() {
    let sandbox = Sandbox::new();

    let output = sandbox.run("mgh", &["switch", "missing"]);

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());

    let error = String::from_utf8_lossy(&output.stderr);

    assert!(
        error.starts_with("\n  ✕ mgh\n") && error.ends_with("\n\n"),
        "{error}"
    );

    let (status, output) = terminal(
        sandbox
            .command("mgh")
            .env_remove("NO_COLOR")
            .args(["switch", "missing"]),
        &[],
    );

    assert!(!status.success());
    assert!(output.contains("\x1b[1;38;2;251;113;133m"), "{output}");

    sandbox.ok("mgh", &["setup"]);
    sandbox.ok("mgh", &["settings", "welcome", "on"]);
    sandbox.allow("school");

    let (status, output) = terminal(
        sandbox
            .command("mgh")
            .env_remove("NO_COLOR")
            .args(["internal", "welcome"]),
        &[],
    );

    assert!(status.success(), "{output}");
    assert!(output.contains("\x1b[1;38;2;251;191;36m"), "{output}");
}

#[test]
fn cliclack_inputs_and_checklists_respect_no_color() {
    for no_color in [true, false] {
        let sandbox = Sandbox::new();

        for (args, question, keys) in [
            (
                vec!["identity", "new"],
                "Identity name: ",
                b"\x1b".as_slice(),
            ),
            (
                vec!["repo", "allowed", "update"],
                "Which identities may use this repository?",
                b" \r".as_slice(),
            ),
        ] {
            let mut command = sandbox.command("mgh");

            if !no_color {
                command.env_remove("NO_COLOR");
            }

            let (_, output) = terminal(command.args(args), &[(question, keys)]);
            let colored = output
                .split("\x1b[")
                .skip(1)
                .any(|sequence| sequence.chars().find(|c| c.is_ascii_alphabetic()) == Some('m'));

            assert_eq!(colored, !no_color, "{output}");
        }
    }
}

#[test]
fn forms_with_redirected_stderr_reject_interaction_without_changing_settings() {
    let sandbox = Sandbox::new();
    let before = std::fs::read(sandbox.path("config/multigh/identities.jsonc")).unwrap();

    for (command, message) in [
        ("identity new", "requires a terminal"),
        ("repo allowed update", "needs an interactive terminal"),
    ] {
        let (status, output) = terminal(
            sandbox
                .command("sh")
                .args(["-c", &format!("exec mgh {command} 2> ../form-error")]),
            &[],
        );

        assert!(!status.success(), "{output}");
        assert!(
            std::fs::read_to_string(sandbox.path("form-error"))
                .unwrap()
                .contains(message)
        );
        assert_eq!(
            before,
            std::fs::read(sandbox.path("config/multigh/identities.jsonc")).unwrap()
        );
        assert!(!sandbox.path("gh-calls").exists());
    }

    assert!(
        !sandbox
            .run("git", &["config", "--get-all", "mgh.allowed-identity"])
            .status
            .success()
    );
}

#[test]
fn warnings_and_full_authentication_reports_indent_all_nonempty_lines() {
    let sandbox = Sandbox::new();
    sandbox.protect();
    sandbox.write("active", "bob\n");
    let output = sandbox.ok("mgh", &["status", "--full"]);
    assert!(output.contains("\n  Allowed identities: personal"));
    assert!(output.contains("\n  Run: mgh switch <allowed-identity>"));
    assert!(output.contains("\n  GitHub authentication details"));
    assert!(
        output
            .lines()
            .filter(|line| !line.is_empty())
            .all(|line| line.starts_with("  ")),
        "{output}"
    );

    sandbox.write(
        "full-auth-report",
        "github.com\n  ✓ Logged in\n  - Active account: true\n\n",
    );
    let report = sandbox.run("mgh", &["status", "--full"]);

    assert!(report.status.success());
    assert_eq!(
        String::from_utf8_lossy(&report.stderr),
        "  github.com\n    ✓ Logged in\n    - Active account: true\n  \n"
    );
}

#[test]
fn headings_rows_changes_and_nested_details_share_one_value_column() {
    let sandbox = Sandbox::new();
    sandbox.protect();
    sandbox.ok("mgh", &["settings", "welcome", "on"]);
    let repo_config = sandbox
        .path("repo/.git/config")
        .canonicalize()
        .unwrap()
        .display()
        .to_string();

    for (command, values) in [
        (
            vec!["repo", "status"],
            vec![
                ("Current repository", "repo"),
                ("Repo config", repo_config.as_str()),
                ("Active identity", "personal"),
                ("GitHub username", "alice"),
                ("Allowed identities", "personal"),
                ("Commit name", "Alice Example"),
            ],
        ),
        (
            vec!["switch", "personal"],
            vec![("✓ Identity selected", "personal"), ("Name", "Bob Example")],
        ),
        (
            vec!["status"],
            vec![
                ("GitHub username", "alice"),
                ("Commit name", "Alice Example"),
            ],
        ),
        (
            vec!["internal", "welcome"],
            vec![
                ("Identity", "personal"),
                ("Commit email", "alice@example.com"),
            ],
        ),
    ] {
        let output = sandbox.ok("mgh", &command);
        assert!(!output.contains('·'), "{output}");

        for (label, value) in values {
            let line = output
                .lines()
                .find(|line| line.trim_start().starts_with(label))
                .unwrap();
            let after_label = line.find(label).unwrap() + label.len();
            let start = after_label + line[after_label..].find(value).unwrap();
            assert_eq!(line[..start].chars().count(), 30, "{line}");
        }
    }
}

#[test]
fn long_labels_wrap_instead_of_moving_the_value_column() {
    let sandbox = Sandbox::new();
    let identity = "a_very_long_identity_name_that_exceeds_the_label_column";
    let config = crate::support::IDENTITIES.replace("\"personal\"", &format!("\"{identity}\""));
    sandbox.write("config/multigh/identities.jsonc", &config);
    sandbox.ok("mgh", &["setup"]);
    let output = sandbox.ok("mgh", &["repo", "allowed", "add", identity]);

    let mut lines = output.lines();
    let label = lines.find(|line| line.trim() == identity).unwrap();
    assert!(label.starts_with("  "));
    let value = lines.next().unwrap();
    assert_eq!(value.trim(), "alice");
    assert_eq!(value.find("alice"), Some(30));
}

#[test]
fn form_titles_use_the_same_value_column_without_dot_separators() {
    let sandbox = Sandbox::new();

    for (args, label, prompt) in [
        (
            vec!["identity", "edit", "personal"],
            "Edit identity",
            "GitHub username",
        ),
        (
            vec!["repo", "allowed", "update"],
            "Repository",
            "Which identities may use this repository?",
        ),
    ] {
        let (_, output) = terminal(sandbox.command("mgh").args(args), &[(prompt, b"\x1b")]);
        let line = output.lines().find(|line| line.contains(label)).unwrap();
        let after_label = line.find(label).unwrap() + label.len();
        let value = after_label
            + line[after_label..]
                .find(|c: char| !c.is_whitespace())
                .unwrap();

        assert_eq!(line[..value].chars().count(), 30, "{line}");
        assert!(!output.contains('·'), "{output}");
    }
}
