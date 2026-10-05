use crate::support::{Sandbox, terminal};
use std::process::Command;

const SHELLS: &[(&str, &[&str], &str)] = &[
    (
        "fish",
        &["--no-config", "-ic"],
        "mgh shell init fish | source",
    ),
    (
        "bash",
        &["--noprofile", "--norc", "-ic"],
        "eval \"$(mgh shell init bash)\"",
    ),
    ("zsh", &["-fic"], "eval \"$(mgh shell init zsh)\""),
];

fn prepared() -> Sandbox {
    let sandbox = Sandbox::new();

    sandbox.protect();
    sandbox.ok("mgh", &["settings", "verbose", "off"]);

    sandbox
}

fn shell(sandbox: &Sandbox, shell: &str, flags: &[&str], script: &str) -> Command {
    let mut command = sandbox.command(shell);

    command
        .current_dir(sandbox.path("home"))
        .args(flags)
        .arg(script);

    command
}

fn allowed(sandbox: &Sandbox, directory: &str) -> String {
    let output = sandbox
        .command("git")
        .current_dir(sandbox.path(directory))
        .args(["config", "--local", "--get-all", "mgh.allowed-identity"])
        .output()
        .unwrap();

    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn git_init_prompts_in_all_supported_shells_and_saves_multiple_identities() {
    for (name, flags, init) in SHELLS {
        let sandbox = prepared();
        let script = format!("{init}; git init 'new repo'");
        let (status, output) = terminal(
            &mut shell(&sandbox, name, flags, &script),
            &[("Which identities may use this repository?", b" \x1b[B \r")],
        );

        assert!(status.success(), "{name}: {output}");
        assert_eq!(allowed(&sandbox, "home/new repo"), "personal\nschool\n");
        assert_eq!(
            std::fs::read_to_string(sandbox.path("active")).unwrap(),
            "alice\n"
        );

        let check = sandbox
            .command("mgh")
            .current_dir(sandbox.path("home/new repo"))
            .args(["repo", "check"])
            .output()
            .unwrap();

        assert!(check.status.success(), "{output}");
    }
}

#[test]
fn init_directory_and_git_options_target_the_initialized_repository() {
    for (args, directory) in [
        ("init", "home"),
        ("init -q -b custom 'new repo'", "home/new repo"),
        (
            "-C .. -C home init --initial-branch custom -- new",
            "home/new",
        ),
        ("init --bare bare", "home/bare"),
        (
            "init --separate-git-dir ../metadata separate",
            "home/separate",
        ),
        (
            "init --object-format sha1 --template '' template",
            "home/template",
        ),
        ("init --shared=group shared", "home/shared"),
        ("init --initial custom abbreviated", "home/abbreviated"),
        (
            "-C .. -C home -c user.name=Temporary init -bmain attached",
            "home/attached",
        ),
        (
            "init -- 'repo with a '\"'\"'quote'",
            "home/repo with a 'quote",
        ),
        ("--bare init global-bare", "home/global-bare"),
        ("--git-dir custom.git init", "home"),
    ] {
        let sandbox = prepared();
        let script = format!("eval \"$(mgh shell init zsh)\"; git {args}");
        let (status, output) = terminal(
            &mut shell(&sandbox, "zsh", &["-fic"], &script),
            &[("Which identities may use this repository?", b" \r")],
        );

        assert!(status.success(), "{args}: {output}");

        if args.starts_with("--git-dir") {
            assert!(
                std::fs::read_to_string(sandbox.path("home/custom.git/config"))
                    .unwrap()
                    .contains("allowed-identity = personal")
            );
        } else {
            assert_eq!(
                allowed(&sandbox, directory),
                "personal\n",
                "{args}: {output}"
            );
        }

        assert_eq!(allowed(&sandbox, "repo"), "personal\n");
    }
}

#[test]
fn reinitialization_preserves_existing_permissions_without_another_picker() {
    for (name, flags, init) in SHELLS {
        let sandbox = prepared();
        let script = format!("{init}; git init new; git init new");
        let (status, output) = terminal(
            &mut shell(&sandbox, name, flags, &script),
            &[("Which identities may use this repository?", b" \r")],
        );

        assert!(status.success(), "{name}: {output}");
        assert_eq!(output.matches("Allowed identities saved").count(), 1);
        assert_eq!(allowed(&sandbox, "home/new"), "personal\n");
    }
}

#[test]
fn canceled_picker_keeps_git_successful_and_commit_protection_active() {
    for (name, flags, init) in SHELLS {
        let sandbox = prepared();
        let script = format!("{init}; git init new");
        let (status, output) = terminal(
            &mut shell(&sandbox, name, flags, &script),
            &[("Which identities may use this repository?", b"\x1b")],
        );

        assert!(status.success(), "{name}: {output}");
        assert!(
            output.contains("commits and pushes remain blocked"),
            "{output}"
        );
        assert!(allowed(&sandbox, "home/new").is_empty());

        let commit = sandbox
            .command("git")
            .current_dir(sandbox.path("home/new"))
            .args(["commit", "--allow-empty", "-m", "test"])
            .output()
            .unwrap();

        assert!(!commit.status.success());
        assert!(String::from_utf8_lossy(&commit.stderr).contains("No identities are authorized"));
    }
}

#[test]
fn protections_off_and_command_config_overrides_are_respected() {
    for args in ["init new", "-c core.hooksPath=custom init new"] {
        let sandbox = prepared();

        if args == "init new" {
            sandbox.ok("git", &["init", "../home/new"]);
            sandbox.ok(
                "git",
                &["-C", "../home/new", "config", "mgh.protections", "false"],
            );
        }

        let script = format!("eval \"$(mgh shell init zsh)\"; git {args}");
        let replies: &[(&str, &[u8])] = if args == "init new" {
            &[]
        } else {
            &[("Which identities may use this repository?", b" \r")]
        };
        let (status, output) = terminal(&mut shell(&sandbox, "zsh", &["-fic"], &script), replies);

        assert!(status.success(), "{output}");
        assert_eq!(
            output.contains("Which identities may use this repository?"),
            !replies.is_empty()
        );

        if args.starts_with("-c") {
            assert!(
                output.contains("Identity protections are not active"),
                "{output}"
            );
        } else {
            assert!(allowed(&sandbox, "home/new").is_empty());
        }
    }
}

#[test]
fn git_failure_status_is_preserved_without_prompting() {
    for (name, flags, init) in SHELLS {
        let sandbox = prepared();
        let native = sandbox.run("git", &["init", "--invalid-option"]);
        let script = format!("{init}; git init --invalid-option");
        let (status, output) = terminal(&mut shell(&sandbox, name, flags, &script), &[]);

        assert_eq!(status.code(), native.status.code(), "{name}: {output}");
        assert!(!output.contains("Which identities"), "{output}");
    }
}

#[test]
fn noninteractive_init_and_non_init_commands_do_not_prompt_or_change_policy() {
    for (name, flags, init) in SHELLS {
        let sandbox = prepared();
        let script = format!("{init}; git init new");
        let output = shell(&sandbox, name, flags, &script).output().unwrap();

        assert!(output.status.success());
        assert!(!String::from_utf8_lossy(&output.stdout).contains("Which identities"));
        assert!(allowed(&sandbox, "home/new").is_empty());

        let script = format!("{init}; git -c alias.say='!printf init' say; git --version");
        let (status, output) = terminal(&mut shell(&sandbox, name, flags, &script), &[]);

        assert!(status.success(), "{output}");
        assert!(output.contains("initgit version"), "{output}");
        assert!(!output.contains("Which identities"), "{output}");
        assert_eq!(allowed(&sandbox, "repo"), "personal\n");
    }
}

#[test]
fn repeated_shell_initialization_preserves_existing_git_functions() {
    for (name, flags, init) in SHELLS {
        let sandbox = prepared();
        let function = if *name == "fish" {
            "function git; printf existing; end"
        } else {
            "git() { printf existing; }"
        };
        let script = format!("{function}; {init}; {init}; git init new");
        let (status, output) = terminal(&mut shell(&sandbox, name, flags, &script), &[]);

        assert!(status.success(), "{name}: {output}");
        assert!(output.contains("existing"), "{output}");
        assert!(!sandbox.path("home/new").exists());
    }
}

#[test]
fn existing_git_aliases_are_preserved_without_breaking_shell_initialization() {
    for (name, flags, init) in SHELLS {
        let sandbox = prepared();
        let script = format!("alias git='printf existing'; {init}; {init}; eval 'git init new'");
        let (status, output) = terminal(&mut shell(&sandbox, name, flags, &script), &[]);

        assert!(status.success(), "{name}: {output}");
        assert!(output.contains("existing"), "{output}");
        assert!(!sandbox.path("home/new").exists());
    }
}

#[test]
fn git_information_flags_with_init_as_a_trailing_word_do_not_authorize_a_repo() {
    let sandbox = prepared();
    let script = "eval \"$(mgh shell init zsh)\"; git --html-path init; git --exec-path init";
    let (status, output) = terminal(&mut shell(&sandbox, "zsh", &["-fic"], script), &[]);

    assert!(status.success(), "{output}");
    assert!(!output.contains("Which identities"), "{output}");
    assert!(!sandbox.path("home/.git").exists());
}
