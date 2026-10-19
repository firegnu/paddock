//! PTY sessions. From Saddle `src/pty.rs` at commit `df1c727`; paddock added `running_programs`,
//! so quitting asks only when something runs in a shell (DESIGN §13 P5-66), `keep` (P5-123), and
//! the pane's bell, its colours and input that queues however much comes (P5-143a).
use crate::{
    palette,
    terminal::{Screen, Size},
};
use anyhow::{Context, Result};
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::{
    io::{Read, Write},
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Sender},
    },
    thread,
    time::{Duration, Instant},
};

/// What wakes a terminal pane to look at its session again: output from the session it shows,
/// and changes to what it shows (a shell or attach done starting, a session that ended). Rings
/// that come before the pane looks are one ring.
pub struct Bell {
    tx: async_channel::Sender<()>,
    rx: async_channel::Receiver<()>,
    /// Whose rings count: a session handed an older number rings no more.
    current: Arc<AtomicU64>,
    /// Something besides output rang since the pane last looked.
    changed: Arc<AtomicBool>,
}
impl Default for Bell {
    fn default() -> Self {
        let (tx, rx) = async_channel::bounded(1);
        Self {
            tx,
            rx,
            current: Arc::default(),
            changed: Arc::default(),
        }
    }
}
impl Bell {
    pub fn new() -> Self {
        Self::default()
    }
    /// What the pane waits on.
    pub fn receiver(&self) -> async_channel::Receiver<()> {
        self.rx.clone()
    }
    /// For the session the pane shows from now on: the ones handed out before ring no more.
    pub fn for_session(&self) -> Ring {
        let mine = self.current.fetch_add(1, Ordering::SeqCst) + 1;
        self.ring(Some(mine))
    }
    /// The pane shows no session: the one handed out last rings no more.
    pub fn silence(&self) {
        self.current.fetch_add(1, Ordering::SeqCst);
    }
    /// For changes to what the pane shows, from whichever session or spawn.
    pub fn for_changes(&self) -> Ring {
        self.ring(None)
    }
    /// Has the pane look at everything again, at once.
    pub fn kick(&self) {
        self.for_changes().changed();
    }
    /// Whether anything besides output rang since this was last asked.
    pub fn take_changed(&self) -> bool {
        self.changed.swap(false, Ordering::SeqCst)
    }
    fn ring(&self, mine: Option<u64>) -> Ring {
        Ring {
            tx: self.tx.clone(),
            current: self.current.clone(),
            changed: self.changed.clone(),
            mine,
        }
    }
}
/// Rings a pane's [`Bell`].
#[derive(Clone)]
pub struct Ring {
    tx: async_channel::Sender<()>,
    current: Arc<AtomicU64>,
    changed: Arc<AtomicBool>,
    /// The session's number; none for a ring that always counts.
    mine: Option<u64>,
}
impl Ring {
    fn counts(&self) -> bool {
        self.mine
            .is_none_or(|mine| self.current.load(Ordering::SeqCst) == mine)
    }
    /// New output.
    pub fn output(&self) {
        if self.counts() {
            let _ = self.tx.try_send(());
        }
    }
    /// A change to what the pane shows.
    pub fn changed(&self) {
        if self.counts() {
            self.changed.store(true, Ordering::SeqCst);
            let _ = self.tx.try_send(());
        }
    }
}

/// Numbers sessions, for a pane to tell a new one from the one it had.
static SESSIONS: AtomicU64 = AtomicU64::new(1);

