// From Saddle `src/control.rs` at commit `f7d1bbafc84102edfef49b47bf49d5df90e2b2e3`.
// Adapted for paddock: private temporary runtime, transport-owned records, Browser and GPUI handoff.
//! Bounded, local JSON control transport. Only the UI loop mutates the workspace.
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    ffi::CString,
    fs,
    io::{Read, Write},
    os::fd::AsRawFd,
    os::unix::{
        fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
        net::{UnixListener, UnixStream},
    },
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
    thread,
    time::{Duration, Instant},
};

#[cfg(test)]
use std::os::unix::fs::PermissionsExt;

#[path = "control_cli.rs"]
mod cli;
pub use cli::run;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Place {
    Tab,
    Left,
    Right,
    Up,
    Down,
}

pub const LIMIT: usize = 64 * 1024;
pub const RECORD_LIMIT: usize = 256;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Caller {
    pub name: Option<String>,
    pub corral_instance: Option<String>,
    pub paddock_instance: Option<String>,
    pub pane: Option<u64>,
}
impl Caller {
    pub fn environment() -> Self {
        let env = |key| std::env::var(key).ok().filter(|s| !s.is_empty());
        Self {
            name: env("CORRAL_NAME"),
            corral_instance: env("CORRAL_INSTANCE"),
            paddock_instance: env("PADDOCK_INSTANCE"),
            pane: env("PADDOCK_PANE").and_then(|s| s.parse().ok()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Content {
    Shell {
        cwd: Option<String>,
    },
    Agent {
        name: String,
    },
    NewAgent {
        name: String,
        cwd: Option<String>,
        role: String,
        prompt: Option<String>,
        argv: Vec<String>,
    },
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    content = "id",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum CloseTarget {
    Pane(u64),
    Tab(u64),
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    Inspect,
    Instances,
    Browse {
        url: String,
        focus: bool,
    },
    Request {
        request: String,
    },
    Open {
        relative_to: String,
        place: crate::control::Place,
        content: Content,
        focus: bool,
    },
    Close {
        target: CloseTarget,
        confirmation: Option<String>,
        confirm_shells: bool,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Message {
    pub instance: String,
    pub request_id: Option<String>,
    pub caller: Caller,
    pub operation: Operation,
}
struct Incoming {
    pub message: Message,
    pub reply: SyncSender<Value>,
}
pub fn error(code: &str, message: impl ToString) -> Value {
    json!({"ok": false, "error": {"code": code, "message": message.to_string()}})
}
pub fn random_id() -> Result<String> {
    let mut bytes = [0_u8; 8];
    fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
pub fn runtime_dir() -> Result<PathBuf> {
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .or_else(|| std::env::var_os("TMPDIR"))
        .map(PathBuf::from)
        .context("no private runtime directory; set XDG_RUNTIME_DIR or TMPDIR")?;
    anyhow::ensure!(base.is_absolute(), "runtime base must be absolute");
    // Canonicalize the OS temporary base (macOS /var is a symlink), but never
    // follow a link in paddock's own directory.
    let base = base.canonicalize()?;
    private_directory(&fs::symlink_metadata(&base)?)?;
    let dir = base.join("paddock");
    prepare_runtime(&dir)?;
    Ok(dir)
}
fn private_directory(metadata: &fs::Metadata) -> Result<()> {
    anyhow::ensure!(
        metadata.uid() == unsafe { libc::geteuid() },
        "runtime directory must be owned by the current user"
    );
    anyhow::ensure!(
        metadata.is_dir() && metadata.mode() & 0o7777 == 0o700,
        "runtime directory must be a real directory with mode 0700"
    );
    Ok(())
}
fn trusted_parent(dir: &std::path::Path) -> Result<PathBuf> {
    anyhow::ensure!(dir.is_absolute(), "runtime directory must be absolute");
    let parent = dir
        .parent()
        .context("runtime directory needs a parent")?
        .canonicalize()?;
    for path in parent.ancestors() {
        let metadata = fs::symlink_metadata(path)?;
        anyhow::ensure!(
            metadata.is_dir()
                && (metadata.uid() == 0 || metadata.uid() == unsafe { libc::geteuid() })
                && (metadata.mode() & 0o022 == 0 || metadata.mode() & 0o1000 != 0),
            "untrusted runtime ancestor: {} (require root/current owner and no other writers, or sticky)",
            path.display()
        );
    }
    Ok(parent.join(dir.file_name().context("runtime directory needs a name")?))
}
fn prepare_runtime(dir: &std::path::Path) -> Result<PathBuf> {
    let dir = trusted_parent(dir)?;
    match fs::DirBuilder::new().mode(0o700).create(&dir) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e.into()),
    }
    private_directory(&fs::symlink_metadata(&dir)?)?;
    Ok(dir)
}
/// Keep the checked directory open so chmod/cleanup cannot follow a replacement
/// ancestor into another directory. Ancestor policy protects against other UIDs;
/// identity checks also reject replacements by our own UID during bind/connect.
struct RuntimeDirectory {
    path: PathBuf,
    file: fs::File,
}
impl RuntimeDirectory {
    fn open(dir: &std::path::Path) -> Result<Self> {
        let path = trusted_parent(dir)?;
        let file = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&path)?;
        private_directory(&file.metadata()?)?;
        let directory = Self { path, file };
        directory.check()?;
        Ok(directory)
    }
    fn check(&self) -> Result<()> {
        anyhow::ensure!(
            trusted_parent(&self.path)? == self.path,
            "runtime ancestor changed"
        );
        let current = fs::symlink_metadata(&self.path)?;
        private_directory(&current)?;
        let original = self.file.metadata()?;
        anyhow::ensure!(
            (current.dev(), current.ino()) == (original.dev(), original.ino()),
            "runtime directory identity changed"
        );
        Ok(())
    }
    fn unlink(&self, name: &str) {
        if let Ok(name) = CString::new(name) {
            unsafe {
                libc::unlinkat(self.file.as_raw_fd(), name.as_ptr(), 0);
            }
        }
    }
}

fn check_peer(stream: &UnixStream, expected_uid: libc::uid_t) -> Result<()> {
    let (mut uid, mut gid) = (0, 0);
    anyhow::ensure!(
        unsafe { libc::getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) } == 0,
        "cannot read control peer credentials: {}",
        std::io::Error::last_os_error()
    );
    anyhow::ensure!(
        uid == expected_uid,
        "control peer UID does not match current user"
    );
    Ok(())
}

#[cfg(test)]
mod permission_tests {
    use super::*;
    #[test]
    fn refuses_permissive_existing_runtime_without_changing_it() {
        let dir = std::env::temp_dir().join(format!("pc-{}", random_id().unwrap()));
        fs::DirBuilder::new().mode(0o755).create(&dir).unwrap();
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(
            prepare_runtime(&dir).is_err(),
            "must refuse, not silently repair permissions"
        );
        assert_eq!(fs::metadata(&dir).unwrap().mode() & 0o777, 0o755);
    }
}

fn socket_path(dir: &std::path::Path, id: &str) -> Result<PathBuf> {
    if id.len() != 16 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
        bail!("invalid instance ID");
    }
    let path = dir.join(format!("{id}.sock"));
    // sockaddr_un.sun_path is 104 bytes on macOS, 108 on Linux.
    if path.as_os_str().as_encoded_bytes().len() >= 104 {
        bail!("runtime socket path too long; set XDG_RUNTIME_DIR to a shorter private directory");
    }
    Ok(path)
}
fn read_json(stream: &mut UnixStream) -> Result<Value> {
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 4096];
    loop {
        // On macOS, setting SO_RCVTIMEO after the peer closes can fail with EINVAL
        // even when the complete response is buffered. Wait for readability instead.
        use std::os::fd::AsRawFd;
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .context("read timed out")?;
        let mut poll = libc::pollfd {
            fd: stream.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let ready = unsafe { libc::poll(&mut poll, 1, remaining.as_millis().max(1) as i32) };
        if ready < 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error.into());
        }
        anyhow::ensure!(ready > 0, "read timed out");
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            bail!("incomplete control message");
        }
        let end = chunk[..n].iter().position(|b| *b == b'\n');
        bytes.extend_from_slice(&chunk[..end.unwrap_or(n)]);
        if bytes.len() > LIMIT {
            bail!("control message exceeds 64 KiB");
        }
        if end.is_some() {
            return Ok(serde_json::from_slice(&bytes)?);
        }
    }
}
fn write_json(stream: &mut UnixStream, value: &impl Serialize) -> Result<()> {
    let mut bytes = serde_json::to_vec(value)?;
    if bytes.len() > LIMIT {
        bail!("control message exceeds 64 KiB");
    }
    bytes.push(b'\n');
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    stream.write_all(&bytes)?;
    Ok(())
}
fn connect(path: &std::path::Path) -> Result<UnixStream> {
    connect_as(path, unsafe { libc::geteuid() })
}
fn connect_as(path: &std::path::Path, expected_uid: libc::uid_t) -> Result<UnixStream> {
    use std::os::fd::{AsRawFd, FromRawFd};
    // Nonblocking connect also bounds a full Unix listen backlog.
    let fd = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM, 0) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let stream = unsafe { UnixStream::from_raw_fd(fd) };
    unsafe {
        libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC);
    }
    stream.set_nonblocking(true)?;
    let mut address: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    address.sun_family = libc::AF_UNIX as _;
    for (out, byte) in address
        .sun_path
        .iter_mut()
        .zip(path.as_os_str().as_encoded_bytes())
    {
        *out = *byte as _;
    }
    let length = std::mem::size_of_val(&address) as libc::socklen_t;
    #[cfg(target_os = "macos")]
    {
        address.sun_len = length as u8;
    }
    let status =
        unsafe { libc::connect(fd, &address as *const _ as *const libc::sockaddr, length) };
    if status < 0 {
        let error = std::io::Error::last_os_error();
        if !matches!(
            error.raw_os_error(),
            Some(libc::EINPROGRESS) | Some(libc::EAGAIN)
        ) {
            return Err(error.into());
        }
        let mut poll = libc::pollfd {
            fd: stream.as_raw_fd(),
            events: libc::POLLOUT,
            revents: 0,
        };
        anyhow::ensure!(
            unsafe { libc::poll(&mut poll, 1, 500) } > 0,
            "socket connection timed out"
        );
        if let Some(error) = stream.take_error()? {
            return Err(error.into());
        }
    }
    stream.set_nonblocking(false)?;
    check_peer(&stream, expected_uid)?;
    Ok(stream)
}
pub fn exchange(message: &Message) -> Result<Value> {
    exchange_in(&runtime_dir()?, message)
}
fn exchange_in(dir: &std::path::Path, message: &Message) -> Result<Value> {
    exchange_checked(dir, message, || {})
}
fn exchange_checked(
    dir: &std::path::Path,
    message: &Message,
    before_connect: impl FnOnce(),
) -> Result<Value> {
    let directory = RuntimeDirectory::open(dir)?;
    let path = socket_path(&directory.path, &message.instance)?;
    use std::os::unix::fs::FileTypeExt;
    let metadata = fs::symlink_metadata(&path)?;
    anyhow::ensure!(
        metadata.file_type().is_socket()
            && metadata.uid() == unsafe { libc::geteuid() }
            && metadata.mode() & 0o7777 == 0o600,
        "socket must be owned by the current user and mode 0600"
    );
    before_connect();
    let mut stream =
        connect(&path).context("instance unavailable (it may have exited or restarted)")?;
    directory.check()?;
    write_json(&mut stream, message)?;
    read_json(&mut stream)
}
pub fn instances(caller: &Caller) -> Result<Vec<Value>> {
    instances_in(&runtime_dir()?, caller)
}
fn instances_in(dir: &std::path::Path, caller: &Caller) -> Result<Vec<Value>> {
    let directory = RuntimeDirectory::open(dir)?;
    let dir = &directory.path;
    instances_from(
        dir,
        caller,
        fs::read_dir(dir)?.map(|entry| entry.map(|e| e.path())),
    )
}
fn instances_from(
    dir: &std::path::Path,
    caller: &Caller,
    entries: impl IntoIterator<Item = std::io::Result<PathBuf>>,
) -> Result<Vec<Value>> {
    let mut result = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(3);
    for entry in entries {
        anyhow::ensure!(
            Instant::now() < deadline,
            "instance discovery incomplete: timed out; use an explicit --instance"
        );
        let path = entry?;
        if path.extension().is_none_or(|s| s != "sock") {
            continue;
        }
        let Some(id) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        // A socket is the registration. Dead processes leave unreachable sockets;
        // ignore these without deleting another instance's files.
        if let Ok(value) = exchange_in(
            dir,
            &Message {
                instance: id.into(),
                request_id: None,
                caller: caller.clone(),
                operation: Operation::Instances,
            },
        ) && value["ok"] == true
            && value["instance"] == id
        {
            result.push(value);
        }
    }
    result.sort_by(|a, b| a["instance"].as_str().cmp(&b["instance"].as_str()));
    Ok(result)
}

