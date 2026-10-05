use crate::support::Sandbox;
use std::fs;

#[test]
fn init_emits_each_shell_handler_without_needing_identities() {
    let sandbox = Sandbox::new();

    fs::remove_file(sandbox.path("config/multigh/identities.jsonc")).unwrap();

    for shell in ["fish", "bash", "zsh"] {
        let script = sandbox.ok("mgh", &["shell", "init", shell]);

        assert!(script.contains("__mgh_enter") && script.contains("command mgh internal enter"));

        let path = sandbox.path(&format!("init.{shell}"));

        fs::write(&path, &script).unwrap();
        sandbox.ok(shell, &["-n", path.to_str().unwrap()]);
    }

    assert!(!sandbox.path("gh-calls").exists());
    assert!(!sandbox.path("state/multigh").exists());
}
