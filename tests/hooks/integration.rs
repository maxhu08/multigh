use crate::support::Sandbox;
use std::fs;

#[test]
fn custom_global_hooks_are_preserved_and_native_hooks_still_reject() {
    let sandbox = Sandbox::new();

    sandbox.ok("git", &["config", "--global", "core.hooksPath", "custom"]);
    sandbox.blocked("mgh", &["setup"], "custom global hooks path");

    assert_eq!(
        sandbox
            .ok("git", &["config", "--global", "core.hooksPath"])
            .trim(),
        "custom"
    );
    assert!(!sandbox.path("state/multigh/hooks").exists());

    sandbox.ok("git", &["config", "--global", "--unset", "core.hooksPath"]);
    sandbox.executable("repo/.git/hooks/pre-commit", "#!/bin/sh\nexit 1\n");
    sandbox.protect();
    sandbox.blocked(
        "git",
        &["commit", "--allow-empty", "-m", "Native rejection"],
        "Existing pre-commit hook rejected",
    );
}

#[test]
fn shared_hooks_preserve_message_and_post_commit_hooks() {
    let sandbox = Sandbox::new();

    sandbox.protect();
    sandbox.executable(
        "repo/.git/hooks/commit-msg",
        "#!/bin/sh\nprintf 'message-hook\\n' >&2\nexit 1\n",
    );
    sandbox.blocked(
        "git",
        &["commit", "--allow-empty", "-m", "Example"],
        "message-hook",
    );
    sandbox.executable("repo/.git/hooks/commit-msg", "#!/bin/sh\nexit 0\n");
    sandbox.executable(
        "repo/.git/hooks/post-commit",
        "#!/bin/sh\nprintf 'committed\\n' >> \"$TEST_MGH_ROOT/post-commit\"\n",
    );
    sandbox.commit();

    assert_eq!(
        fs::read_to_string(sandbox.path("post-commit")).unwrap(),
        "committed\n"
    );

    sandbox.ok("git", &["worktree", "add", "--detach", "../linked"]);

    let output = sandbox
        .command("git")
        .current_dir(sandbox.path("linked"))
        .args(["commit", "--allow-empty", "-m", "Linked"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(sandbox.path("post-commit")).unwrap(),
        "committed\ncommitted\n"
    );
}

#[test]
fn entry_repairs_reset_hook_paths_from_subdirectories() {
    let sandbox = Sandbox::new();

    sandbox.protect();
    fs::create_dir_all(sandbox.path("repo/checks with spaces")).unwrap();
    fs::create_dir_all(sandbox.path("repo/subdir")).unwrap();
    sandbox.executable("repo/checks with spaces/pre-commit", "#!/bin/sh\nexit 1\n");

    for _ in 0..2 {
        sandbox.ok(
            "git",
            &["config", "--local", "core.hooksPath", "checks with spaces"],
        );

        let output = sandbox
            .command("mgh")
            .current_dir(sandbox.path("repo/subdir"))
            .args(["internal", "enter"])
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            sandbox
                .ok("git", &["config", "--local", "mgh.originalHooksPath"])
                .trim(),
            "checks with spaces"
        );

        sandbox.blocked(
            "git",
            &["commit", "--allow-empty", "-m", "Native check"],
            "Existing pre-commit hook rejected",
        );
    }

    sandbox.ok("mgh", &["repo", "protections", "off"]);
    sandbox.ok(
        "git",
        &["config", "--local", "core.hooksPath", "checks with spaces"],
    );
    sandbox.ok("mgh", &["internal", "enter"]);

    assert_eq!(
        sandbox
            .ok("git", &["config", "--local", "core.hooksPath"])
            .trim(),
        "checks with spaces"
    );

    sandbox.ok("mgh", &["repo", "protections", "on"]);
    sandbox.blocked(
        "git",
        &["commit", "--allow-empty", "-m", "Re-enabled"],
        "Existing pre-commit hook rejected",
    );
}
