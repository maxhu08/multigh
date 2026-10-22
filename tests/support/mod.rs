//! Shared isolated environment and process helpers for the acceptance tests.
//! Git and shells are real; only GitHub account responses are simulated.

use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};
use tempfile::TempDir;

pub(super) const IDENTITIES: &str = include_str!("../fixtures/identities.jsonc");

/// Own one temporary home, repository and set of fictional GitHub accounts.
pub(super) struct Sandbox {
    _temp: TempDir,
    pub(super) root: PathBuf,
    pub(super) home: PathBuf,
    pub(super) config: PathBuf,
    pub(super) state: PathBuf,
    pub(super) bin: PathBuf,
    pub(super) repo: PathBuf,
    real_git: PathBuf,
}

impl Sandbox {
    pub(super) fn new() -> Self {
        let temp = tempfile::Builder::new()
            .prefix("mgh acceptance ' ")
            .tempdir()
            .unwrap();
        let root = temp.path().to_path_buf();
        let home = root.join("home");
        let bin = root.join("bin");
        let state = root.join("state/multigh");
        let config = root.join("config/multigh/identities.jsonc");
        let repo = root.join("repo");
        for p in [&home, &bin, config.parent().unwrap(), &repo] {
            fs::create_dir_all(p).unwrap();
        }
        let real_git = selected_tool("git", "MGH_TEST_GIT");
        for (name, variable) in [
            ("git", "MGH_TEST_GIT"),
            ("bash", "MGH_TEST_BASH"),
            ("zsh", "MGH_TEST_ZSH"),
            ("fish", "MGH_TEST_FISH"),
            ("python3", "MGH_TEST_PYTHON"),
            ("cog", "MGH_TEST_COG"),
        ] {
            let selected = if name == "git" {
                real_git.clone()
            } else {
                selected_tool(name, variable)
            };
            let wrapper = bin.join(name);
            fs::write(
                &wrapper,
                format!(
                    "#!/bin/sh\nexec {} \"$@\"\n",
                    shquote(selected.to_str().unwrap())
                ),
            )
            .unwrap();
            fs::set_permissions(wrapper, fs::Permissions::from_mode(0o700)).unwrap();
        }
        symlink(env!("CARGO_BIN_EXE_mgh"), bin.join("mgh")).unwrap();
        let fake = bin.join("gh");
        fs::write(&fake, include_str!("fake_gh.py")).unwrap();
        fs::set_permissions(&fake, fs::Permissions::from_mode(0o700)).unwrap();
        let env = Self {
            _temp: temp,
            root,
            home,
            config,
            state,
            bin,
            repo,
            real_git,
        };
        env.set_accounts(json!({
            "selected": "alice",
            "accounts": [
                {
                    "login": "alice",
                    "active": true,
                    "state": "success"
                },
                {
                    "login": "alice-work",
                    "active": false,
                    "state": "success"
                }
            ]
        }));
        env
    }

