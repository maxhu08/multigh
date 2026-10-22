//! Identity schemas, paths and generated Git rules, including preservation of existing files.

use super::support::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::fs::symlink;

/// The documented example must load with its defaults without writing config, state or
/// authentication.
#[test]
fn example_configuration_loads_without_changing_state() {
    let env = Sandbox::new();
    fs::write(&env.config, include_str!("../../examples/identities.jsonc")).unwrap();
    let config_before = bytes(&env.config);
    let auth_before = bytes(env.root.join("auth.json"));
    let text = out(&env.success(&["identity"]));
    for expected in [
        "personal (Active)",
        "school (Not signed in)",
        "work (Not signed in)",
        "alice@example.com",
        "bob@example.edu",
        "carol@example.org",
    ] {
        assert!(text.contains(expected), "missing {expected:?}: {text}");
    }
    let school = text.split("school (Not signed in)").nth(1).unwrap();
    let school = school.split("work (Not signed in)").next().unwrap();
    assert!(
        school
            .lines()
            .any(|line| line.trim_start().starts_with("Commit name") && line.ends_with("bob"))
    );
    assert_eq!(bytes(&env.config), config_before);
    assert_eq!(bytes(env.root.join("auth.json")), auth_before);
    assert!(!env.home.join(".gitconfig").exists());
    assert!(!env.state.exists());
}

/// Invalid identity definitions and field values must fail before login or saved changes.
#[test]
fn configuration_validation_precedes_authentication_and_mutation() {
    let env = Sandbox::new();
    for (case, text) in [
        ("comment-only file", "// only comments"),
        ("array root", "[]"),
        ("null root", "null"),
        ("malformed JSON", "{"),
        (
            "duplicate username field",
            r#"{"x":{"username":"alice","username":"bob","commit":{"email":"x@y"}}}"#,
        ),
        (
            "unknown commit field",
            r#"{"x":{"username":"alice","commit":{"email":"x@y","extra":1}}}"#,
        ),
        (
            "unknown identity field",
            r#"{"x":{"username":"alice","extra":1,"commit":{"email":"x@y"}}}"#,
        ),
        (
            "case-colliding identity names",
            r#"{"X":{"username":"alice","commit":{"email":"x@y"}},"x":{"username":"bob","commit":{"email":"b@y"}}}"#,
        ),
        (
            "invalid identity name",
            r#"{"1bad":{"username":"alice","commit":{"email":"x@y"}}}"#,
        ),
        (
            "invalid username",
            r#"{"x":{"username":"-bad","commit":{"email":"x@y"}}}"#,
        ),
        (
            "invalid commit name",
            r#"{"x":{"username":"alice","commit":{"name":"<bad>","email":"x@y"}}}"#,
        ),
        (
            "multiple at-signs in email",
            r#"{"x":{"username":"alice","commit":{"email":"a@@b"}}}"#,
        ),
        (
            "whitespace in email",
            r#"{"x":{"username":"alice","commit":{"email":"a b@c"}}}"#,
        ),
        (
            "shared username ignoring case",
            r#"{"a":{"username":"alice","commit":{"email":"x@y"}},"b":{"username":"ALICE","commit":{"email":"b@y"}}}"#,
        ),
        (
            "shared email ignoring case",
            r#"{"a":{"username":"alice","commit":{"email":"x@y"}},"b":{"username":"bob","commit":{"email":"X@Y"}}}"#,
        ),
    ] {
        fs::write(&env.config, text).unwrap();
        let before = bytes(&env.config);
        env.rejects(
            &[
                "identity",
                "new",
                "new",
                "--username",
                "new",
                "--email",
                "n@e",
            ],
            "invalid identity configuration",
        );
        assert_eq!(
            bytes(&env.config),
            before,
            "{case}: config must be preserved"
        );
    }
    env.write_identities();
    for (args, phrase) in [
        (
            vec![
                "identity",
                "new",
                "9bad",
                "--username",
                "bob",
                "--email",
                "b@e",
            ],
            "Invalid identity name",
        ),
        (
            vec![
                "identity",
                "new",
                "bob",
                "--username",
                "-bad",
                "--email",
                "b@e",
            ],
            "Invalid username",
        ),
        (
            vec![
                "identity",
                "new",
                "bob",
                "--username",
                "bob",
                "--email",
                "bad",
            ],
            "Invalid email",
        ),
        (
            vec![
                "identity",
                "new",
                "bob",
                "--username",
                "bob",
                "--email",
                "b@e",
                "--name",
                "a\nb",
            ],
            "single-line",
        ),
        (
            vec![
                "identity",
                "new",
                "PERSONAL",
                "--username",
                "bob",
                "--email",
                "b@e",
            ],
            "already exists",
        ),
    ] {
        let before = bytes(&env.config);
        env.rejects(&args, phrase);
        assert_eq!(bytes(&env.config), before);
    }
    assert!(!env.root.join("gh.log").exists());
    assert!(!env.home.join(".gitconfig").exists());
}

