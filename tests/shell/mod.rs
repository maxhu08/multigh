mod bash;
mod fish;
mod init;
mod zsh;

use crate::support::{Sandbox, quote, terminal};

fn configured_repositories() -> Sandbox {
    let sandbox = Sandbox::new();

    sandbox.protect();
    sandbox.commit();
    sandbox.ok("git", &["clone", ".", "../other"]);

    let output = sandbox
        .command("mgh")
        .current_dir(sandbox.path("other"))
        .args(["repo", "allowed", "add", "personal"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    sandbox
}

fn entry_reports(shell: &str, args: &[&str], script: &str) {
    let sandbox = configured_repositories();
    let script = script.replace("OTHER", &quote(sandbox.path("other").to_str().unwrap()));
    let (status, output) = terminal(sandbox.command(shell).args(args).arg(&script), &[]);

    assert!(status.success(), "{output}");
    assert_eq!(output.matches("Allowed identities").count(), 2, "{output}");

    sandbox.ok("mgh", &["settings", "verbose", "off"]);

    let (status, output) = terminal(sandbox.command(shell).args(args).arg(&script), &[]);

    assert!(status.success(), "{output}");
    assert!(!output.contains("Allowed identities"), "{output}");
}

fn startup_picker(shell: &str, args: &[&str], script: &str) {
    let sandbox = Sandbox::new();

    sandbox.ok("mgh", &["setup"]);
    sandbox.ok("mgh", &["settings", "verbose", "off"]);

    let (status, output) = terminal(
        sandbox.command(shell).args(args).arg(script),
        &[("Which identities may use this repository?", b" \r")],
    );

    assert!(status.success(), "{output}");
    assert_eq!(
        sandbox.ok("git", &["config", "--get-all", "mgh.allowed-identity"]),
        "personal\n"
    );

    sandbox.ok("mgh", &["repo", "check"]);
}

fn noninteractive(shell: &str, args: &[&str], script: &str) {
    let sandbox = configured_repositories();
    let script = script.replace("OTHER", &quote(sandbox.path("other").to_str().unwrap()));

    std::fs::remove_file(sandbox.path("gh-calls")).unwrap();

    let output = sandbox
        .command(shell)
        .args(args)
        .arg(script)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(!sandbox.path("gh-calls").exists());
}

fn autoswitch_on_startup_and_directory_entry(shell: &str, args: &[&str], script: &str) {
    let sandbox = configured_repositories();

    sandbox.ok("mgh", &["repo", "allowed", "add", "school"]);
    sandbox.ok("mgh", &["settings", "autoswitch", "on"]);
    sandbox.write("active", "bob\n");
    let script = script.replace("OTHER", &quote(sandbox.path("other").to_str().unwrap()));
    let (status, output) = terminal(
        sandbox.command(shell).args(args).arg(&script),
        &[("Which allowed identity should be active?", b"\r")],
    );

    assert!(status.success(), "{output}");
    assert!(output.contains("Which allowed identity should be active?"));
    assert_eq!(
        std::fs::read_to_string(sandbox.path("active")).unwrap(),
        "alice\n"
    );
    assert_eq!(
        sandbox.ok("git", &["config", "--get-all", "mgh.allowed-identity"]),
        "personal\nschool\n"
    );
}
