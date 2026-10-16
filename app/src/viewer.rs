//! The shell / `corral attach` lifecycle of one terminal pane. From Saddle `src/viewer.rs` at
//! commit `df1c727`, without the `remembered` field that only Saddle's layout saving uses; paddock
//! added `paused` and `send`, which drops input for a paused agent (DESIGN §13 P5-33).
use crate::{corral::Client, pty::Session, terminal::Size};
use anyhow::Result;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};

pub struct Shell {
    pub program: String,
    pub cwd: String,
    pub state: &'static str,
    pub exit_code: Option<u32>,
    pub env: Vec<(String, String)>,
    pub agent_shell: bool,
}

#[derive(Clone, Default)]
pub struct AgentMetadata {
    pub cwd: Option<String>,
    pub instance: Option<String>,
}

/// What public corral says about `name` before `paddock --attach NAME` attaches to it: the instance
/// to attach to (the attach checks it again, as for any agent with a known instance). Nothing
/// when corral cannot say; the pane then attaches as before, but cannot stand for its agent in
/// `paddock ctl`.
pub fn public_metadata(corral: &str, name: &str) -> AgentMetadata {
    let status = crate::corral::Client::local(corral).json(
        &["status", name],
        std::time::Duration::from_secs(5),
        &std::sync::atomic::AtomicBool::new(false),
    );
    AgentMetadata {
        cwd: None,
        instance: status.ok().and_then(|status| {
            status["instance"]
                .as_str()
                .filter(|instance| !instance.is_empty())
                .map(str::to_owned)
        }),
    }
}

/// What the check before an attach says when corral reports the agent ended, or another instance
/// of it running.
const EXITED: &str = "agent has exited before attach";
const REPLACED: &str = "agent identity changed before attach";

pub struct Viewer {
    pub session: Option<Session>,
    pub showing: Option<String>,
    pub note: String,
    pub shell: Option<Shell>,
    pub metadata: AgentMetadata,
    pub exit_code: Option<u32>,
    /// The agent shown is paused (DESIGN §13 P5-33), as the window last heard from corral: its input
    /// is dropped.
    pub paused: bool,
    /// Another machine's agent shown while another window there is attached to it: corral lets
    /// this one only watch until that one leaves (DESIGN §13 P5-90).
    pub read_only: bool,
    /// Another machine's agent whose ssh connection was lost (ssh exited 255): attached again once
    /// that machine lists it as before.
    pub lost: Option<(String, AgentMetadata)>,
    failed: bool,
    closing: bool,
    pending: Option<(String, AgentMetadata)>,
    generation: u64,
    /// corral, on this Mac or another machine.
    client: Client,
    /// The attach under way found another window attached there.
    elsewhere: Arc<AtomicBool>,
    spawning: Option<(u64, String, JoinHandle<Result<Session>>)>,
}
impl Viewer {
    /// Attaches with this Mac's `corral`.
    pub fn new(corral: String) -> Self {
        Self::with(Client::local(corral))
    }

