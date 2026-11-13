use super::*;
use anyhow::{Context, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions, TryLockError},
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
    thread,
    time::Duration,
};
use tempfile::NamedTempFile;

const CONTEXT: &str = "MGH_EXECUTION_CONTEXT";
const SESSION: &str = "MGH_SHELL_SESSION";

#[derive(Default, Serialize, Deserialize)]
struct Queue {
    next: u64,
    pending: Option<Request>,
    running: Option<Request>,
    manual: Vec<PathBuf>,
}

#[derive(Clone, Serialize, Deserialize)]
struct Request {
    id: u64,
    session: String,
    work: Work,
    repository: PathBuf,
    owner: PathBuf,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub(super) enum Work {
    Autoswitch,
    Maintenance,
}

impl Queue {
    fn request(&self, id: u64, owner: &Path) -> Option<Request> {
        self.pending
            .as_ref()
            .filter(|r| r.id == id && r.owner == owner)
            .cloned()
    }
}

fn root() -> Result<PathBuf> {
    Ok(preferences::state()?.join("background"))
}

fn initialize() -> Result<PathBuf> {
    let root = root()?;
    platform::private_dir(&root)?;
    root.canonicalize()
        .context("Cannot locate background state")
}

fn open(path: &Path) -> Result<File> {
    platform::reject_symlink(path, "Coordination file")?;
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
        .open(path)
        .with_context(|| format!("Cannot open {}", path.display()))
}

struct QueueLock {
    root: PathBuf,
    _file: File,
    state: Queue,
}

impl QueueLock {
    fn acquire(root: &Path) -> Result<Self> {
        let file = open(&root.join("queue.lock"))?;
        file.lock()?;
        let state = match fs::read(root.join("queue.json")) {
            Ok(bytes) => serde_json::from_slice(&bytes).context("Invalid background queue")?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Queue::default(),
            Err(e) => return Err(e.into()),
        };
        Ok(Self {
            root: root.into(),
            _file: file,
            state,
        })
    }

    fn save(&self) -> Result<()> {
        platform::write_private(
            &self.root.join("queue.json"),
            &serde_json::to_vec(&self.state)?,
            false,
        )?;
        Ok(())
    }
}

fn owner(root: &Path) -> Result<NamedTempFile> {
    let file = tempfile::Builder::new()
        .prefix("owner-")
        .tempfile_in(root)?;
    file.as_file().lock()?;
    Ok(file)
}

fn live(root: &Path, path: &Path) -> Result<bool> {
    ensure!(path.parent() == Some(root), "Invalid coordination owner");
    let file = match OpenOptions::new().read(true).write(true).open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e.into()),
    };
    match file.try_lock() {
        Ok(()) => Ok(false),
        Err(TryLockError::WouldBlock) => Ok(true),
        Err(TryLockError::Error(e)) => Err(e.into()),
    }
}

fn inherited(root: &Path) -> Result<bool> {
    match std::env::var_os(CONTEXT) {
        Some(path) => live(root, Path::new(&path)),
        None => Ok(false),
    }
}

fn session() -> Result<String> {
    let value = std::env::var(SESSION).unwrap_or_else(|_| format!("cli-{}", std::process::id()));
    ensure!(
        !value.is_empty()
            && value.len() <= 100
            && value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
        "Invalid shell session identifier"
    );
    Ok(value)
}

fn publish(root: &Path, request: &Request, text: &str) -> Result<()> {
    if text.is_empty() {
        return Ok(());
    }
    let directory = root.join("results").join(&request.session);
    platform::write_private(
        &directory.join(format!("{:020}.error", request.id)),
        format!("Autoswitch in {:?}:\n{text}", request.repository).as_bytes(),
        false,
    )?;
    Ok(())
}

// Called only while holding execution.lock: a recorded running owner can no longer
// be executing, and a pending worker's lease distinguishes waiting from abandoned.
fn recover(queue: &mut QueueLock) -> Result<()> {
    if let Some(request) = queue.state.running.take() {
        publish(
            &queue.root,
            &request,
            "Background entry was interrupted; run mgh switch <identity> to retry.\n",
        )?;
    }
    if let Some(request) = &queue.state.pending
        && !live(&queue.root, &request.owner)?
    {
        publish(
            &queue.root,
            request,
            "Background entry did not complete; run mgh switch <identity> to retry.\n",
        )?;
        queue.state.pending = None;
    }
    let mut manual = Vec::new();
    for path in &queue.state.manual {
        if live(&queue.root, path)? {
            manual.push(path.clone());
        }
    }
    queue.state.manual = manual;
    queue.save()
}

