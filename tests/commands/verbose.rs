use crate::support::Sandbox;

#[test]
fn protections_and_verbose_toggle_independently() {
    let sandbox = Sandbox::new();

    sandbox.protect();

    assert!(sandbox.ok("mgh", &["enter"]).contains("alice"));

    sandbox.ok("mgh", &["verbose", "off"]);

    assert!(sandbox.ok("mgh", &["enter"]).is_empty());

    sandbox.write("active", "bob\n");
    sandbox.blocked(
        "git",
        &["commit", "--allow-empty", "-m", "Blocked"],
        "not allowed",
    );
    sandbox.ok("mgh", &["protections", "off"]);
    sandbox.commit();

    assert_eq!(
        sandbox.ok("git", &["config", "mgh.allowedAccount"]).trim(),
        "personal"
    );

    sandbox.ok("mgh", &["verbose", "on"]);

    assert!(sandbox.ok("mgh", &["enter"]).contains("alice"));

    sandbox.ok("mgh", &["setup"]);
    sandbox.blocked(
        "git",
        &["commit", "--allow-empty", "-m", "Re-enabled"],
        "not allowed",
    );
}

#[test]
fn verbose_preferences_do_not_require_config_or_enable_protections() {
    let sandbox = Sandbox::new();

    std::fs::remove_file(sandbox.path("config/multigh/accounts.conf")).unwrap();

    assert!(sandbox.ok("mgh", &["verbose"]).contains("OFF"));
    assert!(sandbox.ok("mgh", &["verbose", "on"]).contains("ON"));
    assert!(sandbox.path("state/multigh/verbose-enabled").is_file());
    assert!(!sandbox.path("state/multigh/protections-enabled").exists());

    sandbox.ok("mgh", &["verbose", "off"]);
    sandbox.ok("mgh", &["verbose", "off"]);

    assert!(sandbox.ok("mgh", &["verbose"]).contains("OFF"));
    assert!(!sandbox.path("gh-calls").exists());
}

#[test]
fn entry_outside_git_is_silent_with_both_modes_enabled() {
    let sandbox = Sandbox::new();

    sandbox.protect();

    let result = sandbox
        .command("mgh")
        .current_dir(sandbox.path(""))
        .arg("enter")
        .output()
        .unwrap();

    assert!(result.status.success() && result.stdout.is_empty());
}
