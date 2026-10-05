use crate::support::{Sandbox, quote, terminal};
use std::fs;

#[test]
fn status_uses_identity_names_even_when_github_logins_differ_or_are_not_signed_in() {
    let sandbox = Sandbox::new();

    sandbox.write(
        "config/multigh/identities.jsonc",
        r#"{"PeRsOnAl": {"username": "alice", "commit": {"email": "alice@example.com"}}, "WoRk": {"username": "carol", "commit": {"email": "carol@example.com"}}}"#,
    );
    sandbox.write(
        "accounts-json",
        r#"[{"login":"ALICE","active":true,"state":"success"},{"login":"unmapped","active":false,"state":"success"}]"#,
    );

    let output = sandbox.ok("mgh", &["status"]);

    assert!(
        output.contains("personal (Active)\n    GitHub username           alice\n    Commit name               alice\n    Commit email              alice@example.com"),
        "{output}"
    );
    assert!(
        output.contains("work (Not signed in)\n    GitHub username           carol\n    Commit name               carol\n    Commit email              carol@example.com"),
        "{output}"
    );
    assert!(output.contains("git-personal.conf") && output.contains("git-work.conf"));
    assert!(
        output.contains("Unconfigured GitHub accounts\n  unmapped"),
        "{output}"
    );
    assert!(!output.contains("ALICE alice@example.com"));
}

#[test]
fn status_reports_live_authentication_identity_and_protection_without_mutation() {
    let sandbox = Sandbox::new();

    sandbox.protect();
    sandbox.write("selected", "bob\n");

    let before = fs::read(sandbox.path("gitconfig")).unwrap();
    let local = fs::read(sandbox.path("repo/.git/config")).unwrap();
    let status = sandbox.ok("mgh", &["status"]);

    assert!(status.contains(&format!(
        "  personal (Active)\n    GitHub username           alice\n    Commit name               Alice Example\n    Commit email              alice@example.com\n    Identity file             {}",
        sandbox.path("state/multigh/identities/git-personal.conf").display()
    )));
    assert!(status.contains(&format!(
        "  school\n    GitHub username           bob\n    Commit name               Bob Example\n    Commit email              bob@example.edu\n    Identity file             {}",
        sandbox.path("state/multigh/identities/git-school.conf").display()
    )));
    assert!(status.contains("Identities"));
    assert_eq!(status.matches("(Active)").count(), 1);
    assert!(status.contains(&format!(
        "  Accounts\n  {}",
        sandbox.path("config/multigh/identities.jsonc").display()
    )));
    assert!(!status.contains("Account file") && !status.contains("Global commit defaults"));
    assert!(status.contains("Allowed identities          personal"));
    assert!(
        status.contains("alice@example.com")
            && status.contains("Identity and commit details match")
    );
    assert!(status.contains("Commit + push checks enabled"));
    assert!(status.contains("Active identity             personal"));
    assert!(status.contains("Autoswitch (global)         OFF"));
    assert!(status.find("Accounts").unwrap() < status.find("Current repository").unwrap());
    assert_eq!(before, fs::read(sandbox.path("gitconfig")).unwrap());
    assert_eq!(local, fs::read(sandbox.path("repo/.git/config")).unwrap());

    let full = sandbox.ok("mgh", &["status", "--full"]);

    assert!(full.contains("GitHub authentication details"));
}

