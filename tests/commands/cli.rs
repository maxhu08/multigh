use crate::support::Sandbox;

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
