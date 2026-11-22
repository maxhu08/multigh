//! Native Windows Git and PowerShell coverage using isolated config and simulated accounts.
#![cfg(windows)]

use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::{
    env,
    ffi::{OsStr, OsString},
    fs,
    io::{Read, Write},
    os::windows::process::CommandExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
use tempfile::TempDir;

const MGH: &str = env!("CARGO_BIN_EXE_mgh");
const INIT: &str = ". ([ScriptBlock]::Create((mgh shell init powershell | Out-String)))";
const TIMEOUT: Duration = Duration::from_secs(25);
const CREATE_NO_WINDOW: u32 = 0x08000000;
const DETACHED_PROCESS: u32 = 0x00000008;

fn headless_command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

struct Sandbox {
    root: TempDir,
    home: PathBuf,
    repo: PathBuf,
    config: PathBuf,
    vars: Vec<(&'static str, OsString)>,
}

impl Sandbox {
    fn new() -> Self {
        let root = tempfile::Builder::new()
            .prefix("mgh Windows ' é ")
            .tempdir()
            .unwrap();
        checked(
            &headless_command(env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
                .arg("tests/support/fake_gh_windows.rs")
                .arg("-o")
                .arg(root.path().join("gh.exe"))
                .output()
                .unwrap(),
        );
        let home = root.path().join("home");
        let repo = root.path().join("repo");
        let config = root.path().join("config/multigh/identities.jsonc");
        for path in [&home, &repo, config.parent().unwrap()] {
            fs::create_dir_all(path).unwrap();
        }
        fs::write(&config, include_bytes!("fixtures/identities.jsonc")).unwrap();
        fs::write(root.path().join("selected"), "alice").unwrap();
        let path = env::join_paths(
            [root.path().into(), Path::new(MGH).parent().unwrap().into()]
                .into_iter()
                .chain(env::split_paths(&env::var_os("PATH").unwrap())),
        )
        .unwrap();
        let vars = vec![
            ("PATH", path),
            ("HOME", home.clone().into()),
            ("USERPROFILE", home.clone().into()),
            ("XDG_CONFIG_HOME", root.path().join("config").into()),
            ("XDG_STATE_HOME", root.path().join("state").into()),
            ("GIT_CONFIG_GLOBAL", home.join(".gitconfig").into()),
            ("GIT_CONFIG_NOSYSTEM", "1".into()),
            ("GIT_CONFIG_COUNT", "0".into()),
            ("GIT_TERMINAL_PROMPT", "0".into()),
            ("NO_COLOR", "1".into()),
            ("MGH_TEST_AUTH", root.path().into()),
            ("MGH_TEST_REPO", repo.clone().into()),
        ];
        let env = Self {
            root,
            home,
            repo,
            config,
            vars,
        };
        env.git(&["init", "--initial-branch=main"]);
        env
    }

    fn command(&self, program: &str) -> Command {
        let mut command = headless_command(program);
        command
            .env_remove("GIT_CONFIG_PARAMETERS")
            .env_remove("MGH_EXECUTION_CONTEXT")
            .env_remove("MGH_SHELL_SESSION")
            .envs(self.vars.iter().map(|(k, v)| (*k, v)))
            .current_dir(&self.repo);
        command
    }

    fn mgh(&self, args: &[&str]) -> String {
        self.run(MGH, args)
    }

    fn git(&self, args: &[&str]) -> String {
        self.run("git", args)
    }

    fn run(&self, program: &str, args: &[&str]) -> String {
        checked(&self.command(program).args(args).output().unwrap())
    }

    fn prepare_switch(&self, mode: &str) {
        self.mgh(&["setup"]);
        self.mgh(&["repo", "allowed", "add", "work"]);
        self.mgh(&["settings", "verbose", "off"]);
        checked(
            &self
                .command(MGH)
                .current_dir(self.root.path())
                .args(["settings", "autoswitch", mode])
                .output()
                .unwrap(),
        );
    }

    fn file(&self, name: &str) -> PathBuf {
        self.root.path().join(name)
    }

    fn selected(&self) -> String {
        fs::read_to_string(self.file("selected")).unwrap()
    }

    fn headless_console_members(&self, report: &str) -> u32 {
        let data = fs::read_to_string(self.file(report)).unwrap();
        let (members, has_window) = data.split_once(' ').unwrap();
        assert_eq!(has_window, "false", "{report} got a console window");
        members.parse().unwrap()
    }
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn checked(output: &Output) -> String {
    let text = text(output);
    assert!(output.status.success(), "{}: {text}", output.status);
    text
}

fn rejected(output: &Output, expected: &str) {
    assert!(
        !output.status.success(),
        "Unexpected success: {}",
        text(output)
    );
    assert!(text(output).contains(expected), "{}", text(output));
}

fn powershells() -> Vec<&'static str> {
    let mut shells = vec!["powershell.exe"];
    if headless_command("pwsh.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", "exit 0"])
        .output()
        .is_ok_and(|output| output.status.success())
    {
        shells.push("pwsh.exe");
    } else {
        assert!(
            env::var_os("MGH_REQUIRE_PWSH").is_none(),
            "PowerShell 7 is required when MGH_REQUIRE_PWSH is set"
        );
        eprintln!("Skipping PowerShell 7: pwsh.exe is not available");
    }
    shells
}

struct Console {
    child: Box<dyn Child>,
    input: Box<dyn Write + Send>,
    output: mpsc::Receiver<Vec<u8>>,
    bytes: Vec<u8>,
    prompt: usize,
    _master: Box<dyn MasterPty + Send>,
}

impl Console {
    fn new(env: &Sandbox, shell: &str) -> Self {
        let mut console = Self::spawn(
            env,
            shell,
            &[
                "-NoLogo",
                "-NoProfile",
                "-NoExit",
                "-Command",
                "Remove-Module PSReadLine -ErrorAction SilentlyContinue; \
                 $global:promptCount = 0; \
                 function prompt { $global:promptCount++; 'MGH-PROMPT-' + $global:promptCount + '> ' }",
            ],
        );
        console.next_prompt();
        console
    }

    fn spawn(env: &Sandbox, shell: &str, args: &[&str]) -> Self {
        let pty = native_pty_system()
            .openpty(PtySize {
                rows: 40,
                cols: 140,
                pixel_width: 0,
                pixel_height: 0,
            })
            .unwrap();
        let mut command = CommandBuilder::new(shell);
        command.args(args);
        for (key, value) in &env.vars {
            command.env(key, value);
        }
        command.env("TEMP", env.root.path());
        command.env_remove("MGH_EXECUTION_CONTEXT");
        command.cwd(env.root.path());
        let child = pty.slave.spawn_command(command).unwrap();
        drop(pty.slave);
        let input = pty.master.take_writer().unwrap();
        let mut reader = pty.master.try_clone_reader().unwrap();
        let (sender, output) = mpsc::channel();
        thread::spawn(move || {
            let mut bytes = [0; 4096];
            while let Ok(len) = reader.read(&mut bytes) {
                if len == 0 || sender.send(bytes[..len].to_vec()).is_err() {
                    break;
                }
            }
        });
        Self {
            child,
            input,
            output,
            bytes: vec![],
            prompt: 0,
            _master: pty.master,
        }
    }

    fn send(&mut self, keys: &str) {
        self.input.write_all(keys.as_bytes()).unwrap();
        self.input.flush().unwrap();
    }

    fn receive(&mut self, timeout: Duration) -> bool {
        match self.output.recv_timeout(timeout) {
            Ok(bytes) => {
                const CURSOR_QUERY: &[u8] = b"\x1b[6n";
                let start = self.bytes.len().saturating_sub(CURSOR_QUERY.len() - 1);
                self.bytes.extend(bytes);
                let queries = self.bytes[start..]
                    .windows(CURSOR_QUERY.len())
                    .filter(|window| *window == CURSOR_QUERY)
                    .count();
                for _ in 0..queries {
                    self.send("\x1b[1;1R");
                }
                true
            }
            Err(_) => false,
        }
    }

    fn wait_for(&mut self, marker: &str) -> String {
        let deadline = Instant::now() + TIMEOUT;
        loop {
            let text = String::from_utf8_lossy(&self.bytes);
            if let Some(index) = text.find(marker) {
                let end = index + marker.len();
                let text = text[..end].to_string();
                self.bytes.drain(..end);
                return text;
            }
            assert!(
                self.receive(deadline.saturating_duration_since(Instant::now())),
                "Waiting for {marker:?}; output: {}",
                String::from_utf8_lossy(&self.bytes)
            );
        }
    }

    fn next_prompt(&mut self) -> String {
        self.prompt += 1;
        self.wait_for(&format!("MGH-PROMPT-{}>", self.prompt))
    }

    fn run(&mut self, command: &str) -> String {
        self.send(command);
        self.send("\r");
        self.next_prompt()
    }
}

impl Drop for Console {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Setup and hooks work with native Git, a preserved shell hook path, and Git LFS.
#[test]
fn setup_hooks_protections_and_linked_worktree() {
    let env = Sandbox::new();
    let hooks = env.file("project hooks ' é");
    fs::create_dir(&hooks).unwrap();
    env.git(&[
        "config",
        "--local",
        "core.hooksPath",
        hooks.to_str().unwrap(),
    ]);
    env.git(&["lfs", "install", "--local"]);
    fs::write(
        hooks.join("pre-commit"),
        "#!/bin/sh\nprintf 'reached\\n' > project-commit\n[ ! -f reject-commit ]\n",
    )
    .unwrap();
    fs::write(
        hooks.join("pre-push"),
        concat!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > project-push-args\n",
            "cat > project-push-input\nexec git lfs pre-push \"$@\" < project-push-input\n",
        ),
    )
    .unwrap();
    assert!(env.mgh(&["setup"]).contains("$PROFILE"));
    env.mgh(&["doctor"]);
    // Git resolves author details before it reaches the pre-commit hook.
    env.git(&["config", "--local", "user.name", "Unconfigured author"]);
    env.git(&[
        "config",
        "--local",
        "user.email",
        "unconfigured@example.com",
    ]);
    let commit = |message| {
        env.command("git")
            .args(["commit", "--allow-empty", "-m", message])
            .output()
            .unwrap()
    };
    rejected(&commit("blocked"), "No identities are authorized");
    assert!(!env.repo.join("project-commit").exists());

    env.mgh(&["repo", "allowed", "add", "personal"]);
    env.mgh(&["switch", "personal"]);
    fs::write(env.repo.join("reject-commit"), "").unwrap();
    rejected(
        &commit("rejected by project"),
        "Existing pre-commit hook rejected",
    );
    assert!(env.repo.join("project-commit").is_file());
    fs::remove_file(env.repo.join("reject-commit")).unwrap();
    env.git(&["lfs", "track", "*.bin"]);
    fs::write(env.repo.join("asset.bin"), b"fixture asset data").unwrap();
    env.git(&["add", ".gitattributes", "asset.bin"]);
    checked(&commit("allowed"));

    fs::remove_file(hooks.join("pre-commit")).unwrap();
    fs::copy(MGH, hooks.join("pre-commit.exe")).unwrap();
    let native_hook = env.git(&["hook", "run", "pre-commit", "--", "--version"]);
    assert_eq!(native_hook.trim(), env.mgh(&["--version"]).trim());

    let remote = env.file("local remote");
    env.git(&["init", "--bare", remote.to_str().unwrap()]);
    env.git(&["remote", "add", "origin", remote.to_str().unwrap()]);
    env.git(&["push", "origin", "main"]);
    let args = fs::read_to_string(env.repo.join("project-push-args")).unwrap();
    assert_eq!(args.lines().next(), Some("origin"));
    assert!(args.contains("local remote"), "{args}");
    let stdin = fs::read_to_string(env.repo.join("project-push-input")).unwrap();
    assert!(stdin.starts_with("refs/heads/main "), "{stdin}");
    assert!(
        remote.join("lfs/objects").is_dir(),
        "Git LFS did not upload the object to the local remote"
    );

    fs::remove_file(env.repo.join("project-push-input")).unwrap();
    env.mgh(&["switch", "work"]);
    rejected(
        &env.command("git")
            .args(["push", "origin", "main"])
            .output()
            .unwrap(),
        "is not allowed",
    );
    assert!(!env.repo.join("project-push-input").exists());
    let linked = env.file("linked worktree");
    env.git(&["worktree", "add", "-b", "linked", linked.to_str().unwrap()]);
    let allowed = checked(
        &env.command(MGH)
            .current_dir(&linked)
            .args(["repo", "allowed", "list"])
            .output()
            .unwrap(),
    );
    assert!(allowed.contains("personal"), "{allowed}");
}

/// Profile startup is inert in noninteractive shells even when preferences are enabled.
#[test]
fn noninteractive_profile_does_not_install_integration() {
    let env = Sandbox::new();
    env.mgh(&["settings", "welcome", "on"]);
    for shell in powershells() {
        let output = checked(
            &env.command(shell)
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    &format!(
                        "{INIT}; if ($global:_MghPowerShellLoaded) {{ exit 3 }}; Write-Output 'quiet'"
                    ),
                ])
                .output()
                .unwrap(),
        );
        assert_eq!(output.trim(), "quiet");
        // An attached console does not make a one-shot invocation interactive.
        let mut terminal = Console::spawn(
            &env,
            shell,
            &[
                "-NoProfile",
                "-Command",
                &format!(
                    "{INIT}; if ($global:_MghPowerShellLoaded) {{ exit 3 }}; Write-Host 'quiet'"
                ),
            ],
        );
        let output = terminal.wait_for("quiet");
        assert!(!output.contains("GitHub username"), "{output}");
        assert!(!env.home.join(".gitconfig").exists());
    }
}

