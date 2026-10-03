use crate::support::{IDENTITIES, Sandbox};
use serde_json::Value;
use std::fs;

#[test]
fn comments_and_trailing_commas_work_with_identity_protections_without_rewriting_config() {
    let sandbox = Sandbox::new();
    let configuration = format!(
        "// Configured identities\n/* Commit details below */\n{}",
        IDENTITIES
            .replace("\"personal\": {", "\"PeRsOnAl\": { // Personal identity")
            .replace(
                "\"email\": \"alice@example.com\",",
                "\"email\": \"alice@example.com\", /* Primary email */"
            )
            .replace(
                "\"123+alice@users.noreply.github.com\"",
                "\"123+alice@users.noreply.github.com\","
            )
            .replace("\n            ]", "\n            ],")
            .replace("\n        }", "\n        },")
            .replace("\n    }\n", "\n    },\n")
    );

    sandbox.write("config/multigh/identities.jsonc", &configuration);
    sandbox.ok("mgh", &["setup"]);
    sandbox.ok("mgh", &["protections", "--allow", "PERSONAL"]);
    sandbox.ok("mgh", &["check"]);
    sandbox.commit();
    sandbox.ok(
        "git",
        &["config", "user.email", "123+alice@users.noreply.github.com"],
    );
    sandbox.commit();

    assert_eq!(
        configuration,
        fs::read_to_string(sandbox.path("config/multigh/identities.jsonc")).unwrap()
    );
}

#[test]
fn adding_an_identity_preserves_crlf_comments_indentation_and_escaped_commit_names() {
    let sandbox = Sandbox::new();
    let original = r#"/* Identity configuration */
{
  "personal": {
    "username": "alice", // GitHub username
    "commit": {"name": "Alice Example", "email": "alice@example.com"},
  }, // Existing identity
} // End of config
"#
    .replace('\n', "\r\n");
    let name = r#"Carol "Example" \ Team /* text */ // notes é"#;

    sandbox.write("config/multigh/identities.jsonc", &original);
    sandbox.write(
        "accounts-json",
        r#"[{"login":"carol","active":true,"state":"success"}]"#,
    );
    sandbox.ok(
        "mgh",
        &[
            "new",
            "WORK",
            "--username",
            "carol",
            "--email",
            "carol@example.com",
            "--name",
            name,
        ],
    );

    let saved = fs::read_to_string(sandbox.path("config/multigh/identities.jsonc")).unwrap();
    let parsed: Value = jsonc_parser::parse_to_serde_value(&saved, &Default::default()).unwrap();

    assert!(saved.starts_with("/* Identity configuration */\r\n{"));
    assert!(saved.contains("// GitHub username") && saved.contains("// Existing identity"));
    assert!(saved.ends_with("} // End of config\r\n"));
    assert!(saved.contains("\r\n  \"work\": {"));
    assert!(!saved.replace("\r\n", "").contains('\n'));
    assert_eq!(parsed["personal"]["commit"]["name"], "Alice Example");
    assert_eq!(parsed["work"]["commit"]["name"], name);
    assert_eq!(
        sandbox
            .ok("git", &["config", "--global", "user.name"])
            .trim(),
        name
    );
}
