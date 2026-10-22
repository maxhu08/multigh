//! Report layout, semantic colors, plain help, native streams and usable terminal forms.

use super::support::*;
use serde_json::json;
use std::fs;

/// Full authentication output must preserve streams and permitted colors while normalizing only
/// outer report spacing.
#[test]
fn full_authentication_preserves_color_control_and_streams() {
    let env = Sandbox::new();
    env.setup_defaults();
    let o = env
        .mgh(&["status", "--full"])
        .env("CLICOLOR", "0")
        .output()
        .unwrap();
    assert_ok(&o);
    assert!(out(&o).ends_with("\n\n  native auth stdout\n\n"));
    assert_eq!(err(&o), "\n  native auth stderr\n\n");
    assert!(!out(&o).contains('\u{1b}'));
    let spaced = env
        .mgh(&["status", "--full"])
        .env("MGH_NATIVE_LAYOUT", "1")
        .output()
        .unwrap();
    assert_ok(&spaced);
    assert!(out(&spaced).ends_with("\n\n  first auth line\n\n    last auth line\n\n"));
    assert!(!out(&spaced).contains("\n\n\n"));
    assert_eq!(err(&spaced), "\n  native warning without final newline\n\n");
    env.driver();
    for disabled in [false, true] {
        let mut command = env.command("python3");
        command
            .arg(env.root.join("driver.py"))
            .arg(env!("CARGO_BIN_EXE_mgh"))
            .args(["status", "--full"])
            .env("MGH_PTY_STEPS", "[]")
            .env("MGH_NATIVE_STYLE", "1")
            .env_remove("NO_COLOR")
            .env("TERM", "xterm-256color");
        if disabled {
            command.env("CLICOLOR", "0").env("CLICOLOR_FORCE", "1");
        }
        let o = command.output().unwrap();
        assert_ok(&o);
        let text = out(&o);
        assert_eq!(
            text.contains("\r\n  \x1b[31mforced native color\x1b[0m\r\n"),
            !disabled,
            "{text}"
        );
        assert_eq!(
            text.contains("\r\n  \x1b[32mnative stderr color\x1b[0m\r\n"),
            !disabled,
            "{text}"
        );
        assert!(text.contains("\r\n  native auth stdout\r\n\r\n"), "{text}");
        assert!(text.ends_with("  native auth stderr\r\n\r\n"), "{text}");
    }
    let o = env
        .mgh(&["status", "--full"])
        .env("MGH_FULL_FAIL", "1")
        .output()
        .unwrap();
    assert!(!o.status.success());
    assert_eq!(
        err(&o),
        "\n  native auth stderr\n\n  GitHub authentication check failed\n\n"
    );
}

/// Headings, multiline paths and diagnostics must share indentation without making newline-
/// containing paths unusable.
#[test]
fn report_sections_and_multiline_text_share_indentation() {
    let mut env = Sandbox::new();
    env.config = env.config.with_file_name("identities\ncontinued.jsonc");
    env.write_identities();
    let config_before = bytes(&env.config);
    let auth_before = bytes(env.root.join("auth.json"));
    let config = env.config.to_str().unwrap();
    let text = out(&env.success(&["--config", config, "identity"]));
    let lines = text.lines().collect::<Vec<_>>();
    let config_line = lines
        .iter()
        .position(|line| line.starts_with("    Identity config"))
        .unwrap();
    assert_eq!(
        lines[config_line + 1],
        format!("{}continued.jsonc", " ".repeat(30))
    );
    assert_eq!(bytes(&env.config), config_before);
    assert_eq!(bytes(env.root.join("auth.json")), auth_before);
    assert!(!env.state.exists());

    env.init_repo();
    let protections = out(&env.success(&["repo", "protections", "off"]));
    assert!(
        protections.starts_with("\n  Repository protections\n"),
        "{protections}"
    );
    assert!(
        protections
            .lines()
            .any(|line| line.starts_with("    Protection"))
    );
    let edited = out(&env.success(&[
        "--config",
        config,
        "identity",
        "edit",
        "personal",
        "--name",
        "Alice Revised",
    ]));
    assert!(
        edited.starts_with("\n  Identity updated [personal]\n"),
        "{edited}"
    );
    assert!(edited.contains("\n    continued.jsonc\n"), "{edited}");
    assert!(
        fs::read_to_string(env.home.join(".gitconfig"))
            .unwrap()
            .contains("identities\\ncontinued.jsonc identity edit personal"),
        "{edited}"
    );
    assert_eq!(
        env.git_without_hooks(&[
            "config",
            "--file",
            env.state
                .join("identities/git-personal.conf")
                .to_str()
                .unwrap(),
            "user.name"
        ]),
        "Alice Revised"
    );
    let removed = out(&env.success(&["--config", config, "identity", "remove", "work"]));
    assert!(removed.starts_with("\n  Identity removed\n"), "{removed}");

    let env = Sandbox::new();
    let failure = env.rejects(&["setup"], "Setup needs a first identity");
    let diagnostic = err(&failure);
    assert!(diagnostic.lines().filter(|line| !line.is_empty()).count() >= 2);
    assert!(
        diagnostic
            .lines()
            .filter(|line| !line.is_empty())
            .all(|line| line.starts_with("  "))
    );
    assert!(!env.config.exists());
    assert!(!env.state.exists());
}