    /// Start a child with only temporary user paths and controlled test environment variables.
    pub(super) fn command(&self, program: impl AsRef<std::ffi::OsStr>) -> Command {
        let mut c = Command::new(program);
        c.env_clear()
            .current_dir(&self.repo)
            .env(
                "PATH",
                format!(
                    "{}:/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin",
                    self.bin.display()
                ),
            )
            .env("HOME", &self.home)
            .env("XDG_CONFIG_HOME", self.root.join("config"))
            .env("XDG_STATE_HOME", self.root.join("state"))
            .env("GH_CONFIG_DIR", self.root.join("gh"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("BASH_SILENCE_DEPRECATION_WARNING", "1")
            .env("GIT_CONFIG_COUNT", "1")
            .env("GIT_CONFIG_KEY_0", "init.defaultBranch")
            .env("GIT_CONFIG_VALUE_0", "main")
            .env("GIT_CONFIG_GLOBAL", self.home.join(".gitconfig"))
            .env("MGH_TEST_REAL_GIT", &self.real_git)
            .env("MGH_AUTH", self.root.join("auth.json"))
            .env("MGH_GH_LOG", self.root.join("gh.log"))
            .env("TERM", "dumb")
            .env("NO_COLOR", "1");
        c
    }

    pub(super) fn mgh(&self, args: &[&str]) -> Command {
        let mut c = self.command(env!("CARGO_BIN_EXE_mgh"));
        c.args(args);
        c
    }

    pub(super) fn run(&self, args: &[&str]) -> Output {
        self.mgh(args).output().unwrap()
    }

    pub(super) fn success(&self, args: &[&str]) -> Output {
        let o = self.run(args);
        assert_ok(&o);
        o
    }

    pub(super) fn rejects(&self, args: &[&str], text: &str) -> Output {
        let o = self.run(args);
        assert_eq!(o.status.code(), Some(1), "{}", both(&o));
        assert!(both(&o).contains(text), "missing {text:?}: {}", both(&o));
        o
    }

    pub(super) fn init_repo(&self) {
        self.git_without_hooks(&["init", "-b", "main"]);
    }

    /// Prepare fixtures without invoking mgh; enforcement tests use `native_git` or `hook`.
    pub(super) fn git_without_hooks(&self, args: &[&str]) -> String {
        let mut c = self.command("git");
        c.args(["-c", "core.hooksPath=/dev/null"]).args(args);
        let o = c.output().unwrap();
        assert_ok(&o);
        String::from_utf8(o.stdout)
            .unwrap()
            .trim_end_matches('\n')
            .to_string()
    }

    pub(super) fn set_local(&self, key: &str, value: &str) {
        self.git_without_hooks(&["config", "--local", "--replace-all", key, value]);
    }

    pub(super) fn write_identities(&self) {
        fs::write(&self.config, IDENTITIES).unwrap();
    }

    /// Write the shared personal/work identities and run the production setup command.
    pub(super) fn setup_defaults(&self) {
        self.write_identities();
        self.success(&["setup"]);
    }

    pub(super) fn set_accounts(&self, value: Value) {
        fs::write(
            self.root.join("auth.json"),
            serde_json::to_vec(&value).unwrap(),
        )
        .unwrap();
    }

    pub(super) fn accounts(&self) -> Value {
        serde_json::from_slice(&fs::read(self.root.join("auth.json")).unwrap()).unwrap()
    }

    /// Invoke real Git with hooks enabled to observe production acceptance and rejection.
    pub(super) fn native_git(&self, args: &[&str]) -> Output {
        self.command("git").args(args).output().unwrap()
    }

    /// Create history with chosen identities, including commits that protections would reject.
    pub(super) fn commit_without_hooks(&self, name: &str, email: &str) -> String {
        let p = self.repo.join("file");
        fs::write(
            &p,
            format!("{name} {email} {:?}", std::time::SystemTime::now()),
        )
        .unwrap();
        self.git_without_hooks(&["add", "file"]);
        let o = self
            .command("git")
            .args(["-c", "core.hooksPath=/dev/null", "commit", "-m", "fixture"])
            .env("GIT_AUTHOR_NAME", name)
            .env("GIT_AUTHOR_EMAIL", email)
            .env("GIT_COMMITTER_NAME", name)
            .env("GIT_COMMITTER_EMAIL", email)
            .output()
            .unwrap();
        assert_ok(&o);
        self.git_without_hooks(&["rev-parse", "HEAD"])
    }

    pub(super) fn hook(&self, name: &str, input: &[u8], extra: &[&str]) -> Output {
        let mut child = self
            .mgh(&["internal", "hook", name])
            .args(extra)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(input).unwrap();
        child.wait_with_output().unwrap()
    }

    pub(super) fn write_hook(&self, name: &str, body: &str, executable: bool) {
        let p = self.repo.join(".git/hooks").join(name);
        fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(
            p,
            fs::Permissions::from_mode(if executable { 0o700 } else { 0o600 }),
        )
        .unwrap();
    }

    pub(super) fn pty(&self, program: &str, args: &[&str], steps: Value) -> Output {
        self.command("python3")
            .arg(self.root.join("driver.py"))
            .arg(program)
            .args(args)
            .env("MGH_PTY_STEPS", steps.to_string())
            .output()
            .unwrap()
    }

    pub(super) fn driver(&self) {
        fs::write(self.root.join("driver.py"), include_str!("pty.py")).unwrap();
    }
}

fn selected_tool(name: &str, variable: &str) -> PathBuf {
    if let Some(path) = std::env::var_os(variable) {
        let path = PathBuf::from(path)
            .canonicalize()
            .unwrap_or_else(|env| panic!("{variable}: {env}"));
        assert!(
            path.is_file(),
            "{variable} is not a file: {}",
            path.display()
        );
        return path;
    }
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|p| p.join(name))
        .find(|p| p.is_file())
        .unwrap_or_else(|| {
            panic!("Required acceptance-test tool {name} is missing; select it with {variable}")
        })
        .canonicalize()
        .unwrap()
}

pub(super) fn assert_ok(o: &Output) {
    assert!(o.status.success(), "{}", both(o));
}

pub(super) fn both(o: &Output) -> String {
    format!(
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

pub(super) fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

pub(super) fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

pub(super) fn bytes(path: impl AsRef<Path>) -> Vec<u8> {
    fs::read(path).unwrap()
}

pub(super) fn shquote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
