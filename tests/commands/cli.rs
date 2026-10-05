use crate::support::Sandbox;

#[test]
fn root_help_shows_only_the_redesigned_public_commands() {
    let sandbox = Sandbox::new();
    let help = sandbox.ok("mgh", &["--help"]);
    for command in [
        "setup", "status", "switch", "doctor", "identity", "repo", "settings", "shell",
    ] {
        assert!(
            help.lines()
                .any(|line| line.trim_start().starts_with(&format!("{command} "))),
            "{help}"
        );
    }
    assert!(
        !help
            .lines()
            .any(|line| line.trim_start().starts_with("internal "))
    );
    assert_eq!(
        sandbox.ok("mgh", &["--version"]).trim(),
        concat!("mgh ", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn every_public_command_has_consistent_short_long_and_help_forms() {
    let sandbox = Sandbox::new();
    for command in [
        vec!["setup"],
        vec!["status"],
        vec!["switch"],
        vec!["doctor"],
        vec!["identity"],
        vec!["identity", "list"],
        vec!["identity", "new"],
        vec!["identity", "edit"],
        vec!["identity", "remove"],
        vec!["repo"],
        vec!["repo", "status"],
        vec!["repo", "check"],
        vec!["repo", "protections"],
        vec!["settings", "autoswitch"],
        vec!["repo", "allowed"],
        vec!["repo", "allowed", "list"],
        vec!["repo", "allowed", "add"],
        vec!["repo", "allowed", "remove"],
        vec!["repo", "allowed", "update"],
        vec!["settings"],
        vec!["settings", "verbose"],
        vec!["settings", "welcome"],
        vec!["shell"],
        vec!["shell", "init"],
        vec!["shell", "completions"],
    ] {
        let mut short = command.clone();
        short.push("-h");
        let mut long = command.clone();
        long.push("--help");
        let mut help = vec!["help"];
        help.extend(command);
        let output = sandbox.ok("mgh", &short);
        assert_eq!(output, sandbox.ok("mgh", &long));
        assert_eq!(output, sandbox.ok("mgh", &help));
        assert!(output.contains("Usage:"));
    }
}

#[test]
fn removed_commands_and_invalid_arguments_are_rejected() {
    let sandbox = Sandbox::new();
    for command in [
        "new",
        "protect",
        "protections",
        "verbose",
        "welcome",
        "init",
        "check",
        "completions",
        "enter",
        "hook",
    ] {
        sandbox.blocked("mgh", &[command], "unrecognized subcommand");
    }
    sandbox.blocked(
        "mgh",
        &["repo", "autoswitch", "on"],
        "unrecognized subcommand",
    );

    for args in [
        vec!["switch", "personal", "--repo"],
        vec!["identity", "new", "--repo"],
        vec!["repo", "allowed", "update", "personal"],
        vec!["repo", "protections", "maybe"],
        vec!["settings", "autoswitch", "maybe"],
        vec!["settings", "verbose", "maybe"],
        vec!["settings", "welcome", "maybe"],
        vec!["shell", "init", "unknown"],
        vec!["shell", "completions", "unknown"],
        vec!["switch"],
        vec!["repo", "allowed", "add"],
        vec!["identity", "remove"],
        vec!["identity", "edit"],
        vec!["internal", "hook"],
        vec!["status", "--unknown"],
    ] {
        sandbox.blocked("mgh", &args, "error:");
    }
}
