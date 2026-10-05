use crate::support::Sandbox;
use std::fs;

#[test]
fn every_repo_command_requires_a_repository_without_mutating_global_state() {
    let sandbox = Sandbox::new();
    let global = fs::read(sandbox.path("gitconfig")).unwrap();
    for args in [
        vec!["repo"],
        vec!["repo", "status"],
        vec!["repo", "check"],
        vec!["repo", "protections"],
        vec!["repo", "protections", "off"],
        vec!["repo", "allowed"],
        vec!["repo", "allowed", "add", "personal"],
        vec!["repo", "allowed", "remove", "personal"],
        vec!["repo", "allowed", "update"],
    ] {
        let result = sandbox
            .command("mgh")
            .current_dir(sandbox.path(""))
            .args(args)
            .output()
            .unwrap();
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("No Git repository found"));
    }
    assert_eq!(fs::read(sandbox.path("gitconfig")).unwrap(), global);
    assert!(!sandbox.path("gh-calls").exists());
}

#[test]
fn malformed_git_configuration_is_reported_instead_of_treated_as_no_repository() {
    let sandbox = Sandbox::new();
    sandbox.write("repo/.git/config", "[invalid configuration\n");
    let global = fs::read(sandbox.path("gitconfig")).unwrap();

    let result = sandbox.run("mgh", &["repo", "allowed"]);
    let error = String::from_utf8_lossy(&result.stderr);

    assert!(!result.status.success());
    assert!(error.contains("bad config"), "{error}");
    assert!(!error.contains("No Git repository found"), "{error}");
    assert_eq!(fs::read(sandbox.path("gitconfig")).unwrap(), global);
    assert!(!sandbox.path("gh-calls").exists());
}

#[test]
fn relative_git_environment_paths_keep_permission_changes_in_the_selected_repository() {
    let sandbox = Sandbox::new();
    sandbox.protect();
    let global = fs::read(sandbox.path("gitconfig")).unwrap();

    let result = sandbox
        .command("mgh")
        .current_dir(sandbox.path(""))
        .env("GIT_DIR", "repo/.git")
        .env("GIT_WORK_TREE", "repo")
        .args(["repo", "allowed", "add", "school"])
        .output()
        .unwrap();

    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        sandbox.ok(
            "git",
            &["config", "--local", "--get-all", "mgh.allowed-identity"]
        ),
        "personal\nschool\n"
    );
    assert_eq!(fs::read(sandbox.path("gitconfig")).unwrap(), global);
    assert!(!sandbox.path(".git").exists());
    sandbox.ok("mgh", &["repo", "check"]);
}