/// In-memory per-instance ledger. No eviction: all 256 results remain queryable.
#[derive(Default)]
pub struct Records(Vec<(Message, Value)>);
impl Records {
    /// Read results (including close confirmation tokens) without changing their identities.
    pub fn values(&self) -> impl Iterator<Item = &Value> {
        self.0.iter().map(|(_, value)| value)
    }
    pub fn get(&self, id: &str) -> Option<&Value> {
        self.0
            .iter()
            .find(|(m, _)| m.request_id.as_deref() == Some(id))
            .map(|(_, v)| v)
    }
    /// The UI may advance accepted/starting/attaching to complete/failed/target_invalid.
    /// Keep the instance and request identity stable while replacing the result.
    pub fn update(&mut self, id: &str, mut value: Value) -> bool {
        if let Some((message, result)) = self
            .0
            .iter_mut()
            .find(|(m, _)| m.request_id.as_deref() == Some(id))
        {
            stamp(&mut value, message);
            *result = value;
            true
        } else {
            false
        }
    }
    fn dispatch(
        &mut self,
        message: &Message,
        handler: &mut impl FnMut(&Message, &Records) -> Value,
    ) -> Value {
        if let Operation::Request { request } = &message.operation {
            return self.get(request).cloned().unwrap_or_else(|| {
                error(
                    "request_unavailable",
                    "request not recorded in this instance",
                )
            });
        }
        if !message.operation.is_mutation() {
            return handler(message, self);
        }
        let Some(id) = message
            .request_id
            .as_deref()
            .filter(|id| !id.is_empty() && id.len() <= 128)
        else {
            return error("invalid_request", "request_id must contain 1–128 bytes");
        };
        if let Some((original, value)) = self
            .0
            .iter()
            .find(|(m, _)| m.request_id.as_deref() == Some(id))
        {
            return if original == message {
                value.clone()
            } else {
                error(
                    "request_conflict",
                    "request ID already used with different parameters or caller",
                )
            };
        }
        if self.0.len() >= RECORD_LIMIT {
            return error(
                "request_limit",
                "instance request capacity reached; existing requests remain queryable",
            );
        }
        if let Operation::Browse { url, .. } = &message.operation
            && let Err(e) = web_url(url)
        {
            return error("invalid_request", e);
        }
        let mut value = handler(message, self);
        stamp(&mut value, message);
        // A busy UI has not accepted the mutation; the same ID may be retried.
        if value["error"]["code"] == "busy" || value["state"] == "busy" {
            return value;
        }
        self.0.push((message.clone(), value.clone()));
        value
    }
}
impl Operation {
    fn is_mutation(&self) -> bool {
        matches!(
            self,
            Self::Open { .. } | Self::Close { .. } | Self::Browse { .. }
        )
    }
}
fn stamp(value: &mut Value, message: &Message) {
    value["instance"] = json!(message.instance);
    if message.request_id.is_some() {
        value["request_id"] = json!(message.request_id);
    }
}
/// A handler that does nothing the UI would, for the transport's own tests.
#[cfg(test)]
pub fn unsupported(_: &Message, _: &Records) -> Value {
    let mut value = error("unsupported", "not handled in this test");
    value["state"] = json!("failed");
    value
}

