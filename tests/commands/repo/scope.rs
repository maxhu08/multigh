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