/// Comments and trailing commas are allowed; unsupported syntax and missing required fields are
/// not.
#[test]
fn jsonc_is_not_json5_and_required_fields_are_strict() {
    let env = Sandbox::new();
    for text in [
        r#"{personal:{"username":"alice","commit":{"email":"a@b"}}}"#,
        r#"{'personal':{'username':'alice','commit':{'email':'a@b'}}}"#,
        r#"{"p":{"username":"alice" "commit":{"email":"a@b"}}}"#,
        r#"{"p":{"username":"al\x69ce","commit":{"email":"a@b"}}}"#,
        r#"{"p":{"username":"alice","commit":{}}}"#,
        r#"{"p":{"commit":{"email":"a@b"}}}"#,
        r#"{"p":{"username":"alice","commit":{"email":"a@b","additional_emails":[1]}}}"#,
    ] {
        fs::write(&env.config, text).unwrap();
        env.rejects(&["identity", "list"], "invalid identity configuration");
    }
    fs::write(&env.config,r#"{"p":{"username":" alice ","commit":{"email":" X@Y ","additional_emails":["x@y"," X@Y "]}}}"#).unwrap();
    let o = env.success(&["identity"]);
    assert!(out(&o).contains("alice"));
    assert!(out(&o).contains("X@Y"));
}

/// Explicit paths and HOME/XDG fallbacks must select the intended identity file.
#[test]
fn home_userprofile_xdg_empty_and_relative_config_paths() {
    let env = Sandbox::new();
    let fallback = env.home.join(".config/multigh");
    fs::create_dir_all(&fallback).unwrap();
    fs::write(fallback.join("identities.jsonc"), IDENTITIES).unwrap();
    for remove_home in [false, true] {
        let mut c = env.mgh(&["identity"]);
        c.env("XDG_CONFIG_HOME", "");
        if remove_home {
            c.env_remove("HOME").env("USERPROFILE", &env.home);
        }
        let o = c.output().unwrap();
        assert_ok(&o);
        assert!(out(&o).contains("personal"));
    }
    let relative = env.repo.join("other ' ids.jsonc");
    fs::write(&relative, IDENTITIES).unwrap();
    let o = env
        .mgh(&["identity", "--config", "other ' ids.jsonc", "list"])
        .env_remove("HOME")
        .env_remove("USERPROFILE")
        .output()
        .unwrap();
    assert_ok(&o);
    let o = env
        .mgh(&["settings", "verbose", "on"])
        .env("XDG_STATE_HOME", "")
        .output()
        .unwrap();
    assert_ok(&o);
    assert!(
        env.home
            .join(".local/state/multigh/verbose-enabled")
            .exists()
    );
    let o = env
        .mgh(&[
            "identity",
            "new",
            "x",
            "--username",
            "alice",
            "--email",
            "a@b",
            "--config",
            "new/ids.jsonc",
        ])
        .output()
        .unwrap();
    assert_ok(&o);
    assert_eq!(
        fs::metadata(env.repo.join("new"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
}

/// Reading a symlink is allowed, but edits and generated files must leave its target intact.
#[test]
fn symlinks_never_replace_identity_or_managed_targets() {
    let env = Sandbox::new();
    let target = env.root.join("target.jsonc");
    fs::write(&target, IDENTITIES).unwrap();
    symlink(&target, &env.config).unwrap();
    let before = bytes(&target);
    env.success(&["identity", "list"]);
    env.rejects(
        &["identity", "edit", "personal", "--email", "n@e"],
        "symlink",
    );
    assert_eq!(bytes(&target), before);
    assert!(
        fs::symlink_metadata(&env.config)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    fs::remove_file(&env.config).unwrap();
    env.write_identities();
    let dir = env.state.join("identities");
    fs::create_dir_all(&dir).unwrap();
    let generated = dir.join("git-personal.conf");
    symlink(&target, &generated).unwrap();
    env.rejects(&["setup"], "symlink");
    assert_eq!(bytes(&target), before);
    fs::remove_file(generated).unwrap();
    let hooks = env.state.join("hooks");
    fs::create_dir_all(&hooks).unwrap();
    symlink(&target, hooks.join("pre-push")).unwrap();
    env.rejects(&["setup"], "symlink");
    assert_eq!(bytes(&target), before);
}

/// GitHub remotes select identity rules without overwriting foreign rules or rewriting
/// unchanged entries.
#[test]
fn global_rules_all_remote_formats_refresh_noop_and_conflicts() {
    let env = Sandbox::new();
    env.init_repo();
    env.setup_defaults();
    for remote in [
        "https://github.com/alice/project.git",
        "git@github.com:alice/project.git",
        "ssh://git@github.com/alice/project.git",
    ] {
        env.set_local("remote.origin.url", remote);
        assert_eq!(
            env.git_without_hooks(&["config", "user.name"]),
            "Alice Example"
        );
        assert_eq!(
            env.git_without_hooks(&["config", "user.email"]),
            "Alice@example.com"
        );
    }
    env.success(&["switch", "work"]);
    env.set_local(
        "remote.origin.url",
        "https://github.com/unconfigured/project.git",
    );
    assert_eq!(
        env.git_without_hooks(&["config", "user.email"]),
        "work@example.com"
    );
    let before = bytes(env.home.join(".gitconfig"));
    let identity = bytes(env.state.join("identities/git-personal.conf"));
    let o = env.success(&["setup"]);
    assert!(out(&o).contains("Global Git config unchanged"));
    assert_eq!(bytes(env.home.join(".gitconfig")), before);
    assert_eq!(
        bytes(env.state.join("identities/git-personal.conf")),
        identity
    );
    env.success(&["identity", "edit", "personal", "--email", "new@example.com"]);
    assert!(
        String::from_utf8(bytes(env.state.join("identities/git-personal.conf")))
            .unwrap()
            .contains("new@example.com")
    );
    env.git_without_hooks(&[
        "config",
        "--global",
        "includeif.hasconfig:remote.*.url:https://github.com/alice/**.path",
        "/foreign/file",
    ]);
    let before = bytes(env.home.join(".gitconfig"));
    env.rejects(&["setup"], "Conflicting identity rule");
    assert_eq!(bytes(env.home.join(".gitconfig")), before);
}

/// Global writes must respect custom Git config locations and preserve symlinks.
#[test]
fn global_symlink_and_custom_location_preservation() {
    let env = Sandbox::new();
    let target = env.root.join("selected-global");
    fs::write(&target, "# original\n[foreign]\n value = unchanged\n").unwrap();
    let path = env.home.join(".gitconfig");
    symlink(&target, &path).unwrap();
    env.setup_defaults();
    assert!(
        fs::symlink_metadata(&path)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    let text = fs::read_to_string(&target).unwrap();
    assert!(text.contains("# original"));
    assert!(text.contains("Generated by multigh (mgh setup)"));
    let before = bytes(&target);
    env.success(&["setup"]);
    assert_eq!(bytes(&target), before);
}

/// Git chooses the global config path, including HOME/XDG precedence and symlink targets.
#[test]
fn git_default_global_locations_and_symlinks_are_selected_by_git() {
    let env = Sandbox::new();
    env.write_identities();
    let xdg = env.root.join("config/git");
    fs::create_dir_all(&xdg).unwrap();
    let actual = xdg.join("config");
    fs::write(&actual, "[foreign]\n value = preserved\n").unwrap();
    let o = env
        .mgh(&["setup"])
        .env_remove("GIT_CONFIG_GLOBAL")
        .output()
        .unwrap();
    assert_ok(&o);
    assert!(out(&o).contains(actual.to_str().unwrap()));
    assert!(!env.home.join(".gitconfig").exists());
    assert!(
        fs::read_to_string(&actual)
            .unwrap()
            .contains("value = preserved")
    );
    let before = bytes(&actual);
    let o = env
        .mgh(&["setup"])
        .env_remove("GIT_CONFIG_GLOBAL")
        .output()
        .unwrap();
    assert_ok(&o);
    assert_eq!(bytes(&actual), before);
}
