use crate::support::Sandbox;
use std::{fs, os::unix::fs::PermissionsExt};

#[test]
fn shared_hook_aliases_do_not_chain_back_into_themselves() {
    let sandbox = Sandbox::new();

    sandbox.protect();
    std::os::unix::fs::symlink(
        sandbox.path("state/multigh/hooks"),
        sandbox.path("repo/shared-alias"),
    )
    .unwrap();
    sandbox.ok(
        "git",
        &["config", "--local", "core.hooksPath", "shared-alias"],
    );
    sandbox.ok("mgh", &["enter"]);

    assert!(
        !sandbox
            .run(
                "git",
                &["config", "--local", "--get", "mgh.originalHooksPath"]
            )
            .status
            .success()
    );

    sandbox.commit();
    sandbox.ok(
        "git",
        &["config", "--local", "mgh.originalHooksPath", "shared-alias"],
    );
    sandbox.blocked(
        "git",
        &["commit", "--allow-empty", "-m", "Recursion"],
        "Original hooks path points back",
    );
}

#[test]
fn missing_changed_or_non_executable_hooks_do_not_report_active_protections() {
    let sandbox = Sandbox::new();

    sandbox.executable("repo/.git/hooks/pre-commit", "#!/bin/sh\nexit 0\n");
    sandbox.protect();

    let shared = sandbox.path("state/multigh/hooks");

    fs::remove_file(shared.join("pre-commit")).unwrap();
    fs::set_permissions(shared.join("pre-push"), fs::Permissions::from_mode(0o644)).unwrap();

    let entry = sandbox.ok("mgh", &["enter"]);

    assert!(entry.contains("Missing mgh hook: pre-commit"));
    assert!(entry.contains("mgh hook is not executable: pre-push"));
    assert!(entry.contains("Account protections are not active."));
    assert!(!entry.contains("Account protections are active;"));

    let status = sandbox.ok("mgh", &["status"]);

    assert!(status.contains("Missing mgh hook: pre-commit"));
    assert!(!status.contains("Commit + push checks enabled"));

    sandbox.ok("mgh", &["setup"]);

    assert!(
        sandbox
            .ok("mgh", &["enter"])
            .contains("Account protections are active;")
    );

    sandbox.commit();
    fs::write(shared.join("pre-push"), "#!/bin/sh\nexit 0\n").unwrap();

    let entry = sandbox.ok("mgh", &["enter"]);

    assert!(entry.contains("mgh hook is unreadable or changed: pre-push"));
    assert!(!entry.contains("Account protections are active;"));
}

#[test]
fn global_and_command_hook_overrides_are_reported_without_false_confirmation() {
    let sandbox = Sandbox::new();

    sandbox.protect();
    fs::create_dir_all(sandbox.path("repo/custom")).unwrap();
    sandbox.executable("repo/custom/pre-commit", "#!/bin/sh\nexit 0\n");
    sandbox.ok("git", &["config", "--global", "core.hooksPath", "custom"]);

    let entry = sandbox.ok("mgh", &["enter"]);

    assert!(entry.contains("global hook path overrides mgh: custom"));
    assert!(entry.contains("custom/pre-commit"));
    assert!(!entry.contains("Account protections are active;"));
    assert_eq!(
        sandbox
            .ok("git", &["config", "--global", "core.hooksPath"])
            .trim(),
        "custom"
    );

    sandbox.ok(
        "git",
        &[
            "config",
            "--global",
            "core.hooksPath",
            sandbox.path("state/multigh/hooks").to_str().unwrap(),
        ],
    );

    let output = sandbox
        .command("mgh")
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "core.hooksPath")
        .env("GIT_CONFIG_VALUE_0", "custom")
        .arg("enter")
        .output()
        .unwrap();

    assert!(output.status.success());

    let entry = String::from_utf8_lossy(&output.stdout);

    assert!(entry.contains("command hook path overrides mgh: custom"));
    assert!(!entry.contains("Account protections are active;"));
    assert!(
        !sandbox
            .run("git", &["config", "--local", "--get", "core.hooksPath"])
            .status
            .success()
    );
}

#[test]
fn integration_rechecks_effective_settings_after_writing_git_config() {
    let sandbox = Sandbox::new();

    sandbox.protect();
    fs::create_dir_all(sandbox.path("repo/custom")).unwrap();
    sandbox.executable("repo/custom/pre-commit", "#!/bin/sh\nexit 0\n");
    sandbox.ok("git", &["config", "--local", "core.hooksPath", "custom"]);
    sandbox.write("repo/override.conf", "[core]\nhooksPath = custom\n");
    sandbox.ok(
        "git",
        &["config", "--local", "include.path", "../override.conf"],
    );

    let entry = sandbox.ok("mgh", &["enter"]);

    assert!(entry.contains("local hook path overrides mgh: custom"));
    assert!(!entry.contains("Account protections are active;"));
    assert_eq!(
        sandbox
            .ok("git", &["config", "--get", "core.hooksPath"])
            .trim(),
        "custom"
    );
    assert_eq!(
        sandbox
            .ok("git", &["config", "--local", "--get", "core.hooksPath"])
            .trim(),
        sandbox.path("state/multigh/hooks").to_str().unwrap()
    );
}
