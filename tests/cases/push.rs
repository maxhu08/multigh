//! Outgoing commit history and push input, including real rejection at local remotes.

use super::support::*;
use serde_json::json;
use std::fs;
use std::io::Write;
use std::process::Stdio;

/// Long outgoing histories must detect an older committer's additional Unicode address and
/// still exclude it when the remote baseline already contains that commit.
#[test]
fn long_history_checks_older_committers_and_unicode_addresses() {
    let env = Sandbox::new();
    env.init_repo();
    env.setup_defaults();
    env.success(&["repo", "allowed", "add", "personal"]);
    fs::write(
        &env.config,
        IDENTITIES.replace(
            "\"email\": \"work@example.com\"",
            "\"email\": \"work@example.com\", \"additional_emails\": [\"ÉQUIPE@EXAMPLE.COM\"]",
        ),
    )
    .unwrap();
    let mut history = String::new();
    for index in 0..260 {
        let email = if index == 1 {
            "équipe@example.com"
        } else {
            "other@example.com"
        };
        history.push_str(&format!(
            "commit refs/heads/large\nmark :{}\nauthor Collaborator <other@example.com> 1700000000 +0000\ncommitter Collaborator <{email}> 1700000000 +0000\ndata 1\nx\n",
            index + 1,
        ));
        if index > 0 {
            history.push_str(&format!("from :{index}\n"));
        }
        history.push('\n');
    }
    let mut child = env
        .command("git")
        .args(["fast-import", "--quiet"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(history.as_bytes())
        .unwrap();
    assert_ok(&child.wait_with_output().unwrap());
    let head = env.git_without_hooks(&["rev-parse", "large"]);
    let bad = env.git_without_hooks(&["rev-parse", "large~258"]);
    let zero = "0".repeat(40);
    let input = format!("refs/heads/large {head} refs/heads/large {zero}\n");
    let rejected = env.hook("pre-push", input.as_bytes(), &[]);
    assert!(!rejected.status.success(), "{}", both(&rejected));
    assert!(err(&rejected).contains(&bad[..12]), "{}", both(&rejected));
    assert!(err(&rejected).contains("contains your work identity"));
    let input = format!("refs/heads/large {head} refs/heads/large {bad}\n");
    assert_ok(&env.hook("pre-push", input.as_bytes(), &[]));
}

/// Push checks must validate input and baselines, inspect outgoing history and leave rejected
/// remote refs absent.
#[test]
fn push_input_baselines_history_and_real_rejected_refs() {
    let env = Sandbox::new();
    env.init_repo();
    env.setup_defaults();
    env.success(&["repo", "allowed", "add", "personal"]);
    let safe = env.commit_without_hooks("Alice Example", "Alice@example.com");
    let zero = "0".repeat(40);
    for (input, phrase) in [
        (b"bad".to_vec(), "Invalid push-hook input"),
        (b"\xff".to_vec(), "UTF-8"),
        (
            b"refs/heads/main wrong refs/heads/main wrong\n".to_vec(),
            "Invalid commit ID",
        ),
    ] {
        let o = env.hook("pre-push", &input, &[]);
        assert!(!o.status.success());
        assert!(err(&o).contains(phrase), "{}", both(&o));
    }
    let deletion = format!("(delete) {zero} refs/heads/gone {safe}\n");
    assert_ok(&env.hook("pre-push", deletion.as_bytes(), &[]));
    let missing = format!(
        "refs/heads/main {safe} refs/heads/main {}\n",
        "f".repeat(40)
    );
    let o = env.hook("pre-push", missing.as_bytes(), &[]);
    assert!(!o.status.success());
    assert!(err(&o).contains("Fetch the remote and try again"));
    let bad = env.commit_without_hooks("Collaborator", "work@example.com");
    let input = format!("refs/heads/main {bad} refs/heads/main {safe}\n");
    let o = env.hook("pre-push", input.as_bytes(), &[]);
    assert!(!o.status.success());
    assert!(err(&o).contains("contains your work identity in a personal repository"));
    let remote = env.root.join("remote.git");
    env.git_without_hooks(&["init", "--bare", remote.to_str().unwrap()]);
    env.set_local("remote.origin.url", remote.to_str().unwrap());
    let o = env.native_git(&["push", "origin", "HEAD:refs/heads/rejected"]);
    assert!(!o.status.success());
    let o = env
        .command("git")
        .args([
            "--git-dir",
            remote.to_str().unwrap(),
            "show-ref",
            "--verify",
            "refs/heads/rejected",
        ])
        .output()
        .unwrap();
    assert!(!o.status.success());
    env.success(&["repo", "allowed", "add", "work"]);
    assert_ok(&env.hook("pre-push", input.as_bytes(), &[]));
    // A bad baseline itself is excluded when no newer commits exist.
    env.success(&["repo", "allowed", "remove", "work"]);
    let equal = format!("refs/heads/main {bad} refs/heads/main {bad}\n");
    assert_ok(&env.hook("pre-push", equal.as_bytes(), &[]));
}

/// Collaborators are allowed, but disallowed configured identities must be detected across
/// emails and multiple refs.
#[test]
fn push_collaborators_names_additional_addresses_and_multiple_refs() {
    let env = Sandbox::new();
    env.init_repo();
    env.setup_defaults();
    env.success(&["repo", "allowed", "add", "personal"]);
    let zero = "0".repeat(40);
    let safe = env.commit_without_hooks("Unrelated Collaborator", "someone@else");
    let input = format!("refs/heads/a {safe} refs/heads/a {zero}\n");
    assert_ok(&env.hook("pre-push", input.as_bytes(), &[]));
    let bad = env.commit_without_hooks("ALICE-WORK", "someone@else");
    let input = format!(
        "refs/heads/a {safe} refs/heads/a {zero}\nrefs/heads/b {bad} refs/heads/b {zero}\n"
    );
    let o = env.hook("pre-push", input.as_bytes(), &[]);
    assert!(!o.status.success());
    assert!(err(&o).contains(&bad[..12]));
    env.success(&["repo", "allowed", "add", "work"]);
    env.set_accounts(json!({
        "selected": "alice-work",
        "accounts": [
            {
                "login": "alice-work",
                "active": true,
                "state": "success"
            }
        ]
    }));
    let bad = env.commit_without_hooks("Collaborator", "123+ALICE@USERS.NOREPLY.GITHUB.COM");
    env.success(&["repo", "allowed", "remove", "personal"]);
    let input = format!("refs/heads/a {bad} refs/heads/a {zero}\n");
    assert!(!env.hook("pre-push", input.as_bytes(), &[]).status.success());
}

/// Push checks must support SHA-256 and annotated tags, inspect stored commits and avoid
/// shared-name false positives.
#[test]
fn push_sha256_annotated_tags_shared_names_and_environment_not_checked() {
    let env = Sandbox::new();
    env.git_without_hooks(&["init", "--object-format=sha256", "-b", "main"]);
    env.setup_defaults();
    env.success(&["repo", "allowed", "add", "personal"]);
    let safe = env.commit_without_hooks("Alice Example", "Alice@example.com");
    assert_eq!(safe.len(), 64);
    let zero = "0".repeat(64);
    env.git_without_hooks(&[
        "-c",
        "user.name=Alice Example",
        "-c",
        "user.email=Alice@example.com",
        "tag",
        "-a",
        "safe",
        "-m",
        "tag",
    ]);
    let tag = env.git_without_hooks(&["rev-parse", "safe"]);
    let input = format!("refs/tags/safe {tag} refs/tags/safe {zero}\n");
    let mut child = env
        .mgh(&["internal", "hook", "pre-push"])
        .env("GIT_AUTHOR_NAME", "WRONG CURRENT NAME")
        .env("GIT_COMMITTER_EMAIL", "work@example.com")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    assert_ok(&child.wait_with_output().unwrap());
    let bad = env.commit_without_hooks("Alice Work", "outside@else");
    let input = format!("refs/heads/main {bad} refs/heads/main {safe}\n");
    let o = env.hook("pre-push", input.as_bytes(), &[]);
    assert!(!o.status.success());
    assert!(err(&o).contains(&bad[..12]));
    let value = IDENTITIES.replace("Alice Work", "Alice Example");
    fs::write(&env.config, value).unwrap();
    assert_ok(&env.hook("pre-push", input.as_bytes(), &[]));
}

/// Tree/blob tags still require a permitted live account, but have no commit identity to
/// inspect.
#[test]
fn noncommit_tags_require_authentication_but_have_no_commit_identity() {
    let env = Sandbox::new();
    env.init_repo();
    env.setup_defaults();
    env.success(&["repo", "allowed", "add", "personal"]);
    fs::write(env.repo.join("document"), "standalone document").unwrap();
    let blob = env.git_without_hooks(&["hash-object", "-w", "document"]);
    env.git_without_hooks(&["add", "document"]);
    let tree = env.git_without_hooks(&["write-tree"]);
    env.git_without_hooks(&[
        "-c",
        "user.name=Author",
        "-c",
        "user.email=other@else",
        "tag",
        "-a",
        "document",
        "-m",
        "blob tag",
        &blob,
    ]);
    let tag = env.git_without_hooks(&["rev-parse", "document"]);
    let zero = "0".repeat(40);
    for object in [&blob, &tree, &tag] {
        let input = format!("refs/tags/document {object} refs/tags/document {zero}\n");
        assert_ok(&env.hook("pre-push", input.as_bytes(), &[]));
    }
    env.set_accounts(json!({
        "selected": "alice-work",
        "accounts": [
            {
                "login": "alice-work",
                "active": true,
                "state": "success"
            }
        ]
    }));
    let input = format!("refs/tags/document {blob} refs/tags/document {zero}\n");
    let o = env.hook("pre-push", input.as_bytes(), &[]);
    assert!(!o.status.success());
    assert!(err(&o).contains("not allowed"));
}

/// Authorized pushes must replay stdin exactly; rejected identities must be found anywhere in
/// outgoing ancestry.
#[test]
fn protected_push_replays_original_whitespace_and_checks_all_ancestors() {
    let env = Sandbox::new();
    env.init_repo();
    env.setup_defaults();
    env.success(&["repo", "allowed", "add", "personal"]);
    let first = env.commit_without_hooks("Collaborator", "other@else");
    env.write_hook("pre-push", "cat > captured-push", true);
    let zero = "0".repeat(40);
    let input = format!("refs/heads/new\t{first}  refs/heads/new\t{zero}  \r\n");
    assert_ok(&env.hook("pre-push", input.as_bytes(), &[]));
    assert_eq!(bytes(env.repo.join("captured-push")), input.as_bytes());
    let rejected = env.hook("pre-push", b"bad input", &[]);
    assert!(!rejected.status.success());
    assert_eq!(bytes(env.repo.join("captured-push")), input.as_bytes());
    let bad = env.commit_without_hooks("Alice Work", "work@example.com");
    let safe = env.commit_without_hooks("Collaborator", "other@else");
    let input = format!("refs/heads/new {safe} refs/heads/new {zero}\n");
    let o = env.hook("pre-push", input.as_bytes(), &[]);
    assert!(!o.status.success());
    assert!(err(&o).contains(&bad[..12]));
    let input = format!("refs/heads/new {safe} refs/heads/new {bad}\n");
    assert_ok(&env.hook("pre-push", input.as_bytes(), &[]));
    env.git_without_hooks(&[
        "-c",
        "user.name=Alice",
        "-c",
        "user.email=a@b",
        "tag",
        "-a",
        "baseline",
        "-m",
        "baseline",
        &bad,
    ]);
    let baseline = env.git_without_hooks(&["rev-parse", "baseline"]);
    let input = format!("refs/heads/new {safe} refs/heads/new {baseline}\n");
    assert_ok(&env.hook("pre-push", input.as_bytes(), &[]));
}
