use crate::support::{Sandbox, terminal};

#[test]
fn forms_with_redirected_stderr_reject_interaction_without_changing_settings() {
    let sandbox = Sandbox::new();
    let before = std::fs::read(sandbox.path("config/multigh/identities.jsonc")).unwrap();

    for (command, message) in [
        ("identity new", "requires a terminal"),
        ("repo allowed update", "needs an interactive terminal"),
    ] {
        let (status, output) = terminal(
            sandbox
                .command("sh")
                .args(["-c", &format!("exec mgh {command} 2> ../form-error")]),
            &[],
        );

        assert!(!status.success(), "{output}");
        assert!(
            std::fs::read_to_string(sandbox.path("form-error"))
                .unwrap()
                .contains(message)
        );
        assert_eq!(
            before,
            std::fs::read(sandbox.path("config/multigh/identities.jsonc")).unwrap()
        );
        assert!(!sandbox.path("gh-calls").exists());
    }

    assert!(
        !sandbox
            .run("git", &["config", "--get-all", "mgh.allowed-identity"])
            .status
            .success()
    );
}