fn enqueue(tx: &SyncSender<Incoming>, message: Message) -> Value {
    let (reply, wait) = mpsc::sync_channel(1);
    let identity = message.clone();
    let mut value = if tx.try_send(Incoming { message, reply }).is_err() {
        error("busy", "control queue full")
    } else {
        wait.recv_timeout(Duration::from_secs(1))
            .unwrap_or_else(|_| json!({"ok":true,"state":"uncertain"}))
    };
    stamp(&mut value, &identity);
    value
}

pub struct Server {
    pub id: String,
    incoming: Receiver<Incoming>,
    pub records: Records,
    path: PathBuf,
    directory: RuntimeDirectory,
    stop: Arc<AtomicBool>,
    workers: Vec<thread::JoinHandle<()>>,
}
impl Server {
    pub fn start() -> Result<Self> {
        Self::start_in(&runtime_dir()?)
    }
    fn start_in(dir: &std::path::Path) -> Result<Self> {
        Self::start_checked(dir, unsafe { libc::geteuid() }, || {})
    }
    fn start_checked(
        dir: &std::path::Path,
        peer_uid: libc::uid_t,
        before_bind: impl FnOnce(),
    ) -> Result<Self> {
        let dir = prepare_runtime(dir)?;
        let directory = RuntimeDirectory::open(&dir)?;
        let id = random_id()?;
        let path = socket_path(&directory.path, &id)?;
        before_bind();
        let listener = UnixListener::bind(&path)?;
        // Do not chmod through the pathname: it may now name a different directory.
        let name = CString::new(format!("{id}.sock"))?;
        let ready = (|| -> Result<()> {
            directory.check()?;
            anyhow::ensure!(
                unsafe {
                    libc::fchmodat(
                        directory.file.as_raw_fd(),
                        name.as_ptr(),
                        0o600,
                        libc::AT_SYMLINK_NOFOLLOW,
                    )
                } == 0,
                "cannot set socket permissions: {}",
                std::io::Error::last_os_error()
            );
            directory.check()?;
            listener.set_nonblocking(true)?;
            Ok(())
        })();
        if let Err(error) = ready {
            // Revoke the listener before dropping its fd. Another concurrent
            // fork can briefly hold a duplicate until exec closes CLOEXEC fds.
            unsafe {
                libc::shutdown(listener.as_raw_fd(), libc::SHUT_RDWR);
            }
            directory.unlink(&format!("{id}.sock"));
            return Err(error);
        }
        let stop = Arc::new(AtomicBool::new(false));
        let (tx, incoming) = mpsc::sync_channel::<Incoming>(32);
        let (sockets, rx) = mpsc::sync_channel::<UnixStream>(8);
        let rx = Arc::new(Mutex::new(rx));
        let mut workers = Vec::new();
        for _ in 0..4 {
            let (rx, tx, stop, instance) = (rx.clone(), tx.clone(), stop.clone(), id.clone());
            workers.push(thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    let stream = rx.lock().unwrap().recv_timeout(Duration::from_millis(50));
                    let Ok(mut stream) = stream else {
                        continue;
                    };
                    let value = match read_json(&mut stream)
                        .and_then(|v| Ok(serde_json::from_value::<Message>(v)?))
                    {
                        Ok(mut message) => {
                            if message.instance != instance {
                                let _ = write_json(&mut stream, &error("instance_unavailable", "instance has exited or restarted"));
                                continue;
                            }
                            if matches!(message.operation, Operation::Instances) {
                                let _ = write_json(&mut stream, &json!({"ok":true,"instance":instance,"pid":std::process::id()}));
                                continue;
                            }
                            if message.operation.is_mutation() && message.request_id.is_none() {
                                match random_id() {
                                    Ok(id) => message.request_id = Some(id),
                                    Err(e) => {
                                        let _ = write_json(&mut stream, &error("failed", e));
                                        continue;
                                    }
                                }
                            }
                            enqueue(&tx, message)
                        }
                        Err(e) => error("invalid_request", e),
                    };
                    if write_json(&mut stream, &value).is_err() {
                        let _ = write_json(
                            &mut stream,
                            &error("response_too_large", "response exceeds transport limit"),
                        );
                    }
                }
            }));
        }
        let done = stop.clone();
        workers.push(thread::spawn(move || {
            while !done.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        if let Err(e) = check_peer(&stream, peer_uid) {
                            // Fixed small error, without allowing an untrusted peer to
                            // block accept or to reach the worker/message queues.
                            let _ = stream.set_nonblocking(true);
                            let _ = write_json(&mut stream, &error("peer_uid_mismatch", e));
                            continue;
                        }
                        let _ = sockets.try_send(stream);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10))
                    }
                    Err(_) => break,
                }
            }
        }));
        Ok(Self {
            id,
            incoming,
            records: Records::default(),
            path,
            directory,
            stop,
            workers,
        })
    }
    /// Call on the UI thread. FIFO, at most 32 messages per tick; socket workers
    /// never access UI state; `main.rs` hands each request to the window.
    /// Handler results must be JSON objects. `records` is available for close
    /// confirmation lookups; asynchronous work calls `records.update` later.
    pub fn process_pending(&mut self, mut handler: impl FnMut(&Message, &Records) -> Value) {
        for incoming in self.incoming.try_iter().take(32) {
            let mut value = self.records.dispatch(&incoming.message, &mut handler);
            stamp(&mut value, &incoming.message);
            let _ = incoming.reply.try_send(value);
        }
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.directory
            .unlink(self.path.file_name().unwrap().to_str().unwrap());
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn truncated_closed_response_is_not_success() {
        let (mut client, mut server) = UnixStream::pair().unwrap();
        server.write_all(b"{\"ok\":true}").unwrap();
        drop(server);
        assert!(
            read_json(&mut client)
                .unwrap_err()
                .to_string()
                .contains("incomplete")
        );
    }
    #[test]
    fn reads_a_complete_response_after_the_peer_has_closed() {
        let (mut client, mut server) = UnixStream::pair().unwrap();
        let value = json!({"ok":true,"result":"x".repeat(1000)});
        server
            .write_all(&serde_json::to_vec(&value).unwrap())
            .unwrap();
        server.write_all(b"\n").unwrap();
        drop(server);
        assert_eq!(read_json(&mut client).unwrap(), value);
    }
}

