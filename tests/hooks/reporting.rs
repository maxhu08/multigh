use crate::support::Sandbox;

#[test]
fn detected_hooks_are_listed_individually_with_inactive_hooks_marked() {
    let sandbox = Sandbox::new();

    sandbox.executable("repo/.git/hooks/pre-commit", "#!/bin/sh\nexit 0\n");
    sandbox.executable("repo/.git/hooks/pre-push", "#!/bin/sh\nexit 0\n");
    sandbox.write("repo/.git/hooks/commit-msg", "#!/bin/sh\nexit 1\n");
    sandbox.protect();

    let entry = sandbox.ok("mgh", &["enter"]);

    for name in ["pre-commit", "pre-push", "commit-msg"] {
        assert!(
            entry
                .lines()
                .any(|line| line.contains(name) && line.contains(&format!(".git/hooks/{name}")))
        );
    }

    assert!(
        entry
            .lines()
            .any(|line| line.contains("commit-msg") && line.contains("not executable; skipped"))
    );
    assert!(!entry.contains(".sample"));

    sandbox.commit();
}
