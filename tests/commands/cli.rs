use crate::support::Sandbox;

#[test]
fn root_help_lists_commands_without_examples_or_internal_handlers() {
    let sandbox = Sandbox::new();

    for args in [vec![], vec!["-h"], vec!["--help"]] {
        let result = sandbox.run("mgh", &args);
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );

        assert!(text.contains("Usage:") && text.contains("Commands:"));

        for command in [
            "new",
            "switch",
            "status",
            "welcome",
            "setup",
            "protections",
            "verbose",
            "init",
            "check",
            "completions",
        ] {
            assert!(
                text.lines()
                    .any(|line| line.trim_start().starts_with(command))
            );
        }

        assert!(!text.contains("Examples:") && !text.contains(" protect "));
        assert!(
            !text
                .lines()
                .any(|line| line.trim_start().starts_with("enter ")
                    || line.trim_start().starts_with("hook "))
        );
        assert_eq!(result.status.success(), !args.is_empty());
    }

    assert_eq!(
        sandbox.ok("mgh", &["--version"]).trim(),
        concat!("mgh ", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn every_command_supports_detailed_help() {
    let sandbox = Sandbox::new();

    for command in [
        "new",
        "switch",
        "status",
        "welcome",
        "setup",
        "protections",
        "verbose",
        "init",
        "check",
        "completions",
        "hook",
    ] {
        let short = sandbox.ok("mgh", &[command, "-h"]);

        assert_eq!(short, sandbox.ok("mgh", &[command, "--help"]));
        assert_eq!(short, sandbox.ok("mgh", &["help", command]));
        assert!(
            short.contains("Details:") && short.contains("Usage:"),
            "{command}"
        );
    }

    let help = sandbox.ok("mgh", &["protections", "-h"]);

    assert!(help.contains("personal,school,work"));
    assert!(!help.contains("personal,school\n"));
}

#[test]
fn invalid_commands_arguments_and_conflicting_options_are_rejected() {
    let sandbox = Sandbox::new();

    for args in [
        vec!["protect", "personal"],
        vec!["switch"],
        vec!["welcome", "maybe"],
        vec!["verbose", "maybe"],
        vec!["protections", "maybe"],
        vec!["protections", "on", "--repo"],
        vec!["protections", "off", "--allow", "personal"],
        vec!["protections", "--repo", "--allow", "personal"],
        vec!["protections", "--allow"],
        vec!["init", "unknown"],
        vec!["completions", "unknown"],
        vec!["status", "--unknown"],
        vec!["hook", "push"],
    ] {
        sandbox.blocked("mgh", &args, "error:");
    }
}
