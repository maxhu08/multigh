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

        let (status, output) = terminal(command.args(["verbose", "on"]), &[]);

        assert!(status.success(), "{output}");
        assert!(output.contains("Verbose") && output.contains("ON"));
        assert!(!output.contains("\x1b["), "{output}");
    }

    let output = sandbox
        .command("mgh")
        .env_remove("NO_COLOR")
        .env("TERM", "xterm-256color")
        .args(["verbose", "on"])
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
    sandbox.ok("mgh", &["welcome", "on"]);
    sandbox.ok("mgh", &["protections", "--allow", "school"]);

    let (status, output) = terminal(
        sandbox.command("mgh").env_remove("NO_COLOR").arg("welcome"),
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
            (vec!["new"], "Account alias: ", b"\x1b".as_slice()),
            (
                vec!["protections", "--repo"],
                "Which accounts may use this repository?",
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
    let before = std::fs::read(sandbox.path("config/multigh/accounts.conf")).unwrap();

    for (command, message) in [
        ("new", "requires a terminal"),
        ("protections --repo", "needs an interactive terminal"),
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
            std::fs::read(sandbox.path("config/multigh/accounts.conf")).unwrap()
        );
        assert!(!sandbox.path("gh-calls").exists());
    }

    assert!(
        !sandbox
            .run("git", &["config", "--get-all", "mgh.allowedAccount"])
            .status
            .success()
    );
}
