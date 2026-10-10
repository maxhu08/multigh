use crate::support::Sandbox;

#[test]
fn protections_and_verbose_toggle_independently() {
    let sandbox = Sandbox::new();

    sandbox.protect();

    assert!(sandbox.ok("mgh", &["internal", "enter"]).contains("alice"));

    sandbox.ok("mgh", &["settings", "verbose", "off"]);

    assert!(sandbox.ok("mgh", &["internal", "enter"]).is_empty());

    sandbox.write("active", "bob\n");
    sandbox.blocked(
        "git",
        &["commit", "--allow-empty", "-m", "Blocked"],
        "not allowed",
    );
    sandbox.ok("mgh", &["repo", "protections", "off"]);
    sandbox.commit();

    assert_eq!(
        sandbox
            .ok("git", &["config", "mgh.allowed-identity"])
            .trim(),
        "personal"
    );

    sandbox.ok("mgh", &["settings", "verbose", "on"]);

    assert!(sandbox.ok("mgh", &["internal", "enter"]).contains("alice"));

    sandbox.ok("mgh", &["setup"]);
    sandbox.commit();
    sandbox.ok("mgh", &["repo", "protections", "on"]);
    sandbox.blocked(
        "git",
        &["commit", "--allow-empty", "-m", "Re-enabled"],
        "not allowed",
    );
}

#[test]
fn verbose_preferences_do_not_require_config_or_enable_protections() {
    let sandbox = Sandbox::new();

    sandbox.ok("mgh", &["repo", "protections", "off"]);
    let local = std::fs::read(sandbox.path("repo/.git/config")).unwrap();
    std::fs::remove_file(sandbox.path("config/multigh/identities.jsonc")).unwrap();

    assert!(sandbox.ok("mgh", &["settings", "verbose"]).contains("OFF"));
    assert!(
        sandbox
            .ok("mgh", &["settings", "verbose", "on"])
            .contains("ON")
    );
    assert!(sandbox.path("state/multigh/verbose-enabled").is_file());
    assert_eq!(
        std::fs::read(sandbox.path("repo/.git/config")).unwrap(),
        local
    );

    sandbox.ok("mgh", &["settings", "verbose", "off"]);
    sandbox.ok("mgh", &["settings", "verbose", "off"]);

    assert!(sandbox.ok("mgh", &["settings", "verbose"]).contains("OFF"));
    assert_eq!(
        std::fs::read(sandbox.path("repo/.git/config")).unwrap(),
        local
    );
    assert!(!sandbox.path("gh-calls").exists());
}

#[test]
fn entry_outside_git_is_silent_with_both_modes_enabled() {
    let sandbox = Sandbox::new();

    sandbox.protect();

    let result = sandbox
        .command("mgh")
        .current_dir(sandbox.path(""))
        .args(["internal", "enter"])
        .output()
        .unwrap();

    assert!(result.status.success() && result.stdout.is_empty());
}
