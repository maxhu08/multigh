use crate::support::{Sandbox, terminal};
use std::fs;

#[test]
fn autoswitch_is_global_switches_single_and_always_prompts_for_multiple() {
    let sandbox = Sandbox::new();
    sandbox.protect();
    sandbox.write("active", "bob\n");
    sandbox.ok("mgh", &["settings", "autoswitch", "on"]);
    assert_eq!(
        fs::read_to_string(sandbox.path("active")).unwrap(),
        "alice\n"
    );
    sandbox.ok("mgh", &["repo", "allowed", "add", "school"]);
    for keys in [b"\x1b[B\r".as_slice(), b"\r".as_slice()] {
        let (status, output) = terminal(
            sandbox.command("mgh").args(["internal", "enter"]),
            &[("Which allowed identity should be active?", keys)],
        );
        assert!(status.success(), "{output}");
        assert_eq!(fs::read_to_string(sandbox.path("active")).unwrap(), "bob\n");
    }
    let local = fs::read(sandbox.path("repo/.git/config")).unwrap();
    let global = fs::read(sandbox.path("gitconfig")).unwrap();
    let (_, _) = terminal(
        sandbox.command("mgh").args(["internal", "enter"]),
        &[("Which allowed identity should be active?", b"\x1b")],
    );
    assert_eq!(fs::read(sandbox.path("repo/.git/config")).unwrap(), local);
    assert_eq!(fs::read(sandbox.path("gitconfig")).unwrap(), global);
    assert!(
        sandbox
            .ok("mgh", &["internal", "enter"])
            .contains("interactive terminal")
    );
    sandbox.ok("mgh", &["settings", "autoswitch", "off"]);
    sandbox.write("active", "outsider\n");
    sandbox.blocked(
        "git",
        &["commit", "--allow-empty", "-m", "Blocked"],
        "not allowed",
    );
    assert_eq!(
        fs::read_to_string(sandbox.path("active")).unwrap(),
        "outsider\n"
    );
}

#[test]
fn failed_autoswitch_keeps_authentication_and_permissions_and_hooks_do_not_switch() {
    let sandbox = Sandbox::new();
    sandbox.protect();
    sandbox.write("active", "bob\n");
    sandbox.write("fail-switch", "");
    let local = fs::read(sandbox.path("repo/.git/config")).unwrap();
    let global = fs::read(sandbox.path("gitconfig")).unwrap();
    let output = sandbox.ok("mgh", &["settings", "autoswitch", "on"]);

    assert!(output.contains("Unable to switch GitHub account"));
    assert_eq!(fs::read_to_string(sandbox.path("active")).unwrap(), "bob\n");
    assert_eq!(fs::read(sandbox.path("gitconfig")).unwrap(), global);
    assert_eq!(
        sandbox.ok("git", &["config", "--get-all", "mgh.allowed-identity"]),
        "personal\n"
    );
    sandbox.blocked(
        "git",
        &["commit", "--allow-empty", "-m", "Blocked"],
        "not allowed",
    );
    sandbox.ok("mgh", &["settings", "autoswitch", "off"]);
    assert_eq!(fs::read(sandbox.path("repo/.git/config")).unwrap(), local);
}

#[test]
fn invalid_repository_boolean_settings_fail_closed() {
    let sandbox = Sandbox::new();
    sandbox.protect();
    sandbox.ok("git", &["config", "--local", "mgh.protections", "invalid"]);
    sandbox.blocked("mgh", &["repo", "protections"], "bad boolean");
    sandbox.blocked(
        "git",
        &["commit", "--allow-empty", "-m", "Blocked"],
        "bad boolean",
    );
    sandbox.ok("mgh", &["repo", "protections", "on"]);
}

