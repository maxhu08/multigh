use crate::support::{IDENTITIES, Sandbox};
use serde_json::{Value, json};
use std::fs;

#[test]
fn malformed_configuration_is_rejected_before_git_settings_change() {
    let sandbox = Sandbox::new();
    let before = fs::read(sandbox.path("gitconfig")).unwrap();
    let mut malformed = vec![
        "".to_owned(),
        "{".to_owned(),
        "[]".to_owned(),
        "{}".to_owned(),
        "// Only comments".to_owned(),
        "/* Unterminated comment".to_owned(),
        r#"{personal: {"username": "alice", "commit": {"email": "alice@example.com"}}}"#.to_owned(),
        r#"{'personal': {"username": "alice", "commit": {"email": "alice@example.com"}}}"#
            .to_owned(),
        IDENTITIES.replacen("\"username\":", "\"username\"", 1),
        IDENTITIES.replacen("\"alice\",", "\"alice\"", 1),
        format!("{IDENTITIES} {{}}"),
        IDENTITIES.replace("\"personal\"", "\"1personal\""),
        IDENTITIES.replace("\"personal\"", "\"bad.name\""),
        IDENTITIES.replace("\"personal\"", "\"_Comment\""),
        IDENTITIES.replace("additional_emails", "additional_email"),
        IDENTITIES.replacen(
            "\"username\":",
            "\"username\": \"duplicate\", \"username\":",
            1,
        ),
        IDENTITIES.replacen("\"commit\":", "\"commit\": {}, \"commit\":", 1),
        IDENTITIES.replacen(
            "\"email\":",
            "\"email\": \"duplicate@example.com\", \"email\":",
            1,
        ),
        IDENTITIES.replacen("\"name\":", "\"name\": \"Duplicate\", \"name\":", 1),
        IDENTITIES.replacen(
            "\"additional_emails\":",
            "\"additional_emails\": [], \"additional_emails\":",
            1,
        ),
    ];

    for key in ["personal", "PERSONAL"] {
        malformed.push(IDENTITIES.replacen(
            "{",
            &format!(
                r#"{{"{key}": {{"username":"charlie","commit":{{"email":"charlie@example.com"}}}},"#
            ),
            1,
        ));
    }

    for (pointer, value) in [
        ("/personal/username", json!("-alice")),
        ("/personal/username", json!("alice_name")),
        ("/personal/username", json!("")),
        ("/personal/username", json!(42)),
        ("/personal/commit/name", json!("")),
        ("/personal/commit/name", json!("Alice <Example>")),
        ("/personal/commit/name", json!("Alice\tExample")),
        ("/personal/commit/name", json!([])),
        ("/personal/commit/email", json!("")),
        ("/personal/commit/email", json!("alice@@example.com")),
        ("/personal/commit/email", json!("alice@")),
        ("/personal/commit/email", json!("alice example@example.com")),
        ("/personal/commit/email", Value::Null),
        ("/personal/commit/additional_emails", json!("not-an-array")),
        ("/personal/commit/additional_emails", json!([42])),
        ("/personal/commit/additional_emails", json!([""])),
        (
            "/personal/commit/additional_emails/0",
            json!("not-an-email"),
        ),
        ("/school/username", json!("ALICE")),
        ("/school/commit/email", json!("ALICE@example.com")),
        (
            "/school/commit/additional_emails/0",
            json!("123+ALICE@users.noreply.github.com"),
        ),
        (
            "/personal/commit",
            json!({"email":"alice@example.com", "unknown":true}),
        ),
        (
            "/personal",
            json!({"username":"alice", "commit":{"email":"alice@example.com"}, "unknown":true}),
        ),
        ("/personal", json!({"commit":{"email":"alice@example.com"}})),
        ("/personal", json!({"username":"alice"})),
        ("/personal/commit", json!({"name":"Alice"})),
    ] {
        let mut document: Value = serde_json::from_str(IDENTITIES).unwrap();

        *document.pointer_mut(pointer).unwrap() = value;
        malformed.push(document.to_string());
    }

    for configuration in malformed {
        sandbox.write("config/multigh/identities.jsonc", &configuration);

        assert!(
            !sandbox.run("mgh", &["setup"]).status.success(),
            "Accepted config: {configuration}"
        );
        assert_eq!(before, fs::read(sandbox.path("gitconfig")).unwrap());
        assert!(!sandbox.path("state/multigh/identities").exists());
    }
}

#[test]
fn optional_commit_names_custom_identity_names_and_the_complete_example_are_accepted() {
    let sandbox = Sandbox::new();

    sandbox.write(
        "config/multigh/identities.jsonc",
        r#"{"My_Account-2":{"username":"alice","commit":{"email":"alice@example.com","additional_emails":["123+alice@users.noreply.github.com"]}}}"#,
    );
    sandbox.ok("mgh", &["setup"]);
    sandbox.ok("mgh", &["protections", "--allow", "MY_ACCOUNT-2"]);

    assert_eq!(sandbox.ok("git", &["config", "user.name"]).trim(), "alice");

    sandbox.commit();
    sandbox.write(
        "config/multigh/identities.jsonc",
        include_str!("../../examples/identities.jsonc"),
    );
    sandbox.ok("mgh", &["setup"]);

    for identity_name in ["personal", "school", "work"] {
        assert!(
            sandbox
                .path(&format!(
                    "state/multigh/identities/git-{identity_name}.conf"
                ))
                .is_file()
        );
    }
}
