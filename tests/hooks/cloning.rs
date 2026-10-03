use crate::support::Sandbox;

#[test]
fn setup_enables_modes_and_unconfigured_clones_stay_blocked() {
    let sandbox = Sandbox::new();

    let setup = sandbox.ok("mgh", &["setup"]);

    assert!(setup.contains("Protections") && setup.contains("Verbose") && setup.contains("ON"));

    sandbox.blocked(
        "git",
        &["commit", "--allow-empty", "-m", "Unconfigured"],
        "No identities are authorized",
    );
    sandbox.ok("mgh", &["protections", "--allow", "personal"]);
    sandbox.commit();

    for (name, flags) in [
        ("clone", vec![]),
        ("no-checkout", vec!["--no-checkout"]),
        ("bare", vec!["--bare"]),
    ] {
        let destination = format!("../{name}");
        let mut args = vec!["clone"];

        args.extend(flags);
        args.extend([".", &destination]);
        sandbox.ok("git", &args);

        let result = sandbox
            .command("git")
            .current_dir(sandbox.path(name))
            .args(["push", "origin", "HEAD:refs/heads/new"])
            .output()
            .unwrap();

        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("No identities are authorized"));
    }
}

#[test]
fn empty_clones_remain_blocked_until_identities_are_selected() {
    let sandbox = Sandbox::new();

    sandbox.ok("mgh", &["setup"]);
    sandbox.ok("git", &["clone", ".", "../empty"]);

    let output = sandbox
        .command("git")
        .current_dir(sandbox.path("empty"))
        .args([
            "commit",
            "--allow-empty",
            "-m",
            "Unauthorized initial commit",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("No identities are authorized"));

    let output = sandbox
        .command("mgh")
        .current_dir(sandbox.path("empty"))
        .args(["protections", "--allow", "personal"])
        .output()
        .unwrap();

    assert!(output.status.success());

    let output = sandbox
        .command("git")
        .current_dir(sandbox.path("empty"))
        .args(["commit", "--allow-empty", "-m", "Authorized initial commit"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