#[test]
fn global_autoswitch_applies_to_other_repositories_and_preserves_local_policy() {
    let sandbox = Sandbox::new();
    sandbox.protect();
    sandbox.commit();
    sandbox.ok("git", &["clone", ".", "../other"]);
    let allowed = sandbox
        .command("mgh")
        .current_dir(sandbox.path("other"))
        .args(["repo", "allowed", "add", "school"])
        .output()
        .unwrap();
    assert!(allowed.status.success());
    sandbox.ok("git", &["config", "--local", "mgh.autoswitch", "invalid"]);
    let local = fs::read(sandbox.path("repo/.git/config")).unwrap();
    sandbox.ok("mgh", &["settings", "autoswitch", "on"]);
    assert!(sandbox.path("state/multigh/autoswitch-enabled").is_file());
    sandbox.ok("mgh", &["setup"]);
    assert!(
        sandbox
            .ok("mgh", &["settings", "autoswitch"])
            .contains("ON")
    );
    let result = sandbox
        .command("mgh")
        .current_dir(sandbox.path("other"))
        .args(["internal", "enter"])
        .output()
        .unwrap();

    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(fs::read_to_string(sandbox.path("active")).unwrap(), "bob\n");
    assert_eq!(fs::read(sandbox.path("repo/.git/config")).unwrap(), local);
    sandbox.ok("mgh", &["settings", "autoswitch", "off"]);
    sandbox.ok("mgh", &["internal", "enter"]);
    assert_eq!(fs::read_to_string(sandbox.path("active")).unwrap(), "bob\n");
    assert!(!sandbox.path("state/multigh/autoswitch-enabled").exists());
}

#[test]
fn autoswitch_preferences_can_be_changed_outside_git_without_identity_configuration() {
    let sandbox = Sandbox::new();
    fs::remove_file(sandbox.path("config/multigh/identities.jsonc")).unwrap();
    let global = fs::read(sandbox.path("gitconfig")).unwrap();

    for state in ["on", "off"] {
        let result = sandbox
            .command("mgh")
            .current_dir(sandbox.path("home"))
            .args(["settings", "autoswitch", state])
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }

    let all = sandbox.ok("mgh", &["settings"]);
    assert!(all.contains("Autoswitch") && all.contains("Verbose") && all.contains("Welcome"));
    assert_eq!(fs::read(sandbox.path("gitconfig")).unwrap(), global);
    assert!(!sandbox.path("gh-calls").exists());
}

#[test]
fn selecting_permissions_in_a_new_repo_then_autoswitching_uses_only_selected_identities() {
    let sandbox = Sandbox::new();
    sandbox.ok("mgh", &["setup"]);
    let enable = sandbox
        .command("mgh")
        .current_dir(sandbox.path("home"))
        .args(["settings", "autoswitch", "on"])
        .output()
        .unwrap();
    assert!(enable.status.success());
    sandbox.write("active", "bob\n");
    let (status, output) = terminal(
        sandbox.command("mgh").args(["internal", "enter"]),
        &[("Which identities may use this repository?", b" \r")],
    );

    assert!(status.success(), "{output}");
    assert_eq!(
        sandbox.ok("git", &["config", "--get-all", "mgh.allowed-identity"]),
        "personal\n"
    );
    assert_eq!(
        fs::read_to_string(sandbox.path("active")).unwrap(),
        "alice\n"
    );
    sandbox.ok("mgh", &["repo", "check"]);
}

#[test]
fn sole_active_identity_does_not_switch_again_or_rewrite_configuration() {
    let sandbox = Sandbox::new();
    sandbox.protect();
    sandbox.write("active", "bob\n");
    sandbox.ok("mgh", &["settings", "autoswitch", "on"]);
    assert_eq!(
        fs::read_to_string(sandbox.path("active")).unwrap(),
        "alice\n"
    );
    fs::remove_file(sandbox.path("gh-calls")).unwrap();
    let before = fs::read(sandbox.path("gitconfig")).unwrap();
    sandbox.ok("mgh", &["internal", "enter"]);
    assert!(
        !fs::read_to_string(sandbox.path("gh-calls"))
            .unwrap()
            .contains("auth switch")
    );
    assert_eq!(fs::read(sandbox.path("gitconfig")).unwrap(), before);
}