pub(super) struct Guard {
    _file: Option<File>,
    owner: Option<NamedTempFile>,
    manual: Option<NamedTempFile>,
    root: Option<PathBuf>,
}

impl Guard {
    fn empty() -> Self {
        Self {
            _file: None,
            owner: None,
            manual: None,
            root: None,
        }
    }

    pub(super) fn command(&self, command: &mut Command) {
        if let Some(owner) = &self.owner {
            command.env(CONTEXT, owner.path());
        }
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        if let (Some(root), Some(manual)) = (&self.root, &self.manual) {
            // An abandoned reservation is also recoverable via its OS-held lease.
            let result = (|| -> Result<()> {
                let mut queue = QueueLock::acquire(root)?;
                queue.state.manual.retain(|path| path != manual.path());
                queue.save()?;
                Ok(())
            })();
            if let Err(error) = result {
                eprintln!("Cannot clear manual-switch reservation: {error:#}");
            }
        }
    }
}

pub(super) fn guard(manual: bool, wait_pending: bool) -> Result<Guard> {
    if !root()?.exists() && preferences::autoswitch()? != Autoswitch::Background {
        return Ok(Guard::empty());
    }
    let root = initialize()?;
    if inherited(&root)? {
        ensure!(
            !manual,
            "Cannot switch identities from inside a running Git command"
        );
        return Ok(Guard::empty());
    }
    let mut guard = Guard::empty();
    guard.root = Some(root.clone());
    if manual {
        let lease = owner(&root)?;
        let mut queue = QueueLock::acquire(&root)?;
        queue.state.pending = None;
        queue.state.manual.push(lease.path().into());
        queue.save()?;
        guard.manual = Some(lease);
    }
    loop {
        let file = open(&root.join("execution.lock"))?;
        file.lock()?;
        let mut queue = QueueLock::acquire(&root)?;
        recover(&mut queue)?;
        let other_manual = queue
            .state
            .manual
            .first()
            .is_some_and(|path| guard.manual.as_ref().is_none_or(|ours| ours.path() != path));
        if other_manual || (wait_pending && queue.state.pending.is_some()) {
            drop(queue);
            drop(file);
            thread::sleep(Duration::from_millis(10));
            continue;
        }
        guard.owner = Some(owner(&root)?);
        guard._file = Some(file);
        return Ok(guard);
    }
}

pub(super) fn cancel() -> Result<()> {
    if !root()?.exists() {
        return Ok(());
    }
    let root = initialize()?;
    let mut queue = QueueLock::acquire(&root)?;
    queue.state.pending = None;
    queue.save()
}

pub(super) fn enter(inv: &Invocation, startup: bool, ui: &mut dyn Interaction) -> Result<()> {
    let Some(repo) = repositories::Repository::discover()? else {
        cancel()?;
        if startup {
            workflows::welcome(inv, ui)?;
        }
        return Ok(());
    };
    let root = initialize()?;
    let file = open(&root.join("execution.lock"))?;
    let idle = match file.try_lock() {
        Ok(()) => true,
        Err(TryLockError::WouldBlock) => false,
        Err(TryLockError::Error(error)) => return Err(error.into()),
    };
    let mut queue = QueueLock::acquire(&root)?;
    if idle {
        recover(&mut queue)?;
    }
    // Register this entry even if checking fails or no switch is needed: an older
    // request must never switch accounts after this newer directory was entered.
    queue.state.pending = None;
    queue.save()?;
    ui.progress("Autoswitch: checking identity", false);
    let work = workflows::background_check(inv, &repo, idle && queue.state.manual.is_empty(), ui)?;
    enqueue(inv, &mut queue, work, repo.root())?;
    drop(queue);
    drop(file);
    if startup {
        workflows::welcome(inv, ui)?;
    }
    Ok(())
}

fn enqueue(inv: &Invocation, queue: &mut QueueLock, work: Work, repository: &Path) -> Result<()> {
    let session = session()?;
    let id = queue.state.next + 1;
    let mut command = Command::new(std::env::current_exe()?);
    if inv.config.is_some() {
        command.arg("--config").arg(inv.path()?);
    }
    command
        .args(["internal", "worker"])
        .arg(id.to_string())
        .env_remove(CONTEXT)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x00000008 | 0x00000200); // DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP
    }
    let mut child = command.spawn().context("Cannot launch background worker")?;
    let mut ready = String::new();
    BufReader::new(child.stdout.take().unwrap()).read_line(&mut ready)?;
    if ready.is_empty() {
        let status = child.wait()?;
        bail!("Background worker did not start ({status})");
    }
    let path: PathBuf = serde_json::from_str(&ready).context("Invalid worker startup response")?;
    ensure!(
        live(&queue.root, &path)?,
        "Background worker exited during startup"
    );
    queue.state.next = id;
    queue.state.pending = Some(Request {
        id,
        session,
        work,
        repository: repository.into(),
        owner: path,
    });
    queue.save()?;
    // This short-lived enqueue process must not wait for authentication work.
    drop(child);
    Ok(())
}

