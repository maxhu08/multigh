use crate::support::{Sandbox, terminal};
use std::fs;

#[test]
fn identity_rules_are_idempotent_and_conflicts_preserve_configuration() {
    let sandbox = Sandbox::new();

    sandbox.ok("mgh", &["setup"]);

    let before = fs::read_to_string(sandbox.path("gitconfig")).unwrap();

    sandbox.ok("mgh", &["setup"]);

    assert_eq!(
        before,
        fs::read_to_string(sandbox.path("gitconfig")).unwrap()
    );

    for url in [
        "https://github.com/alice/repo.git",
        "git@github.com:alice/repo.git",
        "ssh://git@github.com/alice/repo.git",
    ] {
        sandbox.ok("git", &["remote", "set-url", "origin", url]);

        assert_eq!(
            sandbox.ok("git", &["config", "user.email"]).trim(),
            "alice@example.com"
        );
    }

    sandbox.ok(
        "git",
        &[
            "remote",
            "set-url",
            "origin",
            "https://github.com/unconfigured/project.git",
        ],
    );

    assert_eq!(
        sandbox.ok("git", &["config", "user.email"]).trim(),
        "bob@example.edu"
    );

    sandbox.ok(
        "git",
        &[
            "remote",
            "set-url",
            "origin",
            "https://github.com/alice/repo.git",
        ],
    );

    sandbox.ok("mgh", &["switch", "school"]);

    assert_eq!(
        sandbox
            .ok("git", &["config", "--global", "user.email"])
            .trim(),
        "bob@example.edu"
    );
    assert_eq!(
        sandbox.ok("git", &["config", "user.email"]).trim(),
        "alice@example.com"
    );

    sandbox.ok(
        "git",
        &[
            "config",
            "--global",
            "--add",
            "includeif.hasconfig:remote.*.url:https://github.com/alice/**.path",
            "/another/identity.conf",
        ],
    );

    let before = fs::read_to_string(sandbox.path("gitconfig")).unwrap();

    sandbox.blocked("mgh", &["setup"], "conflicts");

    assert_eq!(
        before,
        fs::read_to_string(sandbox.path("gitconfig")).unwrap()
    );
}