/// Reports need consistent blank padding, aligned values, distinct identity blocks and the
/// correct semantic colors.
#[test]
fn presentation_padding_alignment_before_after_and_semantic_colors() {
    let env = Sandbox::new();
    let o = env.success(&["settings"]);
    let text = out(&o);
    assert!(text.starts_with('\n'));
    assert!(text.ends_with("\n\n"));
    assert!(!text.contains("\n\n\n"));
    assert!(text.starts_with("\n  Global settings\n"), "{text}");
    assert!(!text.contains("(global)"), "{text}");
    for line in text.lines().filter(|l| l.starts_with("    ")) {
        assert_eq!(&line[30..], "OFF");
        assert!(line.starts_with("    "));
    }
    env.setup_defaults();
    env.init_repo();
    let allowed = out(&env.success(&["repo", "allowed"]));
    assert_eq!(
        allowed.matches("Allowed identities").count(),
        1,
        "{allowed}"
    );
    assert!(allowed.contains("none selected"), "{allowed}");
    env.success(&["repo", "allowed", "add", "personal"]);
    let permitted = out(&env.success(&["repo", "allowed"]));
    assert!(permitted.contains("GitHub username"), "{permitted}");
    assert!(permitted.contains("alice"), "{permitted}");
    for command in [vec!["identity"], vec!["status"]] {
        let text = out(&env.success(&command));
        let lines = text.lines().collect::<Vec<_>>();
        for heading in ["    personal (Active)", "    work"] {
            let index = lines
                .iter()
                .position(|line| *line == heading)
                .unwrap_or_else(|| panic!("missing identity heading {heading:?}: {text}"));
            assert_eq!(lines[index - 1], "", "{text}");
            assert!(
                lines[index + 1].starts_with("      GitHub username"),
                "{text}"
            );
            assert_eq!(
                text.lines().filter(|line| *line == heading).count(),
                1,
                "{text}"
            );
        }
        assert!(
            !text.lines().any(|line| line.starts_with("    Identity  ")),
            "{text}"
        );
        assert!(!text.contains("\n\n\n"), "{text}");
        assert!(!text.contains("Active GitHub account"), "{text}");
        assert!(text.starts_with("\n  Identities\n"), "{text}");
        assert_eq!(text.matches("    Identity config ").count(), 1, "{text}");
        assert_eq!(text.matches("(Active)").count(), 1, "{text}");
        if command == ["status"] {
            assert!(text.starts_with("\n  Identities\n"), "{text}");
            assert!(text.contains("\n\n  Repository\n"), "{text}");
            assert!(!text.lines().any(|line| line == "Accounts"), "{text}");
            assert!(!text.contains("Autoswitch"), "{text}");
            assert!(!text.contains("(global)"), "{text}");
            assert!(
                text.contains("\n\n    Identity and commit details match\n"),
                "{text}"
            );
        }
    }
    let repo_only = out(&env.success(&["repo"]));
    assert!(repo_only.contains("Autoswitch (global)"), "{repo_only}");
    assert!(
        repo_only.contains("\n\n    Identity and commit details match\n"),
        "{repo_only}"
    );
    let o = env.success(&["switch", "personal"]);
    assert!(out(&o).contains('→'));
    let o = env.success(&["switch", "personal"]);
    assert!(!out(&o).contains('→'));
    env.driver();
    let exe = env!("CARGO_BIN_EXE_mgh");
    let o = env
        .command("python3")
        .arg(env.root.join("driver.py"))
        .arg(exe)
        .arg("identity")
        .env("MGH_PTY_STEPS", "[]")
        .env_remove("NO_COLOR")
        .env("TERM", "xterm-256color")
        .output()
        .unwrap();
    assert_ok(&o);
    let text = out(&o);
    for rgb in ["103;232;249", "192;132;252", "251;146;60", "148;163;184"] {
        assert!(text.contains(rgb), "missing {rgb}: {text}");
    }
    env.success(&["settings", "welcome", "on"]);
    env.success(&["repo", "protections", "off"]);
    for args in [vec!["settings"], vec!["repo"], vec!["repo", "protections"]] {
        let o = env
            .command("python3")
            .arg(env.root.join("driver.py"))
            .arg(exe)
            .args(&args)
            .env("MGH_PTY_STEPS", "[]")
            .env_remove("NO_COLOR")
            .env("TERM", "xterm-256color")
            .output()
            .unwrap();
        assert_ok(&o);
        let text = out(&o);
        assert!(
            text.contains("\x1b[1;38;2;251;113;133mOFF\x1b[0m"),
            "{args:?}: {text}"
        );
        if args == ["settings"] {
            assert!(text.contains("\x1b[1;38;2;134;239;172mON\x1b[0m"), "{text}");
        }
    }
    let o = env
        .command("python3")
        .arg(env.root.join("driver.py"))
        .arg(exe)
        .arg("settings")
        .env("MGH_PTY_STEPS", "[]")
        .env("TERM", "dumb")
        .env_remove("NO_COLOR")
        .output()
        .unwrap();
    assert_ok(&o);
    assert!(!out(&o).contains("\u{1b}["));
}

