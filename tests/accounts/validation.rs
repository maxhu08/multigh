use crate::support::{ACCOUNTS, Sandbox};
use std::path::Path;

#[test]
fn account_file_rejects_shared_identities_unknown_fields_and_duplicate_sections() {
    let sandbox = Sandbox::new();

    for malformed in [
        ACCOUNTS.replace("bob@example.edu", "alice@example.com"),
        ACCOUNTS.replace("username = bob", "username = alice"),
        ACCOUNTS.replace("allowed_emails", "allowed_email"),
    ] {
        sandbox.write("config/multigh/accounts.conf", &malformed);

        assert!(!sandbox.run("mgh", &["setup"]).status.success());
    }

    sandbox.write(
        "config/multigh/accounts.conf",
        &format!("{ACCOUNTS}\n[personal]\nusername = charlie\nemail = charlie@example.com\n"),
    );

    assert!(!sandbox.run("mgh", &["setup"]).status.success());
    assert!(!Path::new(&sandbox.path("state/multigh/identities")).exists());
}

#[test]
fn malformed_fields_are_rejected_before_git_settings_change() {
    let sandbox = Sandbox::new();

    let before = std::fs::read(sandbox.path("gitconfig")).unwrap();

    for config in [
        "".to_owned(),
        "username = alice\n".to_owned(),
        "[personal\n".to_owned(),
        ACCOUNTS.replace("[personal]", "[1personal]"),
        ACCOUNTS.replace("[personal]", "[bad.name]"),
        ACCOUNTS.replace("username = alice", "username = -alice"),
        ACCOUNTS.replace("username = alice", "username = alice_name"),
        ACCOUNTS.replace("username = alice", "username = "),
        ACCOUNTS.replace("name = Alice Example", "name = "),
        ACCOUNTS.replace("name = Alice Example", "name = Alice <Example>"),
        ACCOUNTS.replace("name = Alice Example", "name = Alice\tExample"),
        ACCOUNTS.replace("email = alice@example.com", "email = "),
        ACCOUNTS.replace("email = alice@example.com", "email = alice@@example.com"),
        ACCOUNTS.replace("email = alice@example.com", "email = alice@"),
        ACCOUNTS.replace(
            "email = alice@example.com",
            "email = alice example@example.com",
        ),
        ACCOUNTS.replace("123+alice@users.noreply.github.com", "not-an-email"),
        ACCOUNTS.replace("username = bob", "username = ALICE"),
        ACCOUNTS.replace("bob@example.edu", "ALICE@example.com"),
        ACCOUNTS.replace(
            "456+bob@users.noreply.github.com",
            "123+ALICE@users.noreply.github.com",
        ),
        ACCOUNTS.replace(
            "email = alice@example.com",
            "email = alice@example.com\nemail = duplicate@example.com",
        ),
    ] {
        sandbox.write("config/multigh/accounts.conf", &config);

        assert!(
            !sandbox.run("mgh", &["setup"]).status.success(),
            "Accepted config: {config}"
        );
        assert_eq!(before, std::fs::read(sandbox.path("gitconfig")).unwrap());
    }
}

#[test]
fn optional_names_custom_aliases_and_the_complete_example_are_accepted() {
    let sandbox = Sandbox::new();

    sandbox.write("config/multigh/accounts.conf", "[My_Account-2]\nusername = alice\nemail = alice@example.com\nallowed_emails =\n    123+alice@users.noreply.github.com\n");
    sandbox.ok("mgh", &["setup"]);
    sandbox.ok("mgh", &["protections", "--allow", "MY_ACCOUNT-2"]);

    assert_eq!(sandbox.ok("git", &["config", "user.name"]).trim(), "alice");

    sandbox.commit();
    sandbox.write(
        "config/multigh/accounts.conf",
        include_str!("../../example/accounts.conf"),
    );
    sandbox.ok("mgh", &["setup"]);

    for alias in ["personal", "school", "work"] {
        assert!(
            sandbox
                .path(&format!("state/multigh/identities/git-{alias}.conf"))
                .is_file()
        );
    }
}