    /// Attaches with `client`'s corral, on this Mac or another machine.
    pub fn with(client: Client) -> Self {
        Self {
            session: None,
            showing: None,
            pending: None,
            generation: 0,
            client,
            elsewhere: Arc::default(),
            read_only: false,
            lost: None,
            spawning: None,
            shell: None,
            metadata: AgentMetadata::default(),
            exit_code: None,
            paused: false,
            failed: false,
            closing: false,
            note: "Select an agent on the left, then press Enter or click.".into(),
        }
    }
    pub fn select(&mut self, name: String) -> Result<()> {
        let metadata = if self.showing.as_ref() == Some(&name) {
            self.metadata.clone()
        } else {
            AgentMetadata::default()
        };
        self.select_agent(name, metadata)
    }
    pub fn select_agent(&mut self, name: String, metadata: AgentMetadata) -> Result<()> {
        self.closing = false;
        if self.showing.as_ref() == Some(&name)
            && self
                .session
                .as_ref()
                .is_some_and(|session| !session.is_stopping())
        {
            return Ok(());
        }
        // Another agent's input flows until the window says it is paused too.
        if self.target() != Some(name.as_str()) {
            self.paused = false;
        }
        self.cancel_pending();
        self.pending = Some((name, metadata));
        self.exit_code = None;
        self.failed = false;
        self.lost = None;
        self.read_only = false;
        if let Some(session) = &mut self.session {
            session.interrupt()?;
        }
        Ok(())
    }
    pub fn disappeared(&mut self, names: &[&str]) -> Result<()> {
        if self.shell.is_some() {
            return Ok(());
        }
        if self
            .pending
            .as_ref()
            .is_some_and(|(name, _)| !names.contains(&name.as_str()))
        {
            self.cancel_pending();
        }
        if self
            .showing
            .as_ref()
            .is_some_and(|name| !names.contains(&name.as_str()))
            && let Some(session) = &mut self.session
        {
            session.interrupt()?;
        }
        Ok(())
    }
    /// Another machine's listing, just read (DESIGN §13 P5-90): an attach whose connection was
    /// lost goes again when the agent is there as before; one that only watches can type once no
    /// other window is attached (corral hands it the keys).
    pub fn listed(&mut self, agents: &[crate::corral::Agent]) -> Result<()> {
        if let Some((name, metadata)) = &self.lost
            && self.pending.is_none()
            && self.session.is_none()
            && agents.iter().any(|a| {
                &a.name == name
                    && a.state.as_deref() != Some("exited")
                    && (metadata.instance.is_none() || a.instance == metadata.instance)
            })
        {
            let (name, metadata) = self.lost.take().expect("checked");
            self.select_agent(name, metadata)?;
        }
        if self.read_only
            && let Some(name) = &self.showing
            && agents.iter().any(|a| &a.name == name && a.attached <= 1)
        {
            self.read_only = false;
        }
        Ok(())
    }
    /// Another machine's agent from the saved layout: attached once that machine lists it as
    /// before ([`Self::listed`]).
    pub fn wait(&mut self, name: String, metadata: AgentMetadata, note: String) {
        self.metadata = metadata.clone();
        self.lost = Some((name, metadata));
        self.note = note;
    }
    pub fn target(&self) -> Option<&str> {
        self.pending
            .as_ref()
            .map(|(name, _)| name.as_str())
            .or(self.showing.as_deref())
    }
    pub fn target_metadata(&self) -> &AgentMetadata {
        self.pending
            .as_ref()
            .map_or(&self.metadata, |(_, metadata)| metadata)
    }
    /// Invalidate pending work without disconnecting the currently displayed session.
    pub fn cancel_pending(&mut self) {
        self.pending = None;
        self.generation += 1;
    }
    pub fn close(&mut self) -> Result<()> {
        self.closing = true;
        self.paused = false;
        self.lost = None;
        self.read_only = false;
        self.cancel_pending();
        if let Some(session) = &mut self.session {
            session.interrupt()?;
        }
        Ok(())
    }
    pub fn closed(&self) -> bool {
        self.session.is_none() && self.spawning.is_none() && self.pending.is_none()
    }
    pub fn tick(&mut self, size: Size) -> Result<()> {
        self.tick_visible(Some(size))
    }
    pub fn tick_visible(&mut self, size: Option<Size>) -> Result<()> {
        self.poll_session(size)?;
        if self
            .spawning
            .as_ref()
            .is_some_and(|(_, _, worker)| worker.is_finished())
        {
            let (generation, name, worker) = self.spawning.take().unwrap();
            let shell_spawn = name.is_empty();
            let current = generation == self.generation
                && !self.closing
                && if shell_spawn {
                    self.shell.as_ref().is_some_and(|s| s.state == "starting")
                } else {
                    self.pending
                        .as_ref()
                        .is_some_and(|(target, _)| target == &name)
                };
            match worker.join().expect("attach worker panicked") {
                Ok(mut session) => {
                    if current {
                        self.exit_code = None;
                        self.failed = false;
                        let pending = self.pending.take();
                        if shell_spawn {
                            self.shell.as_mut().unwrap().state = "running";
                        } else {
                            self.shell = None;
                            self.showing = Some(name);
                            self.read_only = self.elsewhere.load(Ordering::Relaxed);
                            if !session.poll_exit()? {
                                self.metadata = pending.unwrap().1;
                            }
                        }
                    } else {
                        // Keep ownership until the stale child exits, without naming it
                        // as the current agent or allowing it to receive input.
                        session.interrupt()?;
                    }
                    self.session = Some(session);
                    // The returned PTY may already have exited; never publish it as running.
                    self.poll_session(size)?;
                }
                Err(error) if current => {
                    self.pending = None;
                    self.exit_code = None;
                    if shell_spawn {
                        self.shell.as_mut().unwrap().state = "failed";
                    }
                    // corral said the agent has ended: no failure, as the window closes the pane.
                    self.failed = !matches!(error.to_string().as_str(), EXITED | REPLACED);
                    self.note = format!("terminal {name}: {error:#}");
                }
                Err(_) => {}
            }
        }
        if self.session.is_none()
            && self.spawning.is_none()
            && let Some((name, metadata)) = &self.pending
        {
            let name = name.clone();
            let instance = metadata.instance.clone();
            let checked_name = name.clone();
            let client = self.client.clone();
            let size = size.unwrap_or(Size { rows: 24, cols: 80 });
            let elsewhere = Arc::new(AtomicBool::new(false));
            self.elsewhere = elsewhere.clone();
            self.spawning = Some((
                self.generation,
                name,
                thread::spawn(move || {
                    let command = match &client.remote {
                        None => vec![
                            client.program.clone(),
                            "attach".into(),
                            checked_name.clone(),
                        ],
                        Some(link) => link.attach_argv(&checked_name)?,
                    };
                    if let Some(instance) = instance {
                        let status = client.json(
                            &["status", &checked_name],
                            std::time::Duration::from_secs(15),
                            &AtomicBool::new(false),
                        )?;
                        anyhow::ensure!(status["state"].as_str() != Some("exited"), EXITED);
                        anyhow::ensure!(status["instance"].as_str() == Some(&instance), REPLACED);
                        let attached = status["attached"].as_u64().unwrap_or(0) != 0;
                        // Another machine's agent is watched until the other window leaves; this
                        // Mac's is refused, as before.
                        anyhow::ensure!(
                            !attached || client.remote.is_some(),
                            "agent attached elsewhere"
                        );
                        elsewhere.store(attached, Ordering::Relaxed);
                    }
                    Session::spawn(&command, None, size)
                }),
            ));
        }
        if self.session.is_none()
            && self.spawning.is_none()
            && !self.closing
            && let Some(shell) = self.shell.as_ref().filter(|s| s.state == "starting")
        {
            let command = vec![shell.program.clone(), "-i".into()];
            let cwd = std::path::PathBuf::from(&shell.cwd);
            let env = shell.env.clone();
            let agent_shell = shell.agent_shell;
            let corral = self.client.program.clone();
            let size = size.unwrap_or(Size { rows: 24, cols: 80 });
            self.spawning = Some((
                self.generation,
                String::new(),
                thread::spawn(move || {
                    let (command, env) = if agent_shell {
                        let dir = crate::control::runtime_dir()?
                            .join(format!("agent-shell-{}", std::process::id()));
                        let original = std::env::var("ZDOTDIR").ok();
                        let setup = crate::agent_shell::setup(
                            &command[0],
                            &dir,
                            &std::env::current_exe()?,
                            original.as_deref(),
                        )?;
                        let mut env = env;
                        env.extend(setup.env);
                        env.push(("PADDOCK_AGENT_CORRAL".into(), corral));
                        (setup.command, env)
                    } else {
                        (command, env)
                    };
                    Session::spawn_shell(&command, &cwd, size, &env)
                }),
            ));
        }
        Ok(())
    }
    fn poll_session(&mut self, size: Option<Size>) -> Result<()> {
        if let Some(session) = &mut self.session {
            if session.poll_exit()? {
                let exit_code = session.exit_code();
                if let Some(shell) = &mut self.shell {
                    shell.state = "exited";
                    shell.exit_code = session.exit_code();
                    self.note = format!("Terminal exited ({})", shell.exit_code.unwrap_or(0));
                }
                if self.shell.is_none() || self.closing {
                    self.session = None;
                }
                if let Some(name) = self.showing.take() {
                    self.exit_code = exit_code;
                    self.read_only = false;
                    match &self.client.remote {
                        // ssh lost the connection, not the agent: attached again once its machine
                        // answers.
                        Some(link)
                            if exit_code == Some(crate::remote::SSH_FAILED) && !self.closing =>
                        {
                            self.failed = false;
                            self.note = format!("Lost connection to {} — reconnecting…", link.name);
                            self.lost = Some((name, self.metadata.clone()));
                        }
                        _ => {
                            self.failed = exit_code.is_some_and(|code| code != 0);
                            self.note = format!(
                                "{name} attach exited ({}). Select an agent on the left to reconnect.",
                                exit_code.unwrap_or(0)
                            );
                        }
                    }
                }
            } else if let Some(size) = size {
                session.resize(size)?;
            }
        }
        Ok(())
    }
    pub fn start_shell(&mut self, shell: Shell) {
        self.cancel_pending();
        self.closing = false;
        self.paused = false;
        self.metadata = AgentMetadata {
            cwd: Some(shell.cwd.clone()),
            instance: None,
        };
        self.exit_code = None;
        self.failed = false;
        self.showing = None;
        self.shell = Some(shell);
        self.note = "Starting terminal…".into();
    }
    /// Sends input to what the pane shows, unless its agent is paused: then the input is dropped,
    /// never kept to send later, as corral drops what is typed into a paused agent. Whether it was
    /// sent.
    pub fn send(&self, bytes: Vec<u8>) -> Result<bool> {
        let Some(session) = &self.session else {
            anyhow::bail!("no running session");
        };
        if self.paused {
            return Ok(false);
        }
        session.send(bytes)?;
        Ok(true)
    }
    pub fn shell_live(&self) -> bool {
        self.shell
            .as_ref()
            .is_some_and(|s| matches!(s.state, "starting" | "running"))
    }
    /// What runs in the shell besides the shell itself, by name; empty at its prompt, or without
    /// a running shell.
    pub fn shell_programs(&self) -> Vec<String> {
        match &self.session {
            Some(session) if self.shell_live() => session.running_programs(),
            _ => Vec::new(),
        }
    }
    pub fn state(&self) -> &'static str {
        if let Some(shell) = &self.shell {
            return shell.state;
        }
        if self.pending.is_some() || self.spawning.is_some() {
            "attaching"
        } else if self.session.as_ref().is_some_and(Session::running) && self.showing.is_some() {
            "running"
        } else if self.failed {
            "failed"
        } else {
            "disconnected"
        }
    }
}
impl Drop for Viewer {
    fn drop(&mut self) {
        // A spawn already in flight still owns its PTY; join and detach it on exit.
        if let Some((_, _, worker)) = self.spawning.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn already_exited_attach_worker_is_never_published_as_running() {
        let size = Size { rows: 10, cols: 40 };
        let mut viewer = Viewer::new("unused".to_owned());
        viewer.pending = Some(("p/failed".into(), AgentMetadata::default()));
        viewer.spawning = Some((
            0,
            "p/failed".into(),
            thread::spawn(move || {
                let mut session = Session::spawn(
                    &["/bin/sh".into(), "-c".into(), "exit 7".into()],
                    None,
                    size,
                )?;
                let deadline = Instant::now() + Duration::from_secs(3);
                while !session.poll_exit()? {
                    assert!(Instant::now() < deadline);
                    thread::yield_now();
                }
                Ok(session)
            }),
        ));
        let deadline = Instant::now() + Duration::from_secs(3);
        while !viewer.spawning.as_ref().unwrap().2.is_finished() {
            assert!(Instant::now() < deadline);
            thread::yield_now();
        }
        viewer.tick(size).unwrap();
        assert_eq!(viewer.state(), "failed");
        assert_eq!(viewer.exit_code, Some(7));
        assert!(viewer.session.is_none());
        assert!(viewer.showing.is_none());
    }
}