/// Identity and value colors must survive mixed-color messages, while every mgh-owned arrow
/// remains unstyled.
#[test]
fn semantic_values_keep_their_colors_and_arrows_are_plain() {
    let env = Sandbox::new();
    env.driver();
    let terminal = |args: &[&str]| {
        env.command("python3")
            .arg(env.root.join("driver.py"))
            .arg(env!("CARGO_BIN_EXE_mgh"))
            .args(args)
            .env("MGH_PTY_STEPS", "[]")
            .env_remove("NO_COLOR")
            .env("TERM", "xterm-256color")
            .output()
            .unwrap()
    };
    let cyan = "\x1b[1;38;2;103;232;249m";
    let purple = "\x1b[22;38;2;192;132;252m";
    let orange = "\x1b[22;38;2;251;146;60m";
    let slate = "\x1b[22;38;2;148;163;184m";
    let reset = "\x1b[0m";
    let plain_arrows = |text: &str| {
        let mut count = 0;
        for (offset, _) in text.match_indices('→') {
            let (_, last_style) = text[..offset].rsplit_once("\x1b[").unwrap();
            assert_eq!(
                last_style.split('m').next(),
                Some("0"),
                "colored arrow: {text}"
            );
            count += 1;
        }
        assert!(count > 0, "missing arrows: {text}");
    };

    let created = terminal(&[
        "identity",
        "new",
        "personal",
        "--username",
        "alice",
        "--name",
        "Alice Example",
        "--email",
        "Alice@example.com",
    ]);
    assert_ok(&created);
    let text = out(&created);
    assert!(text.contains(&format!("{cyan}personal{reset}")), "{text}");
    assert!(
        text.contains(&format!("{slate}{}{reset}", env.config.display())),
        "{text}"
    );
    plain_arrows(&text);

    env.init_repo();
    env.setup_defaults();
    env.success(&["repo", "allowed", "add", "work"]);
    let permission_before =
        env.git_without_hooks(&["config", "--local", "--get-all", "mgh.allowed-identity"]);
    let switched = terminal(&["settings", "autoswitch", "on"]);
    assert_ok(&switched);
    let text = out(&switched);
    assert!(
        text.contains(&format!("{cyan}personal{reset} → {cyan}work{reset}")),
        "{text}"
    );
    assert!(
        text.contains(&format!("{purple}alice {reset}→{purple} alice-work{reset}")),
        "{text}"
    );
    assert!(
        text.contains(&format!(
            "{orange}Alice Example {reset}→{orange} Alice Work{reset}"
        )),
        "{text}"
    );
    plain_arrows(&text);
    assert!(!text.contains("mgh internal"), "{text}");
    assert_eq!(env.accounts()["selected"], "alice-work");
    assert_eq!(
        env.git_without_hooks(&["config", "--global", "user.name"]),
        "Alice Work"
    );
    assert_eq!(
        env.git_without_hooks(&["config", "--local", "--get-all", "mgh.allowed-identity"]),
        permission_before
    );

    let manual = terminal(&["switch", "personal"]);
    assert_ok(&manual);
    let text = out(&manual);
    assert!(text.contains(&format!("{cyan}personal{reset}")), "{text}");
    plain_arrows(&text);
    let global_before = bytes(env.home.join(".gitconfig"));
    let account_before = bytes(env.root.join("auth.json"));
    let rejected = terminal(&["switch", "unknown→identity"]);
    assert_eq!(rejected.status.code(), Some(1));
    plain_arrows(&out(&rejected));
    assert_eq!(bytes(env.home.join(".gitconfig")), global_before);
    assert_eq!(bytes(env.root.join("auth.json")), account_before);

    for (term, no_color) in [("xterm-256color", true), ("dumb", false)] {
        env.success(&["switch", "personal"]);
        let mut command = env.command("python3");
        command
            .arg(env.root.join("driver.py"))
            .arg(env!("CARGO_BIN_EXE_mgh"))
            .args(["internal", "enter"])
            .env("MGH_PTY_STEPS", "[]")
            .env("TERM", term);
        if !no_color {
            command.env_remove("NO_COLOR");
        }
        let result = command.output().unwrap();
        assert_ok(&result);
        let text = out(&result);
        assert!(text.contains("Autoswitch [personal → work]"), "{text}");
        assert!(!text.contains('\x1b'), "{text}");
    }

    let edited = terminal(&["identity", "edit", "personal", "--name", "Alice Revised"]);
    assert_ok(&edited);
    assert!(out(&edited).contains(&format!("{cyan}personal{reset}")));
    assert!(out(&edited).contains(&format!("{slate}{}{reset}", env.config.display())));
    env.success(&["settings", "welcome", "on"]);
    env.set_local("user.name", "Mismatch");
    let welcome = terminal(&["internal", "welcome"]);
    assert_ok(&welcome);
    assert!(out(&welcome).contains(&format!(
        "Commit details need: mgh switch {reset}{cyan}work{reset}"
    )));
    let removed = terminal(&["identity", "remove", "personal"]);
    assert_ok(&removed);
    assert!(out(&removed).contains(&format!("Removed identity {reset}{cyan}personal{reset}")));
    env.set_local("mgh.allowed-identity", "stale");
    let stale = terminal(&["repo"]);
    assert_ok(&stale);
    assert!(out(&stale).contains(&format!("Unresolved permission '{reset}{cyan}stale{reset}")));
}