pub(super) fn worker(inv: &Invocation, id: u64) -> Result<()> {
    let root = initialize()?;
    let lease = owner(&root)?;
    println!("{}", serde_json::to_string(lease.path())?);
    std::io::stdout().flush()?;
    loop {
        let file = open(&root.join("execution.lock"))?;
        match file.try_lock() {
            Ok(()) => (),
            Err(TryLockError::WouldBlock) => {
                // Superseded workers need not wait for a slow switch to finish.
                let queue = QueueLock::acquire(&root)?;
                if queue.state.request(id, lease.path()).is_none() {
                    return Ok(());
                }
                drop(queue);
                thread::sleep(Duration::from_millis(10));
                continue;
            }
            Err(TryLockError::Error(error)) => return Err(error.into()),
        }
        let mut queue = QueueLock::acquire(&root)?;
        recover(&mut queue)?;
        let Some(request) = queue.state.request(id, lease.path()) else {
            return Ok(());
        };
        if !queue.state.manual.is_empty() {
            drop(queue);
            drop(file);
            thread::sleep(Duration::from_millis(10));
            continue;
        }
        if preferences::autoswitch()? != Autoswitch::Background {
            queue.state.pending = None;
            queue.save()?;
            return Ok(());
        }
        queue.state.pending = None;
        queue.state.running = Some(request.clone());
        queue.save()?;
        drop(queue);
        let mut output = Output::default();
        let result = workflows::enter_background(inv, request.work, &mut output);
        if let Err(error) = result {
            output.warning(&format!("Background entry failed: {error:#}"));
        }
        let mut queue = QueueLock::acquire(&root)?;
        publish(&root, &request, &output.errors)?;
        queue.state.running = None;
        queue.save()?;
        return Ok(());
    }
}

pub(super) fn mutation(manual: bool) -> Result<Guard> {
    if root()?.exists() {
        let root = initialize()?;
        ensure!(
            !inherited(&root)?,
            "Cannot change identity configuration from inside a running Git command"
        );
    }
    guard(manual, false)
}

pub(super) fn poll(ui: &mut dyn Interaction) -> Result<()> {
    if root()?.exists() {
        let root = initialize()?;
        let file = open(&root.join("execution.lock"))?;
        match file.try_lock() {
            Ok(()) => recover(&mut QueueLock::acquire(&root)?)?,
            Err(TryLockError::WouldBlock) => (),
            Err(TryLockError::Error(error)) => return Err(error.into()),
        }
    }
    let directory = root()?.join("results").join(session()?);
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    let mut paths = entries
        .map(|e| e.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    paths.retain(|p| p.extension().is_some_and(|e| e == "error"));
    paths.sort();
    for path in paths {
        ui.warning(&fs::read_to_string(&path)?);
        fs::remove_file(path)?;
    }
    Ok(())
}

#[derive(Default)]
struct Output {
    errors: String,
}

impl Prompts for Output {
    fn interactive(&self) -> bool {
        false
    }
    fn field(&mut self, _: &str, _: Option<&str>, _: Meaning) -> Result<String> {
        bail!("Run this command interactively")
    }
    fn permissions(&mut self, _: &[Choice], _: &[String]) -> Result<Vec<String>> {
        bail!("Run: mgh repo allowed update")
    }
    fn active(&mut self, _: &[Choice], _: Option<&str>) -> Result<String> {
        bail!("Run: mgh switch <allowed-identity>")
    }
}

impl Reports for Output {
    fn separate(&mut self) {}
    fn heading_parts(&mut self, _: &[(&str, Meaning)]) {}
    fn identity_heading(&mut self, _: &str) {}
    fn row(&mut self, _: &str, _: &str, _: Meaning, _: bool) {}
    fn message_parts(&mut self, _: &[(&str, Meaning)]) {}
    fn warning(&mut self, text: &str) {
        self.errors.push_str(text);
        self.errors.push('\n');
    }
    fn progress(&mut self, _: &str, _: bool) {}
    fn before_native(&mut self) {}
    fn native_report(&mut self, _: &[u8], _: &[u8]) -> Result<()> {
        Ok(())
    }
}
