use crate::support::Sandbox;
use std::fs;

#[test]
fn repository_hook_paths_keep_checks_and_push_input_with_mgh() {
    let sandbox = Sandbox::new();

    sandbox.protect();
    fs::create_dir_all(sandbox.path("repo/.husky/_")).unwrap();

    for name in ["pre-commit", "commit-msg", "pre-push"] {
        sandbox.executable(
            &format!("repo/.husky/_/{name}"),
            "#!/bin/sh\nname=$(basename \"$0\")\nscript=$(dirname \"$(dirname \"$0\")\")/$name\n[ ! -f \"$script\" ] && exit 0\n[ \"${HUSKY-}\" = 0 ] && exit 0\nexec sh \"$script\" \"$@\"\n",
        );
    }

    sandbox.write(
        "repo/.husky/pre-commit",
        "printf 'husky-commit\\n' >> \"$TEST_MGH_ROOT/commits\"\n",
    );
    sandbox.write(
        "repo/.husky/pre-push",
        "printf '%s\\n' \"$@\" > \"$TEST_MGH_ROOT/push-args\"\ncat > \"$TEST_MGH_ROOT/updates\"\n",
    );
    sandbox.ok("git", &["config", "--local", "core.hooksPath", ".husky/_"]);

    let entry = sandbox.ok("mgh", &["enter"]);

    assert!(entry.contains("Existing hooks detected"));
    assert!(entry.contains("Identity protections are active; your existing checks are preserved."));
    assert!(
        entry
            .lines()
            .any(|line| line.contains("pre-commit") && line.contains(".husky/pre-commit"))
    );
    assert!(
        entry
            .lines()
            .any(|line| line.contains("pre-push") && line.contains(".husky/pre-push"))
    );
    assert!(!entry.contains(".husky/_"));

    sandbox.ok("mgh", &["verbose", "off"]);

    assert!(sandbox.ok("mgh", &["enter"]).is_empty());

    sandbox.ok("mgh", &["verbose", "on"]);

    assert_eq!(
        sandbox
            .ok("git", &["config", "--local", "mgh.originalHooksPath"])
            .trim(),
        ".husky/_"
    );
    assert_eq!(
        sandbox
            .ok("git", &["config", "--local", "core.hooksPath"])
            .trim(),
        sandbox.path("state/multigh/hooks").to_str().unwrap()
    );

    sandbox.commit();

    assert_eq!(
        fs::read_to_string(sandbox.path("commits")).unwrap(),
        "husky-commit\n"
    );

    sandbox.write("active", "bob\n");

    let output = sandbox
        .command("git")
        .env("HUSKY", "0")
        .args(["commit", "--allow-empty", "-m", "Wrong account"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("GitHub is using bob"));
    assert_eq!(
        fs::read_to_string(sandbox.path("commits")).unwrap(),
        "husky-commit\n"
    );

    sandbox.ok("git", &["init", "--bare", "../remote.git"]);
    sandbox.ok("git", &["remote", "set-url", "origin", "../remote.git"]);
    sandbox.blocked("git", &["push", "origin", "main"], "GitHub is using bob");

    assert!(!sandbox.path("updates").exists());

    sandbox.write("active", "alice\n");
    sandbox.ok("git", &["push", "origin", "main"]);

    assert!(
        fs::read_to_string(sandbox.path("updates"))
            .unwrap()
            .starts_with("refs/heads/main ")
    );
    assert_eq!(
        fs::read_to_string(sandbox.path("push-args")).unwrap(),
        "origin\n../remote.git\n"
    );

    sandbox.write(
        "repo/.husky/commit-msg",
        "printf 'husky-message-rejected\\n' >&2\nexit 1\n",
    );
    sandbox.blocked(
        "git",
        &["commit", "--allow-empty", "-m", "Rejected"],
        "husky-message-rejected",
    );
    sandbox.ok("mgh", &["protections", "off"]);

    assert!(
        !sandbox
            .ok("mgh", &["enter"])
            .contains("Identity protections are active;")
    );

    sandbox.blocked(
        "git",
        &["commit", "--allow-empty", "-m", "Still rejected"],
        "husky-message-rejected",
    );
}
