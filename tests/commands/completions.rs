use crate::support::Sandbox;
use std::fs;

#[test]
fn all_completion_targets_work_without_configuration_or_authentication() {
    let sandbox = Sandbox::new();

    fs::remove_file(sandbox.path("config/multigh/identities.jsonc")).unwrap();

    for shell in ["fish", "bash", "zsh", "powershell", "elvish"] {
        let text = sandbox.ok("mgh", &["completions", shell]);

        assert!(
            !text.is_empty() && text.contains("mgh") && text.contains("new"),
            "{shell}"
        );
    }

    assert!(!sandbox.path("gh-calls").exists());
    assert!(!sandbox.path("state/multigh").exists());
}