#[test]
fn setup_preserves_identity_configuration_enables_modes_and_reports_private_identity_file_locations()
 {
    use std::os::unix::fs::PermissionsExt;

    let sandbox = Sandbox::new();

    let configuration = fs::read(sandbox.path("config/multigh/identities.jsonc")).unwrap();
    let setup = sandbox.ok("mgh", &["setup"]);

    assert!(
        setup.contains(
            sandbox
                .path("config/multigh/identities.jsonc")
                .to_str()
                .unwrap()
        )
    );

    for identity_name in ["personal", "school"] {
        let identity = sandbox.path(&format!(
            "state/multigh/identities/git-{identity_name}.conf"
        ));

        assert!(setup.contains(identity.to_str().unwrap()));
        assert_eq!(
            fs::metadata(identity).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    assert!(sandbox.path("state/multigh/verbose-enabled").is_file());

    for directory in [
        "state/multigh",
        "state/multigh/identities",
        "state/multigh/hooks",
    ] {
        assert_eq!(
            fs::metadata(sandbox.path(directory))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
    }

    assert!(setup.contains("Block commits and pushes") && setup.contains("starting a terminal"));
    assert_eq!(
        configuration,
        fs::read(sandbox.path("config/multigh/identities.jsonc")).unwrap()
    );
    assert!(!sandbox.path("gh-calls").exists());
}

#[test]
fn setup_refreshes_changed_identities_and_removes_obsolete_include_rules() {
    let sandbox = Sandbox::new();

    sandbox.ok("mgh", &["setup"]);
    sandbox.write(
        "config/multigh/identities.jsonc",
        r#"{"personal": {"username": "alice", "commit": {"email": "new@example.com", "name": "New Name"}}}"#,
    );
    sandbox.ok("mgh", &["setup"]);

    assert_eq!(
        sandbox.ok("git", &["config", "user.email"]).trim(),
        "new@example.com"
    );
    assert_eq!(
        sandbox.ok("git", &["config", "user.name"]).trim(),
        "New Name"
    );

    let rules = sandbox.ok("git", &["config", "--global", "--list"]);

    assert!(rules.contains("alice/**"));
    assert!(!rules.contains("bob/**"));
}

#[test]
fn setup_preserves_symlink_targets_and_unrecognized_managed_directory_files() {
    let sandbox = Sandbox::new();

    sandbox.write("outside.conf", "preserve this file\n");
    fs::create_dir_all(sandbox.path("state/multigh/identities")).unwrap();
    std::os::unix::fs::symlink(
        sandbox.path("outside.conf"),
        sandbox.path("state/multigh/identities/git-personal.conf"),
    )
    .unwrap();

    let before = fs::read(sandbox.path("gitconfig")).unwrap();

    sandbox.blocked("mgh", &["setup"], "Identity file is a symlink");

    assert_eq!(before, fs::read(sandbox.path("gitconfig")).unwrap());
    assert_eq!(
        fs::read_to_string(sandbox.path("outside.conf")).unwrap(),
        "preserve this file\n"
    );

    fs::remove_file(sandbox.path("state/multigh/identities/git-personal.conf")).unwrap();
    sandbox.write("state/multigh/hooks/pre-commit", "unrecognized hook\n");
    sandbox.blocked("mgh", &["setup"], "Existing hook was left untouched");

    assert_eq!(
        fs::read_to_string(sandbox.path("state/multigh/hooks/pre-commit")).unwrap(),
        "unrecognized hook\n"
    );

    fs::remove_file(sandbox.path("state/multigh/hooks/pre-commit")).unwrap();
    std::os::unix::fs::symlink(
        sandbox.path("outside.conf"),
        sandbox.path("state/multigh/hooks/pre-commit"),
    )
    .unwrap();

    assert!(!sandbox.run("mgh", &["setup"]).status.success());
    assert_eq!(
        fs::read_to_string(sandbox.path("outside.conf")).unwrap(),
        "preserve this file\n"
    );
}

#[test]
fn setup_displays_the_global_file_selected_by_git_for_home_and_xdg_locations() {
    for (home, xdg) in [(false, false), (true, false), (false, true), (true, true)] {
        let sandbox = Sandbox::new();
        let original = fs::read_to_string(sandbox.path("gitconfig")).unwrap();

        if home {
            sandbox.write("home/.gitconfig", &original);
        }

        if xdg {
            sandbox.write("config/git/config", &original);
        }

        let output = sandbox
            .command("mgh")
            .env_remove("GIT_CONFIG_GLOBAL")
            .arg("setup")
            .output()
            .unwrap();
        let expected = sandbox.path(if !home && xdg {
            "config/git/config"
        } else {
            "home/.gitconfig"
        });

        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = String::from_utf8(output.stdout).unwrap();

        assert!(
            text.lines().any(|line| {
                line.trim_start()
                    .strip_prefix("Git config")
                    .is_some_and(|value| value.trim() == expected.to_string_lossy())
            }),
            "{text}"
        );
        assert!(
            fs::read_to_string(expected)
                .unwrap()
                .contains("# Generated by multigh (mgh setup)")
        );
        assert_eq!(
            original,
            fs::read_to_string(sandbox.path("gitconfig")).unwrap()
        );
    }
}

#[test]
fn onboarding_collects_a_first_identity_and_prints_next_steps() {
    let sandbox = Sandbox::new();
    fs::remove_file(sandbox.path("config/multigh/identities.jsonc")).unwrap();
    let (status, output) = terminal(
        sandbox.command("mgh").arg("setup"),
        &[
            ("Identity name: ", b"personal\r"),
            ("GitHub username: ", b"alice\r"),
            ("Commit name (default: alice): ", b"\r"),
            ("Commit email: ", b"alice@example.com\r"),
        ],
    );
    assert!(status.success(), "{output}");
    assert!(output.contains("Add more identities: mgh identity new"));
    assert!(output.contains("mgh shell init fish"));
    assert!(sandbox.path("state/multigh/hooks/pre-commit").is_file());
    let config: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(sandbox.path("config/multigh/identities.jsonc")).unwrap(),
    )
    .unwrap();
    assert_eq!(config["personal"]["username"], "alice");
}

#[test]
fn empty_identity_configuration_requires_onboarding_without_being_rejected_as_malformed() {
    let sandbox = Sandbox::new();
    let configuration = "// No identities yet\n{}\n";
    sandbox.write("config/multigh/identities.jsonc", configuration);
    let global = fs::read(sandbox.path("gitconfig")).unwrap();
    let local = fs::read(sandbox.path("repo/.git/config")).unwrap();

    sandbox.blocked("mgh", &["setup"], "Setup needs your first identity");

    assert_eq!(
        fs::read_to_string(sandbox.path("config/multigh/identities.jsonc")).unwrap(),
        configuration
    );
    assert_eq!(fs::read(sandbox.path("gitconfig")).unwrap(), global);
    assert_eq!(fs::read(sandbox.path("repo/.git/config")).unwrap(), local);
    assert!(!sandbox.path("state/multigh").exists());
    assert!(!sandbox.path("gh-calls").exists());
}
