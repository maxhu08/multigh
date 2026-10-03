mod terminal;

pub use terminal::terminal;

use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    process::{Command, Output, Stdio},
};
use tempfile::{Builder, TempDir};

pub const IDENTITIES: &str = include_str!("../fixtures/identities.jsonc");

pub struct Sandbox(TempDir);

impl Sandbox {
    pub fn new() -> Self {
        let sandbox = Self(Builder::new().prefix("mgh test's ").tempdir().unwrap());

        for directory in ["bin", "repo", "config/multigh", "state", "home"] {
            fs::create_dir_all(sandbox.0.path().join(directory)).unwrap();
        }

        sandbox.write("config/multigh/identities.jsonc", IDENTITIES);
        sandbox.write("gitconfig", "[user]\nname = Bob Example\nemail = bob@example.edu\n[commit]\ngpgsign = false\n[tag]\ngpgsign = false\n[init]\ndefaultBranch = main\n");
        sandbox.write("active", "alice\n");
        sandbox.executable("bin/gh", include_str!("../fixtures/gh.sh"));
        std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_mgh"), sandbox.0.path().join("bin/mgh"))
            .unwrap();

        sandbox.ok("git", &["init"]);
        sandbox.ok(
            "git",
            &[
                "remote",
                "add",
                "origin",
                "https://github.com/alice/project.git",
            ],
        );

        sandbox
    }

    pub fn path(&self, path: &str) -> std::path::PathBuf {
        self.0.path().join(path)
    }

    pub fn write(&self, path: &str, text: &str) {
        let path = self.path(path);

        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    pub fn executable(&self, path: &str, text: &str) {
        self.write(path, text);
        fs::set_permissions(self.0.path().join(path), fs::Permissions::from_mode(0o755)).unwrap();
    }

    pub fn command(&self, program: &str) -> Command {
        let mut command = Command::new(if program == "mgh" {
            env!("CARGO_BIN_EXE_mgh")
        } else {
            program
        });

        command
            .current_dir(self.0.path().join("repo"))
            .env("HOME", self.path("home"))
            .env("USERPROFILE", self.path("home"))
            .env("GIT_CONFIG_GLOBAL", self.0.path().join("gitconfig"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("XDG_CONFIG_HOME", self.0.path().join("config"))
            .env("XDG_STATE_HOME", self.0.path().join("state"))
            .env("TEST_MGH_ROOT", self.0.path())
            .env(
                "PATH",
                format!(
                    "{}:{}",
                    self.0.path().join("bin").display(),
                    std::env::var("PATH").unwrap()
                ),
            )
            .env("NO_COLOR", "1")
            .env("GIT_ALLOW_PROTOCOL", "file");

        for variable in [
            "GIT_DIR",
            "GIT_COMMON_DIR",
            "GIT_WORK_TREE",
            "GIT_INDEX_FILE",
            "GIT_TEMPLATE_DIR",
            "GIT_CONFIG_COUNT",
            "GIT_CONFIG_PARAMETERS",
            "GIT_AUTHOR_NAME",
            "GIT_AUTHOR_EMAIL",
            "GIT_COMMITTER_NAME",
            "GIT_COMMITTER_EMAIL",
            "GH_TOKEN",
            "GITHUB_TOKEN",
        ] {
            command.env_remove(variable);
        }

        for (key, _) in std::env::vars_os() {
            if key.to_string_lossy().starts_with("GIT_CONFIG_KEY_")
                || key.to_string_lossy().starts_with("GIT_CONFIG_VALUE_")
            {
                command.env_remove(key);
            }
        }
        command
    }

    pub fn run(&self, program: &str, args: &[&str]) -> Output {
        self.command(program).args(args).output().unwrap()
    }

    pub fn input(&self, program: &str, args: &[&str], input: &[u8]) -> Output {
        let mut child = self
            .command(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        child.stdin.take().unwrap().write_all(input).unwrap();

        child.wait_with_output().unwrap()
    }

    pub fn ok(&self, program: &str, args: &[&str]) -> String {
        let output = self.run(program, args);

        assert!(
            output.status.success(),
            "{program} {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        String::from_utf8(output.stdout).unwrap()
    }

    pub fn blocked(&self, program: &str, args: &[&str], message: &str) {
        let output = self.run(program, args);

        assert!(
            !output.status.success(),
            "{program} {args:?} unexpectedly succeeded"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(message),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    pub fn protect(&self) {
        self.ok("mgh", &["setup"]);
        self.ok("mgh", &["protections", "--allow", "personal"]);
    }

    pub fn commit(&self) {
        self.ok("git", &["commit", "--allow-empty", "-m", "Example"]);
    }
}

pub fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}