/// Forms must require terminal stdin/stdout/stderr and color fields consistently instead of
/// hanging on redirected input.
#[test]
fn forms_require_all_three_terminal_streams_and_report_field_colors() {
    let env = Sandbox::new();
    env.driver();
    let exe = env!("CARGO_BIN_EXE_mgh");
    let steps = json!([
        ["Identity name", "personal\r"],
        ["GitHub username", "alice\r"],
        ["Commit name", "\r"],
        ["Commit email", "\u{1b}"]
    ]);
    let o = env
        .command("python3")
        .arg(env.root.join("driver.py"))
        .arg(exe)
        .args(["identity", "new"])
        .env("MGH_PTY_STEPS", steps.to_string())
        .env_remove("NO_COLOR")
        .env("TERM", "xterm-256color")
        .output()
        .unwrap();
    assert!(!o.status.success());
    let text = out(&o);
    for color in ["103;232;249", "192;132;252", "251;146;60"] {
        assert!(text.contains(color), "{color}: {text}");
    }
    assert!(!env.config.exists());
    for redirected in ["stdin", "stdout", "stderr"] {
        let output = env.root.join(format!("redirected-{redirected}"));
        let command = match redirected {
            "stdin" => "mgh identity new < /dev/null; exit 0\n".to_string(),
            "stdout" => format!(
                "mgh identity new > {}; exit 0\n",
                shquote(output.to_str().unwrap())
            ),
            _ => format!(
                "mgh identity new 2> {}; exit 0\n",
                shquote(output.to_str().unwrap())
            ),
        };
        let o = env.pty(
            "bash",
            &["--noprofile", "--norc", "-i"],
            json!([["", command]]),
        );
        assert_ok(&o);
        let content = if redirected == "stderr" {
            fs::read_to_string(output).unwrap()
        } else {
            out(&o)
        };
        assert!(
            content.contains("Identity name is required"),
            "{redirected}: {content}"
        );
        assert!(!env.config.exists());
    }
}

/// Redirected authentication reports must suppress child ANSI color even when force-color
/// variables are set.
#[test]
fn redirected_full_native_report_disables_forced_child_color() {
    let env = Sandbox::new();
    env.setup_defaults();
    let o = env
        .mgh(&["status", "--full"])
        .env_remove("NO_COLOR")
        .env("TERM", "xterm-256color")
        .env("GH_FORCE_TTY", "1")
        .env("MGH_NATIVE_STYLE", "1")
        .output()
        .unwrap();
    assert_ok(&o);
    assert!(!out(&o).contains('\u{1b}'));
    assert!(!err(&o).contains('\u{1b}'));
}