#[cfg(test)]
mod record_tests {
    use super::*;
    #[test]
    fn replay_does_not_execute_twice_and_changed_parameters_conflict() {
        let mut records = Records::default();
        let mut message = Message {
            instance: "0123456789abcdef".into(),
            request_id: Some("preassigned".into()),
            caller: Caller::default(),
            operation: Operation::Browse {
                url: "https://example.com".into(),
                focus: false,
            },
        };
        let mut calls = 0;
        let mut handler = |_: &Message, _: &Records| {
            calls += 1;
            json!({"ok":true,"state":"starting","pane":7,"revision":2})
        };
        let first = records.dispatch(&message, &mut handler);
        assert_eq!(first, records.dispatch(&message, &mut handler));
        message.caller.name = Some("different".into());
        assert_eq!(
            records.dispatch(&message, &mut handler)["error"]["code"],
            "request_conflict"
        );
        assert_eq!(calls, 1);
    }
}

/// Same local-address completion as the Browser toolbar, restricted to web URLs.
fn browse_url(input: &str) -> Result<String> {
    let url = crate::browser_view::address(input).context("browse needs a web URL")?;
    web_url(&url)?;
    Ok(url)
}

fn web_url(url: &str) -> Result<()> {
    let (scheme, rest) = url
        .split_once("://")
        .context("browse needs http or https")?;
    anyhow::ensure!(
        (scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https"))
            && !rest.starts_with(['/', '?', '#'])
            && !rest.is_empty(),
        "browse accepts only http/https URLs with a host"
    );
    let parsed = gpui::http_client::Url::parse(url)?;
    anyhow::ensure!(parsed.has_host(), "browse URL needs a host");
    Ok(())
}

#[cfg(test)]
#[path = "control_tests.rs"]
mod transport_tests;
