use crate::support::{Sandbox, terminal};
use std::fs;

#[test]
fn status_reports_live_authentication_identity_and_protection_without_mutation() {
    let sandbox = Sandbox::new();

    sandbox.protect();
    sandbox.write("selected", "bob\n");

    let before = fs::read(sandbox.path("gitconfig")).unwrap();
    let local = fs::read(sandbox.path("repo/.git/config")).unwrap();
    let status = sandbox.ok("mgh", &["status"]);

    assert!(status.contains(&format!(
        "  alice alice@example.com (Active)\n  {}",
        sandbox.path("state/multigh/identities/git-personal.conf").display()
    )));
    assert!(status.contains(&format!(
        "  bob bob@example.edu\n  {}",
        sandbox.path("state/multigh/identities/git-school.conf").display()
    )));
    assert!(status.contains("Identities"));
    assert_eq!(status.matches("(Active)").count(), 1);
    assert!(status.contains(&format!(
        "  Accounts\n  {}",
        sandbox.path("config/multigh/accounts.conf").display()
    )));
    assert!(!status.contains("Account file") && !status.contains("Global commit defaults"));
    assert!(status.contains("Allowed accounts personal"));
    assert!(status.contains("alice@example.com") && status.contains("Account and identity match"));
    assert!(status.contains("Commit + push checks enabled"));
    assert!(status.find("Accounts").unwrap() < status.find("Current repository").unwrap());
    assert_eq!(before, fs::read(sandbox.path("gitconfig")).unwrap());
    assert_eq!(local, fs::read(sandbox.path("repo/.git/config")).unwrap());

    let full = sandbox.ok("mgh", &["status", "--full"]);

    assert!(full.contains("GitHub authentication details"));
}

#[test]
fn status_explains_unconfigured_disallowed_and_mismatched_repositories() {
    let sandbox = Sandbox::new();

    assert!(
        sandbox
            .ok("mgh", &["status"])
            .contains("No accounts are authorized")
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
            .contains("identity does not match")
    );

    sandbox.ok("mgh", &["protections", "off"]);

    assert!(
        sandbox
            .ok("mgh", &["status"])
            .contains("Protection       OFF")
    );
}

#[test]
fn status_handles_authentication_failures_and_invalid_account_config() {
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
    fs::remove_file(sandbox.path("config/multigh/accounts.conf")).unwrap();

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
    assert!(text.contains("alice alice@example.com (Active)"));
    assert!(text.contains("bob bob@example.edu"));
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
    assert!(output.contains("someone-with-a-long-login not configured\n  Identity not configured"));
    assert!(output.contains(&format!(
        "  ALICE alice@example.com (Active)\n  {}",
        sandbox.path("state/multigh/identities/git-personal.conf").display()
    )));
    assert!(output.contains(&format!(
        "  bob bob@example.edu\n  {}",
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
                    sandbox.path("config/multigh/accounts.conf").display()
                )),
                "{output}"
            );
        } else {
            assert!(!output.contains("\x1b["), "{output}");
            assert!(
                output.contains(&format!(
                    "  Accounts\r\n  {}",
                    sandbox.path("config/multigh/accounts.conf").display()
                )),
                "{output}"
            );
        }
    }
}
