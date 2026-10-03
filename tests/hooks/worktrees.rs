use crate::support::Sandbox;
use std::fs;

#[test]
fn worktree_hook_overrides_are_preserved_without_affecting_other_worktrees() {
    let sandbox = Sandbox::new();

    sandbox.protect();
    sandbox.commit();
    sandbox.ok("git", &["config", "extensions.worktreeConfig", "true"]);
    sandbox.ok("git", &["worktree", "add", "--detach", "../linked"]);
    fs::create_dir_all(sandbox.path("linked/custom hooks")).unwrap();
    fs::create_dir_all(sandbox.path("linked/subdir")).unwrap();
    sandbox.executable(
        "linked/custom hooks/pre-commit",
        "#!/bin/sh\nprintf 'worktree-hook\\n' >&2\nexit 1\n",
    );

    let run = |program: &str, args: &[&str], subdir: bool| {
        sandbox
            .command(program)
            .current_dir(sandbox.path(if subdir { "linked/subdir" } else { "linked" }))
            .args(args)
            .output()
            .unwrap()
    };

    assert!(
        run(
            "git",
            &["config", "--worktree", "core.hooksPath", "custom hooks"],
            false
        )
        .status
        .success()
    );

    let entry = run("mgh", &["enter"], true);

    assert!(
        entry.status.success(),
        "{}",
        String::from_utf8_lossy(&entry.stderr)
    );

    let text = String::from_utf8_lossy(&entry.stdout);

    assert!(text.contains("Existing hooks detected") && text.contains("custom hooks/pre-commit"));
    assert!(text.contains("Identity protections are active;"));

    let preserved = run(
        "git",
        &["config", "--worktree", "--get", "mgh.originalHooksPath"],
        false,
    );

    assert_eq!(
        String::from_utf8_lossy(&preserved.stdout).trim(),
        "custom hooks"
    );
    assert!(
        !sandbox
            .run(
                "git",
                &["config", "--local", "--get", "mgh.originalHooksPath"]
            )
            .status
            .success()
    );

    let commit = run("git", &["commit", "--allow-empty", "-m", "Rejected"], false);

    assert!(!commit.status.success());
    assert!(String::from_utf8_lossy(&commit.stderr).contains("worktree-hook"));

    sandbox.commit();
}