#[test]
fn repository_status_reports_local_preferences_and_works_in_bare_repositories() {
    let sandbox = Sandbox::new();
    sandbox.protect();
    sandbox.ok("mgh", &["settings", "autoswitch", "on"]);
    assert!(
        sandbox
            .ok("mgh", &["repo", "status"])
            .contains("Autoswitch (global)         ON")
    );
    sandbox.ok("git", &["init", "--bare", "../bare.git"]);
    let output = sandbox
        .command("mgh")
        .current_dir(sandbox.path("bare.git"))
        .args(["repo", "status"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("Current repository          bare.git"));
    assert!(text.contains("No identities are authorized"));
    assert!(text.contains("Autoswitch (global)         ON"));
    assert!(text.contains(&format!(
        "Repo config                 {}",
        sandbox.path("bare.git/config").canonicalize().unwrap().display()
    )));
}

#[test]
fn linked_worktree_status_shows_the_common_repository_config_path_without_changes() {
    let sandbox = Sandbox::new();
    sandbox.protect();
    sandbox.commit();
    sandbox.ok("git", &["worktree", "add", "-b", "linked", "../linked"]);
    let config = sandbox.path("repo/.git/config");
    let original = fs::read(&config).unwrap();
    let output = sandbox
        .command("mgh")
        .current_dir(sandbox.path("linked"))
        .args(["repo", "status"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.contains(&format!(
            "Repo config                 {}",
            config.canonicalize().unwrap().display()
        )),
        "{text}"
    );
    assert!(!text.contains("·"));
    assert_eq!(fs::read(config).unwrap(), original);
}

#[test]
fn status_explains_unconfigured_disallowed_and_mismatched_repositories() {
    let sandbox = Sandbox::new();

    assert!(
        sandbox
            .ok("mgh", &["status"])
            .contains("No identities are authorized")
    );

    sandbox.protect();
    sandbox.write("active", "bob\n");

    assert!(
        sandbox
            .ok("mgh", &["status"])
            .contains("not allowed in this repository")
    );

    sandbox.write("active", "alice\n");
    sandbox.ok(
        "git",
        &["config", "--local", "user.email", "bob@example.edu"],
    );

    assert!(
        sandbox
            .ok("mgh", &["status"])
            .contains("commit details do not match identity")
    );

    sandbox.ok("mgh", &["repo", "protections", "off"]);

    assert!(
        sandbox
            .ok("mgh", &["status"])
            .contains("Protection                  OFF")
    );
}

#[test]
fn status_handles_authentication_failures_and_invalid_identity_config() {
    let sandbox = Sandbox::new();

    sandbox.write("accounts-json", "[]\n");
    sandbox.blocked("mgh", &["status"], "No active GitHub account");
    sandbox.write("accounts-json", "invalid json\n");

    assert!(!sandbox.run("mgh", &["status"]).status.success());

    sandbox.write(
        "accounts-json",
        "[{\"login\":\"alice\",\"active\":true,\"state\":\"expired\"}]\n",
    );

    assert!(sandbox.ok("mgh", &["status"]).contains("expired"));

    sandbox.write("fail-full", "");
    sandbox.blocked(
        "mgh",
        &["status", "--full"],
        "GitHub authentication check failed",
    );
    fs::remove_file(sandbox.path("config/multigh/identities.jsonc")).unwrap();

    assert!(sandbox.ok("mgh", &["status"]).contains("Read "));

    sandbox.write("fail-auth", "");
    sandbox.blocked("mgh", &["status"], "gh:");
}

#[test]
fn status_outside_a_repository_reports_identity_entries_without_global_defaults() {
    let sandbox = Sandbox::new();

    let output = sandbox
        .command("mgh")
        .current_dir(sandbox.path(""))
        .arg("status")
        .output()
        .unwrap();

    assert!(output.status.success());

    let text = String::from_utf8_lossy(&output.stdout);

    assert!(text.contains("Identities"));
    assert!(text.contains("personal (Active)\n    GitHub username           alice\n    Commit name               Alice Example\n    Commit email              alice@example.com"));
    assert!(text.contains("school\n    GitHub username           bob\n    Commit name               Bob Example\n    Commit email              bob@example.edu"));
    assert!(!text.contains("GitHub accounts") && !text.contains("Account file"));
    assert!(!text.contains("Global commit defaults"));
    assert!(!text.contains("Current repository"));
}

#[test]
fn status_matches_identity_files_case_insensitively_and_explains_unconfigured_accounts() {
    let sandbox = Sandbox::new();

    sandbox.write(
        "accounts-json",
        r#"[{"login":"someone-with-a-long-login","active":false,"state":"success"},{"login":"ALICE","active":true,"state":"success"},{"login":"bob","active":false,"state":"success"}]"#,
    );

    let output = sandbox.ok("mgh", &["status"]);
    assert_eq!(output.matches("(Active)").count(), 1);
    assert!(output.contains("Unconfigured GitHub accounts\n  someone-with-a-long-login"));
    assert!(output.contains(&format!(
        "  personal (Active)\n    GitHub username           alice\n    Commit name               Alice Example\n    Commit email              alice@example.com\n    Identity file             {}",
        sandbox.path("state/multigh/identities/git-personal.conf").display()
    )));
    assert!(output.contains(&format!(
        "  school\n    GitHub username           bob\n    Commit name               Bob Example\n    Commit email              bob@example.edu\n    Identity file             {}",
        sandbox.path("state/multigh/identities/git-school.conf").display()
    )));
}

#[test]
fn status_active_marker_and_accounts_heading_respect_terminal_colors() {
    let sandbox = Sandbox::new();

    for color in [true, false] {
        let mut command = sandbox.command("mgh");

        command.current_dir(sandbox.path(""));

        if color {
            command.env_remove("NO_COLOR").env("TERM", "xterm-256color");
        }

        let (status, output) = terminal(command.arg("status"), &[]);

        assert!(status.success(), "{output}");
        assert_eq!(output.matches("(Active)").count(), 1);

        if color {
            assert!(
                output.contains("\x1b[1;38;2;134;239;172m(Active)\x1b[0m"),
                "{output}"
            );
            assert!(
                output.contains(&format!(
                    "\x1b[1;38;2;192;132;252mAccounts\x1b[0m\r\n  \x1b[38;2;148;163;184m{}\x1b[0m",
                    sandbox.path("config/multigh/identities.jsonc").display()
                )),
                "{output}"
            );
        } else {
            assert!(!output.contains("\x1b["), "{output}");
            assert!(
                output.contains(&format!(
                    "  Accounts\r\n  {}",
                    sandbox.path("config/multigh/identities.jsonc").display()
                )),
                "{output}"
            );
        }
    }
}

#[test]
fn full_status_preserves_github_cli_colors_bold_text_and_indentation() {
    let sandbox = Sandbox::new();
    sandbox.write("styled-auth-report", "");
    let before = fs::read(sandbox.path("gitconfig")).unwrap();
    let local = fs::read(sandbox.path("repo/.git/config")).unwrap();
    let (status, output) = terminal(
        sandbox
            .command("mgh")
            .env_remove("NO_COLOR")
            .env_remove("CLICOLOR")
            .env_remove("CLICOLOR_FORCE")
            .args(["status", "--full"]),
        &[],
    );

    assert!(status.success(), "{output}");
    assert!(
        output.contains("  \x1b[1mAuthentication report\x1b[0m\r\n"),
        "{output}"
    );
    assert!(
        output
            .contains("  github.com\r\n    \x1b[32m✓\x1b[0m Logged in as \x1b[1malice\x1b[0m\r\n"),
        "{output}"
    );
    assert_eq!(fs::read(sandbox.path("gitconfig")).unwrap(), before);
    assert_eq!(fs::read(sandbox.path("repo/.git/config")).unwrap(), local);
}

#[test]
fn full_status_respects_color_opt_outs_and_redirected_output_even_with_forced_gh_color() {
    let sandbox = Sandbox::new();
    sandbox.write("styled-auth-report", "");

    for (key, value) in [("NO_COLOR", ""), ("TERM", "dumb"), ("CLICOLOR", "0")] {
        let (status, output) = terminal(
            sandbox
                .command("mgh")
                .env_remove("NO_COLOR")
                .env("CLICOLOR_FORCE", "1")
                .env("GH_FORCE_TTY", "1")
                .env(key, value)
                .args(["status", "--full"]),
            &[],
        );

        assert!(status.success(), "{output}");
        assert!(
            output.contains("  Authentication report\r\n"),
            "{key}: {output}"
        );
        assert!(
            output.contains("  github.com\r\n    ✓ Logged in as alice\r\n"),
            "{key}: {output}"
        );
    }

    let report = sandbox
        .command("mgh")
        .env_remove("NO_COLOR")
        .env("CLICOLOR_FORCE", "1")
        .env("GH_FORCE_TTY", "1")
        .args(["status", "--full"])
        .output()
        .unwrap();

    assert!(report.status.success());
    assert!(!report.stdout.contains(&0x1b));
    assert_eq!(
        String::from_utf8_lossy(&report.stderr),
        "  github.com\n    ✓ Logged in as alice\n"
    );
}

#[test]
fn full_status_keeps_styled_reports_and_failure_status_when_authentication_fails() {
    let sandbox = Sandbox::new();
    sandbox.write("styled-auth-report", "");
    sandbox.write("fail-full", "");
    let (status, output) = terminal(
        sandbox
            .command("mgh")
            .env_remove("NO_COLOR")
            .args(["status", "--full"]),
        &[],
    );

    assert!(!status.success());
    assert!(
        output.contains("\x1b[1mAuthentication report\x1b[0m"),
        "{output}"
    );
    assert!(
        output.contains("\x1b[32m✓\x1b[0m Logged in as \x1b[1malice\x1b[0m"),
        "{output}"
    );
    assert!(
        output.contains("GitHub authentication check failed"),
        "{output}"
    );
}

#[test]
fn full_status_does_not_send_ansi_styles_to_either_redirected_stream() {
    let sandbox = Sandbox::new();
    sandbox.write("styled-auth-report", "");

    for stream in [">", "2>"] {
        let file = sandbox.path("redirected-report");
        let script = format!(
            "mgh status --full {stream} {}",
            quote(file.to_str().unwrap())
        );
        let (status, output) = terminal(
            sandbox
                .command("zsh")
                .env_remove("NO_COLOR")
                .env("CLICOLOR_FORCE", "1")
                .env("GH_FORCE_TTY", "1")
                .args(["-fc", &script]),
            &[],
        );
        let redirected = fs::read_to_string(&file).unwrap();

        assert!(status.success(), "{output}");
        assert!(!redirected.contains('\x1b'), "{redirected}");

        if stream == ">" {
            assert!(redirected.contains("  Authentication report\n"));
            assert!(output.contains("  github.com\r\n    ✓ Logged in as alice\r\n"));
        } else {
            assert_eq!(redirected, "  github.com\n    ✓ Logged in as alice\n");
            assert!(output.contains("  Authentication report\r\n"));
        }
    }
}