/// Captured queries must not create visible consoles when mgh has no console.
#[test]
fn detached_status_keeps_queries_headless() {
    let env = Sandbox::new();
    checked(
        &env.command(MGH)
            .creation_flags(DETACHED_PROCESS)
            .args(["status", "--full"])
            .output()
            .unwrap(),
    );
    env.headless_console_members("status-console");
    env.headless_console_members("full-status-console");
}

/// Welcome is once per session; entry is once per repository and pickers keep the console.
#[test]
fn powershell_welcome_and_foreground_entry() {
    for shell in powershells() {
        let env = Sandbox::new();
        env.prepare_switch("foreground");
        env.mgh(&["settings", "welcome", "on"]);
        fs::create_dir(env.repo.join("subdir")).unwrap();
        let config = env.file("custom identities ' é.jsonc");
        fs::rename(&env.config, &config).unwrap();

        let mut terminal = Console::new(&env, shell);
        let output = terminal.run(concat!(
            "Set-StrictMode -Version Latest; ",
            "function global:git { 'existing-' + 'git' }; ",
            "$i = mgh --config (Join-Path $env:MGH_TEST_AUTH \"custom identities ' $([char]233).jsonc\") ",
            "shell init powershell | Out-String; $s = [ScriptBlock]::Create($i); . $s; . $s"
        ));
        assert_eq!(output.matches("GitHub username").count(), 1, "{output}");
        assert!(output.contains("alice"), "{output}");
        let output = terminal.run("git");
        assert!(output.contains("existing-git"), "{output}");
        let output = terminal.run("Set-Location -LiteralPath $env:MGH_TEST_REPO");
        assert!(output.contains("switching to work"), "{output}");
        assert_eq!(env.selected(), "alice-work");
        let output = terminal.run("Set-Location subdir");
        assert!(!output.contains("Autoswitch:"), "{output}");
        let completion = terminal.run(
            "(TabExpansion2 'mgh settings wel' 16).CompletionMatches | Select-Object -ExpandProperty CompletionText",
        );
        assert!(completion.contains("welcome"), "{completion}");
        let output = terminal.run("git.exe missing-mgh-subcommand");
        assert!(output.contains("is not a git command"), "{output}");
        let output = terminal.run("Write-Host ('GIT-' + $LASTEXITCODE)");
        assert!(output.contains("GIT-1"), "{output}");

        checked(
            &env.command(MGH)
                .arg("--config")
                .arg(config)
                .args(["repo", "allowed", "add", "personal"])
                .output()
                .unwrap(),
        );
        terminal.run("Set-Location -LiteralPath $env:MGH_TEST_AUTH");
        terminal.send("Set-Location -LiteralPath $env:MGH_TEST_REPO\r");
        terminal.wait_for("Which allowed identity should be active?");
        terminal.send("\r");
        terminal.next_prompt();
        assert_eq!(env.selected(), "alice-work");
    }
}

