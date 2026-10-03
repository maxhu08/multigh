use crate::support::{IDENTITIES, Sandbox};
use std::fs;

#[test]
fn push_checks_outgoing_commits_and_chains_hook_input() {
    let sandbox = Sandbox::new();

    sandbox.write(
        "config/multigh/identities.jsonc",
        &IDENTITIES
            .replace("\"personal\"", "\"PeRsOnAl\"")
            .replace("\"school\"", "\"SCHOOL\""),
    );
    sandbox.executable(
        "repo/.git/hooks/pre-push",
        "#!/bin/sh\ncat > \"$TEST_MGH_ROOT/updates\"\n",
    );
    sandbox.protect();
    sandbox.ok("git", &["config", "mgh.account", "PERSONAL"]);
    sandbox.commit();
    sandbox.ok("git", &["init", "--bare", "../remote.git"]);
    sandbox.ok("git", &["remote", "set-url", "origin", "../remote.git"]);

    sandbox.write("active", "bob\n");
    sandbox.blocked("git", &["push", "origin", "main"], "GitHub is using bob");

    sandbox.write("active", "alice\n");
    sandbox.ok("git", &["push", "origin", "main"]);

    assert!(
        fs::read_to_string(sandbox.path("updates"))
            .unwrap()
            .starts_with("refs/heads/main ")
    );

    sandbox.ok(
        "git",
        &[
            "commit",
            "--no-verify",
            "--allow-empty",
            "--author=Alice Example <456+bob@users.noreply.github.com>",
            "-m",
            "Bypassed guard",
        ],
    );
    sandbox.blocked(
        "git",
        &["push", "origin", "main"],
        "contains your school identity",
    );

    sandbox.ok(
        "git",
        &[
            "commit",
            "--amend",
            "--allow-empty",
            "--no-edit",
            "--reset-author",
        ],
    );
    sandbox.ok("git", &["push", "origin", "main"]);

    sandbox.ok(
        "git",
        &[
            "commit",
            "--allow-empty",
            "--author=Collaborator <someone@example.org>",
            "--no-verify",
            "-m",
            "Collaborator",
        ],
    );
    sandbox.ok("git", &["push", "origin", "main"]);
}
