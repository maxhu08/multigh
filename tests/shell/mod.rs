mod bash;
mod fish;
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
        .args(["protections", "--allow", "personal"])
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
    assert_eq!(output.matches("Allowed accounts").count(), 2, "{output}");

    sandbox.ok("mgh", &["verbose", "off"]);

    let (status, output) = terminal(sandbox.command(shell).args(args).arg(&script), &[]);

    assert!(status.success(), "{output}");
    assert!(!output.contains("Allowed accounts"), "{output}");
}

fn startup_picker(shell: &str, args: &[&str], script: &str) {
    let sandbox = Sandbox::new();

    sandbox.ok("mgh", &["setup"]);
    sandbox.ok("mgh", &["verbose", "off"]);

    let (status, output) = terminal(
        sandbox.command(shell).args(args).arg(script),
        &[("Which accounts may use this repository?", b" \r")],
    );

    assert!(status.success(), "{output}");
    assert_eq!(
        sandbox.ok("git", &["config", "--get-all", "mgh.allowedAccount"]),
        "personal\n"
    );

    sandbox.ok("mgh", &["check"]);
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