/// A pending switch leaves the prompt responsive, blocks integrated Git, and reports failures once.
#[test]
fn powershell_background_entry_and_git_wait() {
    for shell in powershells() {
        let env = Sandbox::new();
        env.prepare_switch("background");
        let mut terminal = Console::new(&env, shell);
        terminal.run(INIT);
        let pause = env.file("pause-switch");
        fs::write(&pause, "").unwrap();
        let output = terminal.run("Set-Location -LiteralPath $env:MGH_TEST_REPO");
        assert!(
            output.contains("switching to work in background"),
            "{output}"
        );
        let deadline = Instant::now() + TIMEOUT;
        while !env.file("switch-started").exists() {
            assert!(Instant::now() < deadline, "Background switch did not start");
            thread::sleep(Duration::from_millis(10));
        }
        assert!(
            env.headless_console_members("switch-console") > 1,
            "GitHub CLI did not inherit the background worker's console"
        );
        assert_eq!(env.selected(), "alice");
        terminal.send("git rev-parse --is-inside-work-tree; Write-Host ('GIT-' + 'FINISHED')\r");
        let deadline = Instant::now() + Duration::from_millis(350);
        while terminal.receive(deadline.saturating_duration_since(Instant::now())) {
            assert!(
                !String::from_utf8_lossy(&terminal.bytes).contains("GIT-FINISHED"),
                "Git ran before the switch finished"
            );
            if Instant::now() >= deadline {
                break;
            }
        }
        fs::remove_file(&pause).unwrap();
        assert!(terminal.next_prompt().contains("GIT-FINISHED"));
        assert_eq!(env.selected(), "alice-work");

        terminal.run("Set-Location -LiteralPath $env:MGH_TEST_AUTH");
        env.mgh(&["switch", "personal"]);
        fs::write(env.file("fail-switch"), "").unwrap();
        let mut output = terminal.run("Set-Location -LiteralPath $env:MGH_TEST_REPO");
        let deadline = Instant::now() + TIMEOUT;
        while !output.contains("test switch failed") {
            assert!(Instant::now() < deadline, "{output}");
            output.push_str(&terminal.run("$null = 0"));
        }
        assert!(!terminal.run("$null = 0").contains("test switch failed"));
        assert_eq!(env.selected(), "alice");
    }
}

/// The companion keeps Git's process arguments and streams, including piped input.
#[test]
fn native_git_companion_preserves_streams_and_exit_status() {
    use std::process::Stdio;

    let env = Sandbox::new();
    let mut child = env
        .command(env!("CARGO_BIN_EXE_mgh-git"))
        .args(["hash-object", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"a\nb\x00c").unwrap();
    let hash = checked(&child.wait_with_output().unwrap());
    fs::write(env.repo.join("input file"), b"a\nb\x00c").unwrap();
    assert_eq!(hash, env.git(&["hash-object", "input file"]));
    let args = [
        OsStr::new("-c"),
        OsStr::new("mgh.example=a \"quoted\" value"),
        OsStr::new("config"),
        OsStr::new("--get"),
        OsStr::new("mgh.example"),
    ];
    let output = env
        .command(env!("CARGO_BIN_EXE_mgh-git"))
        .args(args)
        .output()
        .unwrap();
    assert_eq!(checked(&output).trim(), "a \"quoted\" value");
    let bad = env
        .command(env!("CARGO_BIN_EXE_mgh-git"))
        .args(["config", "--get", "mgh.missing"])
        .output()
        .unwrap();
    assert_eq!(bad.status.code(), Some(1), "{}", text(&bad));
}
