use crate::support::{IDENTITIES, Sandbox};
use std::fs;

#[test]
fn explicit_absolute_and_relative_config_paths_override_the_default() {
    let sandbox = Sandbox::new();

    sandbox.write(
        "repo/other.conf",
        &IDENTITIES.replace("alice@example.com", "different@example.com"),
    );
    fs::remove_file(sandbox.path("config/multigh/identities.jsonc")).unwrap();
    sandbox.ok("mgh", &["--config", "other.conf", "setup"]);

    assert_eq!(
        sandbox.ok("git", &["config", "user.email"]).trim(),
        "different@example.com"
    );

    sandbox.ok(
        "mgh",
        &[
            "protections",
            "--config",
            sandbox.path("repo/other.conf").to_str().unwrap(),
            "--allow",
            "personal",
        ],
    );
    sandbox.commit();

    let output = sandbox.ok("mgh", &["setup", "--config", "other.conf"]);

    assert!(output.contains(sandbox.path("repo/other.conf").to_str().unwrap()));
}

#[test]
fn config_and_state_fallbacks_use_an_isolated_userprofile_when_xdg_and_home_are_absent() {
    let sandbox = Sandbox::new();

    sandbox.write("home/.config/multigh/identities.jsonc", IDENTITIES);

    let output = sandbox
        .command("mgh")
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("XDG_STATE_HOME")
        .env_remove("HOME")
        .env("USERPROFILE", sandbox.path("home"))
        .arg("setup")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("home/.config/multigh/identities.jsonc")
    );
    assert!(
        sandbox
            .path("home/.local/state/multigh/identities/git-personal.conf")
            .is_file()
    );
    assert!(!sandbox.path("state/multigh").exists());
}

#[test]
fn absent_config_paths_are_reported_and_config_independent_commands_need_no_home() {
    let sandbox = Sandbox::new();

    sandbox.blocked("mgh", &["--config", "missing.conf", "setup"], "Read ");

    let result = sandbox
        .command("mgh")
        .env_remove("HOME")
        .env_remove("USERPROFILE")
        .env_remove("XDG_CONFIG_HOME")
        .arg("setup")
        .output()
        .unwrap();

    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("Cannot locate your home directory"));

    for args in [
        vec!["init", "fish"],
        vec!["completions", "bash"],
        vec!["--help"],
    ] {
        let result = sandbox
            .command("mgh")
            .env_remove("HOME")
            .env_remove("USERPROFILE")
            .env_remove("XDG_CONFIG_HOME")
            .args(args)
            .output()
            .unwrap();

        assert!(result.status.success());
    }
}

#[test]
fn explicit_config_needs_no_default_home_or_config_directory() {
    let sandbox = Sandbox::new();

    let output = sandbox
        .command("mgh")
        .env_remove("HOME")
        .env_remove("USERPROFILE")
        .env_remove("XDG_CONFIG_HOME")
        .args([
            "--config",
            sandbox
                .path("config/multigh/identities.jsonc")
                .to_str()
                .unwrap(),
            "setup",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
