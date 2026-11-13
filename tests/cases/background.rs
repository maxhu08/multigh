//! Background entry ordering and real-shell responsiveness with explicitly paused dependencies.

use super::support::*;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

fn prepared() -> Sandbox {
    let env = Sandbox::new();
    env.init_repo();
    env.setup_defaults();
    env.success(&["repo", "allowed", "add", "work"]);
    env.success(&["settings", "autoswitch", "on"]);
    env.success(&["switch", "personal"]);
    assert_ok(
        &env.mgh(&["settings", "autoswitch", "background"])
            .current_dir(&env.home)
            .output()
            .unwrap(),
    );
    fs::write(env.root.join("gh.log"), "").unwrap();
    env
}

fn repository(env: &Sandbox, name: &str, identity: &str) -> PathBuf {
    let path = env.root.join(name);
    fs::create_dir(&path).unwrap();
    env.git_without_hooks(&["-C", path.to_str().unwrap(), "init", "-b", "main"]);
    assert_ok(
        &env.mgh(&["repo", "allowed", "add", identity])
            .current_dir(&path)
            .output()
            .unwrap(),
    );
    path
}

fn queue(env: &Sandbox) -> Value {
    serde_json::from_slice(&bytes(env.state.join("background/queue.json"))).unwrap()
}

