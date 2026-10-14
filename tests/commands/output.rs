use crate::support::{Sandbox, terminal};

fn assert_padding(output: &str) {
    assert!(!output.trim().is_empty(), "expected a report");
    assert!(
        output.starts_with('\n') && !output.starts_with("\n\n"),
        "{output:?}"
    );
    assert!(
        output.ends_with("\n\n") && !output.ends_with("\n\n\n"),
        "{output:?}"
    );
    assert!(!output.contains("\n\n\n"), "{output:?}");
}

#[test]
fn reports_warnings_and_errors_have_one_blank_line_of_padding() {
    let sandbox = Sandbox::new();
    sandbox.protect();
    sandbox.ok("mgh", &["settings", "welcome", "on"]);

    for args in [
        vec!["status"],
        vec!["status", "--full"],
        vec!["identity"],
        vec!["repo", "status"],
        vec!["repo", "allowed"],
        vec!["repo", "protections"],
        vec!["settings"],
        vec!["settings", "verbose"],
        vec!["settings", "welcome"],
        vec!["settings", "autoswitch"],
        vec!["internal", "welcome"],
        vec!["switch", "personal"],
        vec!["setup"],
        vec!["doctor"],
        vec!["--help"],
        vec!["identity", "edit", "school", "--name", "Bob Example"],
        vec!["identity", "remove", "school"],
    ] {
        let output = sandbox.ok("mgh", &args);
        assert_padding(&output);
        assert!(
            !output.contains('\x1b'),
            "redirected output contains color: {output:?}"
        );
    }

    for (args, diagnostic) in [
        (vec!["switch", "missing"], "Unknown identity"),
        (vec!["--unknown"], "unexpected argument '--unknown'"),
        (vec!["identity", "new"], "Identity name requires a terminal"),
    ] {
        let output = sandbox.run("mgh", &args);
        assert!(!output.status.success());
        if !output.stdout.is_empty() {
            assert_padding(&String::from_utf8(output.stdout).unwrap());
        }
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(error.contains(diagnostic), "{error}");
        assert_padding(&error);
    }

    sandbox.write("accounts-json", "[]");
    let output = sandbox.run("mgh", &["status"]);
    assert!(!output.status.success());
    assert_padding(&String::from_utf8(output.stdout).unwrap());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains("No active GitHub account"), "{error}");
    assert_padding(&error);
}

#[test]
fn semantic_colors_are_shared_by_identity_repository_welcome_and_change_reports() {
    let sandbox = Sandbox::new();
    sandbox.protect();
    sandbox.ok("mgh", &["settings", "welcome", "on"]);

    let mut reports = Vec::new();
    for args in [
        vec!["identity"],
        vec!["repo", "status"],
        vec!["internal", "welcome"],
        vec!["repo", "allowed"],
        vec!["switch", "personal"],
        vec!["setup"],
        vec!["switch", "school"],
    ] {
        let (status, output) = terminal(
            sandbox.command("mgh").env_remove("NO_COLOR").args(&args),
            &[],
        );
        assert!(status.success(), "{output}");
        reports.push(output);
    }

    let (status, form) = terminal(
        sandbox
            .command("mgh")
            .env_remove("NO_COLOR")
            .args(["identity", "edit", "personal"]),
        &[
            ("GitHub username", b"\r"),
            ("Commit email", b"\r"),
            ("Commit name", b"\r"),
        ],
    );
    assert!(status.success(), "{form}");

    for (value, indexes) in [
        ("personal".to_owned(), vec![0, 1, 2, 3, 4, 5]),
        ("alice".to_owned(), vec![0, 1, 2, 3, 4]),
        ("Alice Example".to_owned(), vec![0, 1, 4, 6]),
        ("alice@example.com".to_owned(), vec![0, 1, 2, 4, 6]),
        ("bob".to_owned(), vec![0, 6]),
        ("Bob Example".to_owned(), vec![0, 4, 6]),
        ("bob@example.edu".to_owned(), vec![0, 4, 6]),
        (
            sandbox
                .path("config/multigh/identities.jsonc")
                .display()
                .to_string(),
            vec![0, 5],
        ),
        (
            sandbox
                .path("state/multigh/identities/git-personal.conf")
                .display()
                .to_string(),
            vec![0, 5],
        ),
    ] {
        let first = &reports[indexes[0]];
        let prefix = first.split_once(&value).unwrap().0;
        let style = prefix.rsplit_once("\x1b[").unwrap().1;
        assert!(
            style.ends_with('m'),
            "value is not independently colored: {first:?}"
        );
        let painted = format!("\x1b[{style}{value}\x1b[0m");
        if ["alice", "Alice Example", "alice@example.com"].contains(&value.as_str()) {
            assert!(
                form.lines()
                    .any(|line| line.contains(&painted) && !line.contains("default:")),
                "uncolored form value {value}: {form:?}"
            );
        }
        for index in indexes {
            assert!(
                reports[index].contains(&painted),
                "inconsistent color for {value}: {:?}",
                reports[index]
            );
        }
    }
}

#[test]
fn terminal_reports_share_padding_with_forms_and_errors_and_respect_color_preferences() {
    let sandbox = Sandbox::new();
    sandbox.protect();

    for keys in [b"\r".as_slice(), b"\x1b".as_slice()] {
        let (status, output) = terminal(
            sandbox.command("mgh").args(["repo", "allowed", "update"]),
            &[("Which identities may use this repository?", keys)],
        );
        assert_eq!(status.success(), keys == b"\r", "{output}");
        if !status.success() {
            assert!(output.contains("Operation cancelled."), "{output}");
        }
        assert_padding(&output.replace("\r\n", "\n"));
    }

    for (variable, value) in [("NO_COLOR", "1"), ("TERM", "dumb")] {
        let (status, output) = terminal(
            sandbox
                .command("mgh")
                .env_remove("NO_COLOR")
                .env(variable, value)
                .arg("status"),
            &[],
        );
        assert!(status.success(), "{output}");
        assert!(!output.contains('\x1b'), "{output:?}");
        assert_padding(&output.replace("\r\n", "\n"));
    }
}

#[test]
fn status_aligns_top_level_and_nested_report_values() {
    let sandbox = Sandbox::new();
    sandbox.protect();

    let output = sandbox.ok("mgh", &["status"]);

    assert!(
        output
            .lines()
            .any(|line| line == "    Commit name               Alice Example"),
        "{output}"
    );
    assert!(
        output
            .lines()
            .any(|line| line == "  Allowed identities          personal"),
        "{output}"
    );
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
