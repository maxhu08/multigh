use crate::support::Sandbox;

#[test]
fn disallowed_author_and_committer_names_block_even_with_unknown_emails() {
    for kind in ["AUTHOR", "COMMITTER"] {
        let sandbox = Sandbox::new();

        sandbox.protect();

        let output = sandbox
            .command("git")
            .env(format!("GIT_{kind}_NAME"), "bOb ExAmPlE")
            .env(format!("GIT_{kind}_EMAIL"), "unlisted@example.net")
            .args([
                "commit",
                "--allow-empty",
                "--no-verify",
                "-m",
                "Wrong account name",
            ])
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );

        sandbox.ok("git", &["init", "--bare", "../remote.git"]);
        sandbox.blocked(
            "git",
            &["push", "../remote.git", "HEAD:refs/heads/main"],
            "contains your school identity",
        );

        assert!(
            !sandbox
                .run(
                    "git",
                    &[
                        "--git-dir=../remote.git",
                        "rev-parse",
                        "--verify",
                        "refs/heads/main"
                    ]
                )
                .status
                .success()
        );
    }
}

#[test]
fn already_remote_commits_are_excluded_and_multiple_new_refs_are_checked() {
    let sandbox = Sandbox::new();

    sandbox.protect();
    sandbox.ok(
        "git",
        &[
            "commit",
            "--allow-empty",
            "--no-verify",
            "--author=Bob Example <bob@example.edu>",
            "-m",
            "Existing baseline",
        ],
    );

    let baseline = sandbox.ok("git", &["rev-parse", "HEAD"]);

    sandbox.ok("git", &["init", "--bare", "../remote.git"]);
    sandbox.ok(
        "git",
        &[
            "push",
            "--no-verify",
            "../remote.git",
            "HEAD:refs/heads/main",
        ],
    );
    sandbox.commit();
    sandbox.ok("git", &["push", "../remote.git", "HEAD:refs/heads/main"]);
    sandbox.ok("git", &["branch", "old-history", baseline.trim()]);
    sandbox.blocked(
        "git",
        &[
            "push",
            "../remote.git",
            "HEAD:refs/heads/main",
            "old-history:refs/heads/old-history",
        ],
        "contains your school identity",
    );

    assert!(
        !sandbox
            .run(
                "git",
                &[
                    "--git-dir=../remote.git",
                    "rev-parse",
                    "--verify",
                    "refs/heads/old-history"
                ]
            )
            .status
            .success()
    );
}

#[test]
fn sha256_repositories_check_outgoing_commits_using_full_length_ids() {
    let sandbox = Sandbox::new();

    sandbox.protect();
    sandbox.ok("git", &["init", "--object-format=sha256", "../sha256"]);

    let run = |program: &str, args: &[&str]| {
        sandbox
            .command(program)
            .current_dir(sandbox.path("sha256"))
            .args(args)
            .output()
            .unwrap()
    };

    assert!(
        run("mgh", &["repo", "allowed", "add", "personal"])
            .status
            .success()
    );
    assert!(
        run(
            "git",
            &["commit", "--allow-empty", "-m", "Initial SHA256 commit"]
        )
        .status
        .success()
    );

    let head = run("git", &["rev-parse", "HEAD"]);

    assert!(head.status.success());
    assert_eq!(String::from_utf8_lossy(&head.stdout).trim().len(), 64);

    sandbox.ok(
        "git",
        &[
            "init",
            "--bare",
            "--object-format=sha256",
            "../remote-sha256.git",
        ],
    );

    let output = run(
        "git",
        &["push", "../remote-sha256.git", "HEAD:refs/heads/main"],
    );

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        run(
            "git",
            &[
                "commit",
                "--no-verify",
                "--allow-empty",
                "--author=Bob Example <bob@example.edu>",
                "-m",
                "Wrong SHA256 identity"
            ]
        )
        .status
        .success()
    );

    let output = run(
        "git",
        &["push", "../remote-sha256.git", "HEAD:refs/heads/main"],
    );

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("contains your school identity"));
}