pub struct Session {
    pub screen: Arc<Mutex<Screen>>,
    id: u64,
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    /// To the thread that writes to the PTY, in order, however slowly the program reads.
    input: Sender<Vec<u8>>,
    /// Rung by the thread that reads the PTY.
    ring: Arc<Mutex<Option<Ring>>>,
    size: Size,
    stopping: Option<Instant>,
    exited: bool,
    exit_code: Option<u32>,
    shell: bool,
    groups: Vec<i32>,
    /// Let go once the child has ended (P5-123: an attached pane's hold on its machine's
    /// connection).
    _kept: Option<Box<dyn Send + Sync>>,
}
impl Session {
    /// `theme`: the pane's colours, which the program is told as they are when it asks.
    pub fn spawn(
        command: &[String],
        cwd: Option<&Path>,
        size: Size,
        theme: palette::Shared,
    ) -> Result<Self> {
        Self::spawn_owned(command, cwd, size, None, theme)
    }
    pub fn spawn_shell(
        command: &[String],
        cwd: &Path,
        size: Size,
        env: &[(String, String)],
        theme: palette::Shared,
    ) -> Result<Self> {
        Self::spawn_owned(command, Some(cwd), size, Some(env), theme)
    }
    fn spawn_owned(
        command: &[String],
        cwd: Option<&Path>,
        size: Size,
        env: Option<&[(String, String)]>,
        theme: palette::Shared,
    ) -> Result<Self> {
        let program = command.first().context("empty terminal command")?;
        let spawning = crate::command::spawn_guard();
        let pair = native_pty_system().openpty(pty_size(size))?;
        let mut cmd = CommandBuilder::new(program);
        cmd.args(&command[1..]);
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        if let Some(env) = env {
            for key in [
                "CORRAL_NAME",
                "CORRAL_INSTANCE",
                "SADDLE_INSTANCE",
                "SADDLE_PANE",
                "SADDLE_REVISION",
            ] {
                cmd.env_remove(key);
            }
            for (key, value) in env {
                cmd.env(key, value);
            }
        }
        if let Some(cwd) = cwd {
            cmd.cwd(cwd);
        }
        let mut reader = pair.master.try_clone_reader()?;
        let mut writer = pair.master.take_writer()?;
        let child = pair
            .slave
            .spawn_command(cmd)
            .with_context(|| format!("starting {program}"))?;
        drop(pair.slave);
        drop(spawning);
        let screen = Arc::new(Mutex::new(Screen::with_theme(size, theme)));
        // Unbounded: typing, pastes and replies wait their turn, none is turned away. What is
        // left once the program has gone is dropped with the thread.
        let (input, rx) = mpsc::channel::<Vec<u8>>();
        thread::spawn(move || {
            while let Ok(bytes) = rx.recv() {
                if writer
                    .write_all(&bytes)
                    .and_then(|_| writer.flush())
                    .is_err()
                {
                    break;
                }
            }
        });
        let output = screen.clone();
        let reply = input.clone();
        let ring: Arc<Mutex<Option<Ring>>> = Arc::default();
        let rings = ring.clone();
        thread::spawn(move || {
            let mut bytes = [0; 4096];
            loop {
                match reader.read(&mut bytes) {
                    Ok(0) => break,
                    Ok(n) => {
                        let replies = output.lock().unwrap().process(&bytes[..n]);
                        // Queued after what was typed before; never waits for it.
                        if !replies.is_empty() {
                            let _ = reply.send(replies);
                        }
                        if let Some(ring) = &*rings.lock().unwrap() {
                            ring.output();
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(_) => break,
                }
            }
            // The program has most likely ended.
            if let Some(ring) = &*rings.lock().unwrap() {
                ring.changed();
            }
        });
        Ok(Self {
            screen,
            id: SESSIONS.fetch_add(1, Ordering::Relaxed),
            master: pair.master,
            child,
            input,
            ring,
            size,
            stopping: None,
            exited: false,
            exit_code: None,
            shell: env.is_some(),
            groups: Vec::new(),
            _kept: None,
        })
    }
    /// This session among all of them.
    pub fn id(&self) -> u64 {
        self.id
    }
    /// Has `ring` rung when output comes and when the PTY closes, instead of any before.
    pub fn set_ring(&self, ring: Ring) {
        *self.ring.lock().unwrap() = Some(ring);
    }
    /// Keeps `thing` until the child has ended and the session is dropped.
    pub fn keep(&mut self, thing: impl Send + Sync + 'static) {
        self._kept = Some(Box::new(thing));
    }
    /// Queues `bytes` for the program, after what came before; never waits. Once the program has
    /// gone they are dropped.
    pub fn send(&self, bytes: Vec<u8>) {
        if !bytes.is_empty() {
            let _ = self.input.send(bytes);
        }
    }
    pub fn resize(&mut self, size: Size) -> Result<()> {
        if self.size.rows != size.rows || self.size.cols != size.cols {
            // Resize the parser before the child can redraw for the new size.
            self.screen.lock().unwrap().resize(size);
            self.master.resize(pty_size(size))?;
            self.size = size;
        }
        Ok(())
    }
    pub(crate) fn is_stopping(&self) -> bool {
        self.stopping.is_some()
    }
    pub fn exit_code(&self) -> Option<u32> {
        self.exit_code
    }
    pub fn running(&self) -> bool {
        !self.exited && !self.is_stopping()
    }
    /// What runs in this shell besides the shell itself, by name: the program in the foreground,
    /// then the shell's background jobs, which end with it too. Empty at the prompt, and for a
    /// session that is not a shell or is ending.
    pub fn running_programs(&self) -> Vec<String> {
        if !self.shell || !self.running() {
            return Vec::new();
        }
        let Some(shell) = self.child.process_id().map(|pid| pid as libc::pid_t) else {
            return Vec::new();
        };
        // Each job is a process group of its own; a pipeline is one job.
        let mut jobs: Vec<(libc::pid_t, libc::pid_t)> = Vec::new();
        if let Some(front) = self
            .master
            .process_group_leader()
            .filter(|group| *group > 1 && *group != shell)
        {
            jobs.push((front, front));
        }
        for child in children(shell) {
            // SAFETY: only reads the process group of the shell's own child.
            let group = unsafe { libc::getpgid(child) };
            // In the shell's own group: a helper of the shell's, such as a prompt's.
            if group > 1 && group != shell && jobs.iter().all(|(g, _)| *g != group) {
                jobs.push((group, child));
            }
        }
        let mut names: Vec<String> = Vec::new();
        for (group, pid) in jobs {
            // The group's leader names the job; it may have ended before the rest of a pipeline.
            let name = process_name(group)
                .or_else(|| process_name(pid))
                .unwrap_or_else(|| "a program".to_owned());
            if !names.contains(&name) {
                names.push(name);
            }
        }
        names
    }
    pub fn interrupt(&mut self) -> Result<()> {
        if self.stopping.is_none() && !self.poll_exit()? {
            if self.shell {
                if let Some(pid) = self.child.process_id() {
                    self.groups.push(pid as i32);
                }
                if let Some(group) = self.master.process_group_leader().filter(|p| *p > 1)
                    && !self.groups.contains(&group)
                {
                    self.groups.push(group);
                }
                self.signal_groups(libc::SIGHUP);
            } else {
                self.signal(libc::SIGHUP);
            }
            self.stopping = Some(Instant::now());
        }
        Ok(())
    }
    fn signal_groups(&self, signal: i32) {
        for group in &self.groups {
            // These are the process groups of this owned PTY, captured before shutdown.
            unsafe {
                libc::kill(-*group, signal);
            }
        }
    }
    fn signal(&self, signal: i32) {
        if let Some(pid) = self.child.process_id() {
            // Only this PTY child, never a corral agent or a name/path-based process group.
            unsafe {
                libc::kill(pid as i32, signal);
            }
        }
    }
    pub fn poll_exit(&mut self) -> Result<bool> {
        if self.exited {
            return Ok(true);
        }
        if let Some(status) = self.child.try_wait()? {
            self.exit_code = Some(status.exit_code());
            if self.shell && self.stopping.is_some() {
                self.signal_groups(libc::SIGKILL);
            }
            self.exited = true;
            return Ok(true);
        }
        if self
            .stopping
            .is_some_and(|start| start.elapsed() >= Duration::from_secs(3))
        {
            if self.shell {
                self.signal_groups(libc::SIGKILL);
            }
            self.signal(libc::SIGKILL);
        }
        Ok(false)
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.interrupt();
        while !self.poll_exit().unwrap_or(true) {
            thread::sleep(Duration::from_millis(10));
        }
    }
}

/// Starts every pane's hangup before dropping any: their three-second deadlines overlap.
pub fn shutdown(mut sessions: Vec<Session>) {
    for session in &mut sessions {
        let _ = session.interrupt();
    }
    drop(sessions);
}

/// The processes whose parent is `parent`.
fn children(parent: libc::pid_t) -> Vec<libc::pid_t> {
    let mut pids = [0 as libc::pid_t; 256];
    let bytes = std::mem::size_of_val(&pids) as libc::c_int;
    // SAFETY: the buffer is `bytes` long; the call returns how many pids it wrote.
    let count = unsafe { libc::proc_listchildpids(parent, pids.as_mut_ptr().cast(), bytes) };
    pids[..count.clamp(0, pids.len() as libc::c_int) as usize].to_vec()
}

/// A process's short name, such as `cargo`; `None` once it has ended.
fn process_name(pid: libc::pid_t) -> Option<String> {
    let mut name = [0u8; 256];
    // SAFETY: the buffer is as long as said; the call returns how many bytes it wrote.
    let length = unsafe { libc::proc_name(pid, name.as_mut_ptr().cast(), name.len() as u32) };
    (length > 0).then(|| String::from_utf8_lossy(&name[..length as usize]).into_owned())
}

fn pty_size(size: Size) -> PtySize {
    PtySize {
        rows: size.rows.max(1),
        cols: size.cols.max(2),
        pixel_width: 0,
        pixel_height: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alacritty_terminal::index::{Column, Line};

    fn ignoring(signals: &str) -> Session {
        let script = format!("trap '' {signals}; printf '\\033]2;ready\\007'; exec /bin/sleep 30");
        let command = ["/bin/zsh", "-f", "-c", &script].map(str::to_owned);
        let session = Session::spawn(
            &command,
            None,
            Size { rows: 24, cols: 80 },
            palette::Shared::default(),
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while session.screen.lock().unwrap().title() != Some("ready") {
            assert!(Instant::now() < deadline, "child did not install its traps");
            thread::sleep(Duration::from_millis(10));
        }
        session
    }

    #[test]
    fn dropping_a_command_that_ignores_sigint_hangs_up_promptly() {
        let session = ignoring("INT");
        let start = Instant::now();
        drop(session);
        let elapsed = start.elapsed();
        assert!(elapsed < Duration::from_secs(1), "drop took {elapsed:?}");
    }

    #[test]
    fn quitting_commands_that_ignore_sigint_hangs_up_promptly() {
        let sessions = (0..3).map(|_| ignoring("INT")).collect();
        let start = Instant::now();
        shutdown(sessions);
        let elapsed = start.elapsed();
        assert!(elapsed < Duration::from_secs(1), "quit took {elapsed:?}");
    }

    #[test]
    fn quitting_stubborn_commands_shares_one_kill_deadline() {
        let sessions = (0..3).map(|_| ignoring("INT HUP")).collect();
        let start = Instant::now();
        shutdown(sessions);
        let elapsed = start.elapsed();
        assert!(
            elapsed >= Duration::from_secs(3),
            "killed early: {elapsed:?}"
        );
        assert!(elapsed < Duration::from_secs(5), "quit took {elapsed:?}");
    }

    #[test]
    fn dropping_a_command_that_ignores_sighup_keeps_the_three_second_kill_fallback() {
        let session = ignoring("INT HUP");
        let start = Instant::now();
        drop(session);
        let elapsed = start.elapsed();
        assert!(
            elapsed >= Duration::from_secs(3),
            "killed early: {elapsed:?}"
        );
        assert!(elapsed < Duration::from_secs(5), "drop took {elapsed:?}");
    }

    #[test]
    fn quitting_an_already_exited_command_does_not_wait() {
        let command = ["/bin/zsh", "-f", "-c", "exit 7"].map(str::to_owned);
        let mut session = Session::spawn(
            &command,
            None,
            Size { rows: 24, cols: 80 },
            palette::Shared::default(),
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !session.poll_exit().unwrap() {
            assert!(Instant::now() < deadline, "child did not exit");
            thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(session.exit_code(), Some(7));
        let start = Instant::now();
        shutdown(vec![session]);
        let elapsed = start.elapsed();
        assert!(elapsed < Duration::from_secs(1), "quit took {elapsed:?}");
    }

    #[test]
    fn dropping_a_shell_still_ends_its_foreground_job_group() {
        let session = shell();
        session.send(b"/bin/sleep 30\n".to_vec());
        assert_eq!(settled(&session, &["sleep"]), ["sleep"]);
        let group = session.master.process_group_leader().unwrap();
        assert!(group > 1);
        assert_ne!(Some(group as u32), session.child.process_id());
        let start = Instant::now();
        drop(session);
        let elapsed = start.elapsed();
        assert!(elapsed < Duration::from_secs(1), "drop took {elapsed:?}");
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            // SAFETY: signal 0 only checks this test's own PTY job group; it sends no signal.
            if unsafe { libc::kill(-group, 0) } == -1
                && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
            {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "shell's job group is still alive"
            );
            thread::sleep(Duration::from_millis(10));
        }
    }

    /// zsh without the user's startup files, interactive so it runs jobs in their own groups.
    fn shell() -> Session {
        let command = ["/bin/zsh", "-f", "-i"].map(str::to_owned);
        let size = Size { rows: 24, cols: 80 };
        Session::spawn_shell(
            &command,
            &std::env::temp_dir(),
            size,
            &[],
            palette::Shared::default(),
        )
        .unwrap()
    }

    /// What runs in `session` once it is `want`, or after five seconds whatever it is then.
    fn settled(session: &Session, want: &[&str]) -> Vec<String> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let now = session.running_programs();
            if now == want || Instant::now() > deadline {
                return now;
            }
            thread::sleep(Duration::from_millis(50));
        }
    }

    #[test]
    fn a_shell_runs_nothing_at_its_prompt_and_names_a_command_or_a_job() {
        let session = shell();
        thread::sleep(Duration::from_millis(500));
        assert!(settled(&session, &[]).is_empty());
        // In the foreground.
        session.send(b"sleep 30\n".to_vec());
        assert_eq!(settled(&session, &["sleep"]), ["sleep"]);
        session.send(vec![3]);
        assert!(settled(&session, &[]).is_empty());
        // In the background, which quitting would end too.
        session.send(b"sleep 31 &\n".to_vec());
        assert_eq!(settled(&session, &["sleep"]), ["sleep"]);
        session.send(b"kill %1\n".to_vec());
        assert!(settled(&session, &[]).is_empty());
    }

    /// A scratch file of the test's own.
    fn scratch(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("paddock-pty-{name}-{}", std::process::id()))
    }

    /// Waits up to `within` for a ring, taking it.
    fn rang(rings: &async_channel::Receiver<()>, within: Duration) -> bool {
        let deadline = Instant::now() + within;
        while Instant::now() < deadline {
            if rings.try_recv().is_ok() {
                return true;
            }
            thread::sleep(Duration::from_millis(5));
        }
        false
    }

    #[test]
    fn input_sent_while_the_program_reads_nothing_all_arrives_in_order() {
        let out = scratch("input-order");
        let _ = std::fs::remove_file(&out);
        // Far more than the PTY holds, so the writer is stuck on it while the rest comes.
        let blob = vec![b'x'; 64 * 1024];
        let numbers: Vec<Vec<u8>> = (0..150).map(|i| format!("<{i}>").into_bytes()).collect();
        let total = blob.len() + numbers.iter().map(Vec::len).sum::<usize>();
        let script = format!(
            "stty raw -echo; printf '\\033]2;ready\\007'; sleep 1; head -c {total} > '{}'; \
             printf '\\033]2;done\\007'; exec /bin/sleep 30",
            out.display()
        );
        let command = ["/bin/sh", "-c", &script].map(str::to_owned);
        let session = Session::spawn(
            &command,
            None,
            Size { rows: 24, cols: 80 },
            palette::Shared::default(),
        )
        .unwrap();
        let wait_title = |want: &str, within: Duration| {
            let deadline = Instant::now() + within;
            while session.screen.lock().unwrap().title() != Some(want) {
                assert!(Instant::now() < deadline, "never titled {want}");
                thread::sleep(Duration::from_millis(10));
            }
        };
        wait_title("ready", Duration::from_secs(5));
        let start = Instant::now();
        session.send(blob.clone());
        // Sending can no longer turn input away (from the 66th on it was "busy").
        for number in &numbers {
            session.send(number.clone());
        }
        assert!(
            start.elapsed() < Duration::from_millis(500),
            "sending waited"
        );
        wait_title("done", Duration::from_secs(10));
        let got = std::fs::read(&out).unwrap();
        let _ = std::fs::remove_file(&out);
        let want: Vec<u8> = std::iter::once(blob).chain(numbers).flatten().collect();
        assert_eq!(got.len(), want.len());
        assert!(got == want, "input arrived out of order");
    }

    #[test]
    fn output_rings_the_pane_and_a_session_it_moved_on_from_no_longer_does() {
        let bell = Bell::new();
        let rings = bell.receiver();
        let script = "sleep 0.3; printf hi; sleep 0.7; printf again; exec /bin/sleep 30";
        let command = ["/bin/sh", "-c", script].map(str::to_owned);
        let session = Session::spawn(
            &command,
            None,
            Size { rows: 24, cols: 80 },
            palette::Shared::default(),
        )
        .unwrap();
        session.set_ring(bell.for_session());
        assert!(rang(&rings, Duration::from_secs(3)), "output did not ring");
        // The pane shows another session from now on.
        let _next = bell.for_session();
        while rings.try_recv().is_ok() {}
        let deadline = Instant::now() + Duration::from_secs(3);
        while !session.screen.lock().unwrap().term.grid()[Line(0)][Column(6)]
            .c
            .is_alphabetic()
        {
            assert!(Instant::now() < deadline, "no second output");
            thread::sleep(Duration::from_millis(10));
        }
        assert!(
            !rang(&rings, Duration::from_millis(200)),
            "an old session still rang"
        );
    }

    #[test]
    fn a_session_whose_program_ends_rings_a_change() {
        let bell = Bell::new();
        let rings = bell.receiver();
        let command = ["/bin/sh", "-c", "sleep 0.3"].map(str::to_owned);
        let size = Size { rows: 24, cols: 80 };
        let session = Session::spawn(&command, None, size, palette::Shared::default()).unwrap();
        session.set_ring(bell.for_session());
        assert!(rang(&rings, Duration::from_secs(3)), "the end did not ring");
        assert!(bell.take_changed(), "rang as output only");
        assert!(!bell.take_changed());
    }

    #[test]
    fn what_a_pane_attaches_to_is_not_a_shell() {
        let command = ["/bin/sleep", "30"].map(str::to_owned);
        let session = Session::spawn(
            &command,
            None,
            Size { rows: 24, cols: 80 },
            palette::Shared::default(),
        )
        .unwrap();
        thread::sleep(Duration::from_millis(300));
        assert!(session.running_programs().is_empty());
    }
}