fn wait_until(mut ready: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !ready() {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for test dependency"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn release(gate: &Path) {
    fs::write(gate.with_extension("release"), "").unwrap();
}

fn entry(env: &Sandbox, path: &Path, session: &str) -> Command {
    let mut command = env.mgh(&["internal", "shell-enter"]);
    command.current_dir(path).env("MGH_SHELL_SESSION", session);
    command
}

fn barrier(env: &Sandbox) {
    env.success(&["internal", "git", "--", "--version"]);
}

fn poll(env: &Sandbox, session: &str) -> String {
    let output = env
        .mgh(&["internal", "poll"])
        .env("MGH_SHELL_SESSION", session)
        .output()
        .unwrap();
    assert_ok(&output);
    out(&output)
}

fn calls(env: &Sandbox) -> Vec<Value> {
    fs::read_to_string(env.root.join("gh.log"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn switches(env: &Sandbox) -> Vec<String> {
    calls(env)
        .iter()
        .filter(|args| args[0] == "auth" && args[1] == "switch")
        .map(|args| {
            args.as_array()
                .unwrap()
                .last()
                .unwrap()
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect()
}

fn spawn(command: &mut Command) -> Child {
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

/// Autoswitch modes need no identity configuration to save/display, and setup preserves them.
#[test]
fn autoswitch_modes_default_off_and_survive_setup() {
    let env = Sandbox::new();
    let text = out(&env.success(&["settings", "autoswitch"]));
    assert!(text.contains("OFF"), "{text}");
    assert!(!env.state.exists());
    env.success(&["settings", "autoswitch", "background"]);
    assert_eq!(
        fs::read_to_string(env.state.join("autoswitch-mode")).unwrap(),
        "background\n"
    );
    assert!(!env.root.join("gh.log").exists());
    env.setup_defaults();
    assert!(out(&env.success(&["settings", "autoswitch"])).contains("BACKGROUND"));
    env.success(&["settings", "autoswitch", "off"]);
    assert_eq!(
        fs::read_to_string(env.state.join("autoswitch-mode")).unwrap(),
        "off\n"
    );
    env.success(&["settings", "autoswitch", "on"]);
    assert!(out(&env.success(&["settings", "autoswitch"])).contains("FOREGROUND"));
}

/// Only the mode file controls autoswitch; obsolete toggle markers are ignored, not migrated.
#[test]
fn autoswitch_uses_only_the_mode_file() {
    let env = Sandbox::new();
    fs::create_dir_all(&env.state).unwrap();
    fs::write(env.state.join("autoswitch-enabled"), "").unwrap();
    fs::write(env.state.join("background-enabled"), "").unwrap();
    assert!(out(&env.success(&["settings", "autoswitch"])).contains("OFF"));
    assert!(!env.state.join("autoswitch-mode").exists());
    env.success(&["settings", "autoswitch", "foreground"]);
    assert!(out(&env.success(&["settings"])).contains("FOREGROUND"));
    fs::write(env.state.join("autoswitch-mode"), "broken\n").unwrap();
    env.rejects(&["settings", "autoswitch"], "Invalid autoswitch mode");
}

/// A running entry completes, only the globally latest pending request runs, and results
/// belong to the requesting terminal rather than whichever process acquires the lock first.
#[test]
fn background_latest_pending_wins_across_terminals() {
    let env = prepared();
    let b = repository(&env, "b", "personal");
    let c = repository(&env, "c", "work");
    fs::write(env.root.join("gh.log"), "").unwrap();
    let gate = env.root.join("switch");
    let first = entry(&env, &env.repo, "a")
        .env("MGH_SWITCH_GATE", &gate)
        .output()
        .unwrap();
    assert_ok(&first);
    assert_eq!(
        out(&first),
        "\n  Autoswitch: checking identity; personal is not allowed, switching to work in background...\n\n"
    );
    wait_until(|| gate.with_extension("started").exists());
    assert_ok(&entry(&env, &b, "b").output().unwrap());
    assert_ok(&entry(&env, &c, "c").output().unwrap());
    assert!(!queue(&env)["running"].is_null());
    assert_eq!(queue(&env)["pending"]["session"], "c");
    assert_eq!(env.accounts()["selected"], "alice");
    release(&gate);
    barrier(&env);
    assert_eq!(env.accounts()["selected"], "alice-work");
    assert_eq!(switches(&env), ["alice-work"]);
    assert_eq!(
        calls(&env)
            .iter()
            .filter(|args| args[0] == "config")
            .count(),
        3
    );
    assert!(poll(&env, "a").is_empty());
    assert!(poll(&env, "b").is_empty());
    assert!(poll(&env, "c").is_empty());
    assert!(poll(&env, "a").is_empty());
    assert!(!gate.with_extension("timed-out").exists());
}

/// An allowed account is reported at entry, never replayed after account/directory changes.
#[test]
fn background_reports_fresh_permissions_without_delayed_status() {
    let env = prepared();
    let b = repository(&env, "b", "personal");
    let first = entry(&env, &b, "b").output().unwrap();
    assert_ok(&first);
    assert!(out(&first).contains("personal is allowed, no switch needed."));
    barrier(&env);
    assert!(poll(&env, "b").is_empty());
    env.success(&["switch", "work"]);
    let gate = env.root.join("switch");
    let next = entry(&env, &b, "b")
        .env("MGH_SWITCH_GATE", &gate)
        .output()
        .unwrap();
    assert_ok(&next);
    assert!(out(&next).contains("work is not allowed, switching to personal in background..."));
    wait_until(|| gate.with_extension("started").exists());
    assert!(poll(&env, "b").is_empty());
    release(&gate);
    barrier(&env);
    assert!(poll(&env, "b").is_empty());
}

/// A currently allowed account is not stable while an older switch is in progress.
/// Returning to its repository must enqueue reconciliation, not incorrectly declare a no-op.
#[test]
fn background_busy_entry_rechecks_a_currently_allowed_account() {
    let env = prepared();
    let b = repository(&env, "b", "personal");
    let gate = env.root.join("switch");
    assert_ok(
        &entry(&env, &env.repo, "a")
            .env("MGH_SWITCH_GATE", &gate)
            .output()
            .unwrap(),
    );
    wait_until(|| gate.with_extension("started").exists());
    assert_eq!(env.accounts()["selected"], "alice");
    let next = entry(&env, &b, "b").output().unwrap();
    assert_ok(&next);
    assert!(out(&next).contains("switching to personal in background..."));
    assert!(!out(&next).contains("is allowed"));
    assert_eq!(queue(&env)["pending"]["session"], "b");
    release(&gate);
    barrier(&env);
    assert_eq!(env.accounts()["selected"], "alice");
    assert_eq!(switches(&env), ["alice-work", "alice"]);
    assert!(poll(&env, "a").is_empty());
    assert!(poll(&env, "b").is_empty());
}

/// The worker reloads permissions rather than applying the foreground check's old target.
#[test]
fn background_worker_revalidates_permissions_before_switching() {
    let env = prepared();
    let b = repository(&env, "b", "personal");
    let gate = env.root.join("switch");
    assert_ok(
        &entry(&env, &env.repo, "a")
            .env("MGH_SWITCH_GATE", &gate)
            .output()
            .unwrap(),
    );
    wait_until(|| gate.with_extension("started").exists());
    assert_ok(&entry(&env, &b, "b").output().unwrap());
    env.git_without_hooks(&[
        "-C",
        b.to_str().unwrap(),
        "config",
        "--local",
        "--replace-all",
        "mgh.allowed-identity",
        "",
    ]);
    release(&gate);
    barrier(&env);
    assert_eq!(env.accounts()["selected"], "alice-work");
    let text = poll(&env, "b");
    assert!(text.contains("mgh repo allowed update"), "{text}");
    assert!(text.contains(b.to_str().unwrap()), "{text}");
    assert!(poll(&env, "a").is_empty());
}

/// A failed immediate account check must not claim authorization or leave queued switching.
#[test]
fn background_immediate_query_failure_preserves_account_and_cancels_pending_work() {
    let env = prepared();
    let output = entry(&env, &env.repo, "a")
        .env("MGH_SELECTED_FAIL", "1")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(both(&output).contains("selected query exploded"));
    assert!(!out(&output).contains("is allowed"));
    assert!(queue(&env)["pending"].is_null());
    assert_eq!(env.accounts()["selected"], "alice");
    assert!(switches(&env).is_empty());
    assert!(poll(&env, "a").is_empty());
}

/// Manual selection discards older pending entries but a newer entry runs after it.
#[test]
fn background_manual_override_preserves_newer_entries() {
    let env = prepared();
    let b = repository(&env, "b", "personal");
    fs::write(env.root.join("gh.log"), "").unwrap();
    let gate = env.root.join("switch");
    assert_ok(
        &entry(&env, &env.repo, "a")
            .env("MGH_SWITCH_GATE", &gate)
            .output()
            .unwrap(),
    );
    wait_until(|| gate.with_extension("started").exists());
    assert_ok(&entry(&env, &b, "b").output().unwrap());
    let manual = spawn(&mut env.mgh(&["switch", "personal"]));
    wait_until(|| {
        queue(&env)["manual"]
            .as_array()
            .is_some_and(|a| !a.is_empty())
    });
    assert!(queue(&env)["pending"].is_null());
    assert_ok(&entry(&env, &env.repo, "c").output().unwrap());
    release(&gate);
    assert_ok(&manual.wait_with_output().unwrap());
    barrier(&env);
    assert_eq!(switches(&env), ["alice-work", "alice", "alice-work"]);
    assert!(poll(&env, "b").is_empty());
    assert_eq!(env.accounts()["selected"], "alice-work");
}

/// A deferred request retains its own explicit config and environment, not those of an
/// earlier worker or the Git command that waits for it.
#[test]
fn background_preserves_request_configuration_and_environment() {
    let env = prepared();
    let b = repository(&env, "b", "personal");
    let config = env.root.join("other ' identities.jsonc");
    fs::write(
        &config,
        IDENTITIES.replace("Alice@example.com", "alternate@example.com"),
    )
    .unwrap();
    let global = env.root.join("other-global.gitconfig");
    let gate = env.root.join("switch");
    assert_ok(
        &entry(&env, &env.repo, "a")
            .env("MGH_SWITCH_GATE", &gate)
            .output()
            .unwrap(),
    );
    wait_until(|| gate.with_extension("started").exists());
    assert_ok(
        &env.mgh(&[
            "--config",
            config.to_str().unwrap(),
            "internal",
            "shell-enter",
        ])
        .current_dir(&b)
        .env("MGH_SHELL_SESSION", "b")
        .env("GIT_CONFIG_GLOBAL", &global)
        .output()
        .unwrap(),
    );
    release(&gate);
    barrier(&env);
    assert_eq!(env.accounts()["selected"], "alice");
    assert!(
        fs::read_to_string(global)
            .unwrap()
            .contains("alternate@example.com")
    );
    assert!(
        fs::read_to_string(env.home.join(".gitconfig"))
            .unwrap()
            .contains("work@example.com")
    );
    assert!(poll(&env, "b").is_empty());
}

/// Leaving Git or changing modes cancels queued work, without interrupting
/// the switch that already started.
#[test]
fn background_pending_entry_cancellation() {
    for cancellation in ["leave", "off", "foreground"] {
        let env = prepared();
        let b = repository(&env, "b", "personal");
        let gate = env.root.join("switch");
        assert_ok(
            &entry(&env, &env.repo, "a")
                .env("MGH_SWITCH_GATE", &gate)
                .output()
                .unwrap(),
        );
        wait_until(|| gate.with_extension("started").exists());
        assert_ok(&entry(&env, &b, "b").output().unwrap());
        let foreground = if cancellation == "leave" {
            assert_ok(&entry(&env, &env.home, "outside").output().unwrap());
            None
        } else if cancellation == "foreground" {
            let child = spawn(&mut env.mgh(&["settings", "autoswitch", "foreground"]));
            wait_until(|| queue(&env)["pending"].is_null());
            Some(child)
        } else {
            env.success(&["settings", "autoswitch", "off"]);
            None
        };
        assert!(queue(&env)["pending"].is_null());
        release(&gate);
        if let Some(child) = foreground {
            assert_ok(&child.wait_with_output().unwrap());
        }
        barrier(&env);
        assert_eq!(env.accounts()["selected"], "alice-work");
        assert!(poll(&env, "b").is_empty());
    }
}

/// Wrapped Git waits for entry and retains ownership through real protection/project hooks;
/// nested mgh checks must not deadlock, and another terminal cannot switch mid-commit.
#[test]
fn background_git_waits_and_holds_identity_through_hooks() {
    let env = prepared();
    let b = repository(&env, "b", "personal");
    fs::write(env.repo.join("file"), "content").unwrap();
    env.git_without_hooks(&["add", "file"]);
    env.write_hook(
        "pre-commit",
        concat!(
            "mgh repo check || exit\n",
            "if mgh switch personal; then exit 9; fi\n",
            "python3 -c 'import os,time,pathlib; ",
            "p=os.environ[\"MGH_GIT_GATE\"]; pathlib.Path(p+\".started\").touch(); ",
            "deadline=time.monotonic()+15\n",
            "while not pathlib.Path(p+\".release\").exists():\n",
            " if time.monotonic()>deadline: raise RuntimeError(\"git gate timed out\")\n",
            " time.sleep(0.01)'"
        ),
        true,
    );
    let switch_gate = env.root.join("switch");
    let git_gate = env.root.join("git");
    assert_ok(
        &entry(&env, &env.repo, "a")
            .env("MGH_SWITCH_GATE", &switch_gate)
            .output()
            .unwrap(),
    );
    wait_until(|| switch_gate.with_extension("started").exists());
    let git = spawn(
        env.mgh(&["internal", "git", "--", "commit", "-m", "background test"])
            .env("MGH_GIT_GATE", &git_gate),
    );
    assert!(!git_gate.with_extension("started").exists());
    release(&switch_gate);
    wait_until(|| git_gate.with_extension("started").exists());
    assert_ok(&entry(&env, &b, "b").output().unwrap());
    assert!(!queue(&env)["pending"].is_null());
    assert_eq!(env.accounts()["selected"], "alice-work");
    release(&git_gate);
    assert_ok(&git.wait_with_output().unwrap());
    barrier(&env);
    assert_eq!(env.accounts()["selected"], "alice");
    assert_eq!(
        env.git_without_hooks(&["log", "-1", "--format=%ae"]),
        "work@example.com"
    );
}

/// Initial checkout hooks enqueue instead of waiting for the wrapper's execution lock.
#[test]
fn background_wrapped_clone_defers_initial_checkout_entry() {
    let env = prepared();
    env.commit_without_hooks("Alice Work", "work@example.com");
    let target = env.root.join("clone");
    let output = env
        .mgh(&[
            "internal",
            "git",
            "--",
            "clone",
            env.repo.to_str().unwrap(),
            target.to_str().unwrap(),
        ])
        .env("MGH_SHELL_SESSION", "clone")
        .output()
        .unwrap();
    assert_ok(&output);
    barrier(&env);
    assert!(poll(&env, "clone").contains("mgh repo allowed update"));
    assert!(
        !env.native_git(&[
            "-C",
            target.to_str().unwrap(),
            "config",
            "--local",
            "--get-all",
            "mgh.allowed-identity",
        ])
        .status
        .success()
    );
}

/// Errors and abrupt worker exits do not leave a permanent lock or silently disappear.
#[test]
fn background_failures_and_worker_exit_are_reported_and_recoverable() {
    for crash in [false, true] {
        let env = prepared();
        let gate = env.root.join("switch");
        let mut command = entry(&env, &env.repo, "a");
        if crash {
            command
                .env("MGH_SWITCH_GATE", &gate)
                .env("MGH_SWITCH_CRASH", "1");
        } else {
            command.env("MGH_SWITCH_FAIL", "1");
        }
        assert_ok(&command.output().unwrap());
        let mut text = String::new();
        if crash {
            wait_until(|| gate.with_extension("started").exists());
            wait_until(|| {
                text = poll(&env, "a");
                !text.is_empty()
            });
        } else {
            barrier(&env);
            text = poll(&env, "a");
        }
        barrier(&env);
        assert!(
            text.contains(if crash {
                "interrupted"
            } else {
                "switch exploded"
            }),
            "{text}"
        );
        assert_eq!(env.accounts()["selected"], "alice");
        assert!(text.contains(env.repo.to_str().unwrap()), "{text}");
        env.success(&["switch", "work"]);
        assert_eq!(env.accounts()["selected"], "alice-work");
    }
}

/// Neither identity nor permission checklists may take terminal input from background work.
#[test]
fn background_entry_reports_manual_actions_instead_of_prompting() {
    let env = prepared();
    env.success(&["repo", "allowed", "add", "personal"]);
    let output = entry(&env, &env.repo, "multiple").output().unwrap();
    assert_ok(&output);
    assert!(out(&output).contains("mgh switch <allowed-identity>"));
    barrier(&env);
    assert!(poll(&env, "multiple").is_empty());
    env.success(&["repo", "allowed", "remove", "personal"]);
    env.success(&["repo", "allowed", "remove", "work"]);
    let output = entry(&env, &env.repo, "empty").output().unwrap();
    assert_ok(&output);
    assert!(out(&output).contains("mgh repo allowed update"));
    barrier(&env);
    assert!(poll(&env, "empty").is_empty());
}

/// Every supported shell gives immediate permission feedback and stays responsive while
/// authentication is paused, with no delayed success/status replay.
#[test]
fn background_actual_shells_stay_responsive_and_report_at_prompts() {
    let env = prepared();
    env.driver();
    for shell in ["bash", "zsh", "fish"] {
        env.success(&["switch", "personal"]);
        let gate = env.root.join(format!("switch-{shell}"));
        let script = env.root.join(format!("{shell}.init"));
        fs::write(&script, out(&env.success(&["shell", "init", shell]))).unwrap();
        let setup = match shell {
            "bash" => format!(
                "PS1='READY> '; source {}; echo LOADED\n",
                shquote(script.to_str().unwrap())
            ),
            "zsh" => format!(
                "PROMPT='READY> '; source {}; echo LOADED\n",
                shquote(script.to_str().unwrap())
            ),
            _ => format!(
                "function fish_prompt; printf 'READY> '; end; source {}; echo LOADED\n",
                shquote(script.to_str().unwrap())
            ),
        };
        let release_code = format!(
            "import pathlib,time; p=pathlib.Path({:?}); deadline=time.monotonic()+10\n\
             while not p.with_suffix('.started').exists():\n \
              if time.monotonic()>deadline: raise RuntimeError('worker did not start')\n \
              time.sleep(0.01)\n\
             print('RESPONSIVE',flush=True); p.with_suffix('.release').touch()",
            gate.to_str().unwrap()
        );
        let steps = json!([
            ["", setup],
            [
                "LOADED\r\n",
                format!("cd {}\n", shquote(env.repo.to_str().unwrap()))
            ],
            [
                "switching to work in background...",
                format!(
                    "python3 -c {}; git status --short; echo COMMAND_DONE\n",
                    shquote(&release_code)
                )
            ],
            ["COMMAND_DONE\r\n", "echo NEXT_PROMPT\n"],
            ["NEXT_PROMPT\r\n", "exit 0\n"],
        ]);
        let args = match shell {
            "bash" => vec!["--noprofile", "--norc", "-i"],
            "zsh" => vec!["-f", "-i"],
            _ => vec!["--no-config", "-i"],
        };
        let output = env
            .command("python3")
            .current_dir(&env.home)
            .arg(env.root.join("driver.py"))
            .arg(shell)
            .args(args)
            .env("MGH_SWITCH_GATE", &gate)
            .env("MGH_PTY_STEPS", steps.to_string())
            .output()
            .unwrap();
        assert_ok(&output);
        let text = out(&output);
        assert!(text.contains("RESPONSIVE\r\n"), "{shell}: {text}");
        assert!(
            !gate.with_extension("timed-out").exists(),
            "{shell}: {text}"
        );
        assert!(
            text.find("switching to work in background...").unwrap()
                < text.find("RESPONSIVE\r\n").unwrap(),
            "{shell}: {text}"
        );
        assert_eq!(
            text.matches("Autoswitch: checking identity").count(),
            1,
            "{shell}: {text}"
        );
        assert!(!text.contains("no switch needed"), "{shell}: {text}");
    }
}
