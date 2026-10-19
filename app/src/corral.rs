//! The `corral` command: one-off JSON calls and the background `ls`/`status` poller. From Saddle
//! `src/corral.rs` at commit `df1c727`; paddock added the background reader of last replies.
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::Value;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Agent {
    pub name: String,
    pub kind: Option<String>,
    pub instance: Option<String>,
    pub cwd: Option<String>,
    pub starting: bool,
    pub incompatible: bool,
    pub proto: Option<u64>,
    pub state: Option<String>,
    pub last_tool: Option<String>,
    pub turn_started: Option<f64>,
    pub state_started: Option<f64>,
    pub last_output: Option<f64>,
    pub idle_for: Option<f64>,
    pub attached: usize,
    pub last_input_source: Option<String>,
    /// When the last input arrived, from `corral status`.
    pub last_input_at: Option<f64>,
    pub title: Option<String>,
    pub started: Option<f64>,
    pub error: Option<String>,
    pub labels: serde_json::Map<String, Value>,
    /// Frozen by `corral pause` (DESIGN §13 P5-33); an older corral without the field never is.
    pub paused: bool,
    /// When it was paused.
    pub paused_at: Option<f64>,
    /// The fence profile it runs in (DESIGN §13 P5-120); none without one, or from an older
    /// corral.
    pub sandbox: Option<Sandbox>,
}

/// `sandbox` in `corral ls` and `corral status`: the profile the agent was started in.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Sandbox {
    pub profile: String,
}

/// Effort explicitly labelled when the agent was delegated; not the runtime's actual effort.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effort {
    Medium,
    High,
    Xhigh,
}

/// Role from the public `role` label; never inferred from the agent's name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Controller,
    Regular,
    Implementer,
    Reviewer,
}

impl Agent {
    pub fn role(&self) -> Option<Role> {
        match self.labels.get("role")?.as_str()? {
            "controller" => Some(Role::Controller),
            "regular" => Some(Role::Regular),
            "implementer" => Some(Role::Implementer),
            "reviewer" => Some(Role::Reviewer),
            _ => None,
        }
    }
    pub fn effort(&self) -> Option<Effort> {
        match self.labels.get("effort")?.as_str()? {
            "medium" => Some(Effort::Medium),
            "high" => Some(Effort::High),
            "xhigh" => Some(Effort::Xhigh),
            _ => None,
        }
    }
}

/// How long one `corral send` may take: corral itself waits up to 15 seconds for the agent to take
/// the text.
const SEND_TIMEOUT: Duration = Duration::from_secs(30);

/// corral's JSON answer to `args`, as `output` holds it; an error when it is not JSON, corral
/// failed, or it said `ok: false`.
pub fn answer(args: &[&str], output: &crate::command::Output) -> Result<Value> {
    let value: Value = serde_json::from_slice(&output.stdout).with_context(|| {
        format!(
            "{}: invalid JSON ({})",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )
    })?;
    checked(args, output.status.success(), value)
}

/// `value` unless corral failed (`success` false) or said `ok: false`: then its error.
pub fn checked(args: &[&str], success: bool, value: Value) -> Result<Value> {
    if !success || value.get("ok") == Some(&Value::Bool(false)) {
        bail!(
            "{}: {}",
            args.join(" "),
            value
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("command failed")
        );
    }
    Ok(value)
}

/// Takes `agent`'s `corral status` into what `corral ls` said of it: the status in full, with
/// the listing's directory and, when the status has none, its instance; or why it failed.
pub fn merge(agent: &mut Agent, status: Result<Value>) {
    match status.and_then(|v| Ok(serde_json::from_value::<Agent>(v)?)) {
        Ok(mut status) => {
            status.name.clone_from(&agent.name);
            status.cwd = agent.cwd.take();
            if status.instance.is_none() {
                status.instance = agent.instance.take();
            }
            *agent = status;
        }
        Err(error) => agent.error = Some(error.to_string()),
    }
}

/// corral on this Mac (`program`), or on another machine (`remote`, DESIGN §13 P5-90), where
/// `program` is not used.
#[derive(Clone)]
pub struct Client {
    pub program: String,
    pub remote: Option<Arc<crate::remote::Link>>,
}
impl Client {
    /// corral on this Mac.
    pub fn local(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            remote: None,
        }
    }

    /// Runs corral with `args`, here or there.
    fn output(
        &self,
        args: &[&str],
        timeout: Duration,
        cancel: &AtomicBool,
    ) -> Result<crate::command::Output> {
        match &self.remote {
            None => crate::command::run(&self.program, args, None, timeout, cancel),
            Some(link) => link
                .corral(args, timeout, cancel)
                .map_err(|problem| anyhow::anyhow!("{}: {problem}", link.label())),
        }
    }

    pub fn json(&self, args: &[&str], timeout: Duration, cancel: &AtomicBool) -> Result<Value> {
        answer(args, &self.output(args, timeout, cancel)?)
    }

    pub fn collect(&self) -> Result<Vec<Agent>> {
        self.collect_until(&AtomicBool::new(false))
    }

    pub fn collect_until(&self, cancel: &AtomicBool) -> Result<Vec<Agent>> {
        if let Some(link) = &self.remote {
            return crate::remote::collect(link, cancel).map_err(|fault| match fault {
                crate::remote::Fault::Ssh(problem) => anyhow::anyhow!("{problem}"),
                crate::remote::Fault::Corral(error) => error,
            });
        }
        #[derive(Deserialize)]
        struct Listing {
            agents: Vec<Agent>,
        }
        let listing: Listing =
            serde_json::from_value(self.json(&["ls"], Duration::from_secs(15), cancel)?)?;
        let mut agents = listing.agents;
        for agent in &mut agents {
            if cancel.load(Ordering::Relaxed) {
                bail!("cancelled");
            }
            if agent.starting || agent.incompatible {
                continue;
            }
            let status = self.json(&["status", &agent.name], Duration::from_secs(15), cancel);
            merge(agent, status);
        }
        agents.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(agents)
    }

    /// The agent's last reply as `corral reply` gives it; `None` when it has none yet or it can't
    /// be read.
    pub fn reply(&self, name: &str, cancel: &AtomicBool) -> Option<Reply> {
        let value = self
            .json(&["reply", name], Duration::from_secs(15), cancel)
            .ok()?;
        Some(Reply {
            text: value.get("text")?.as_str()?.to_owned(),
            at: value.get("at").and_then(Value::as_f64),
        })
    }

    /// Sends `text` to `name` as the Inbox's reply box has it (DESIGN §13 P5-87):
    /// `corral send <name> <text>`, the text one argument and untouched, never forced and never
    /// held back for later.
    pub fn send(&self, name: &str, text: &str, cancel: &AtomicBool) -> Result<(), Refused> {
        let unanswered = |message: String| Refused {
            code: None,
            message,
        };
        let args = ["send", name, text];
        let output = self
            .output(&args, SEND_TIMEOUT, cancel)
            // Not the command line, which has the whole reply in it: only what went wrong.
            .map_err(|error| match error.chain().nth(1) {
                _ if self.remote.is_some() => unanswered(format!("{error}")),
                Some(cause) => unanswered(format!("corral did not run: {cause}")),
                None => unanswered("corral did not answer in time".into()),
            })?;
        let value: Value = serde_json::from_slice(&output.stdout)
            .map_err(|_| unanswered("corral did not say how it went".into()))?;
        if output.status.success() && value.get("ok") != Some(&Value::Bool(false)) {
            return Ok(());
        }
        let said = |key: &str| value.get(key).and_then(Value::as_str).map(str::to_owned);
        let code = said("error");
        let message = said("message")
            .or_else(|| code.clone())
            .unwrap_or_else(|| "corral send failed".into());
        Err(Refused { code, message })
    }
}

/// An agent's last reply: its text as written, and when it was said.
#[derive(Clone, Debug, PartialEq)]
pub struct Reply {
    pub text: String,
    pub at: Option<f64>,
}

impl From<String> for Reply {
    fn from(text: String) -> Self {
        Reply { text, at: None }
    }
}

/// Why a `corral send` did not go through: corral's word for it (`not_idle`, `human_active`…;
/// none when corral could not be run or did not answer in its JSON) and its sentence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refused {
    pub code: Option<String>,
    pub message: String,
}

/// Reads agents' last replies in the background, one at a time, each when asked; `K` says which
/// request a result answers.
pub struct Replies<K> {
    pub updates: mpsc::Receiver<(K, Option<Reply>)>,
    asks: Option<mpsc::Sender<(K, String)>>,
    cancel: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}
impl<K: Send + 'static> Replies<K> {
    pub fn start(client: Client) -> Self {
        let (tx, updates) = mpsc::channel();
        let (asks, wait) = mpsc::channel::<(K, String)>();
        let cancel = Arc::new(AtomicBool::new(false));
        let quitting = cancel.clone();
        let worker = thread::spawn(move || {
            for (key, name) in wait {
                if quitting.load(Ordering::Relaxed) {
                    break;
                }
                if tx.send((key, client.reply(&name, &quitting))).is_err() {
                    break;
                }
            }
        });
        Self {
            updates,
            asks: Some(asks),
            cancel,
            worker: Some(worker),
        }
    }
    /// Reads `name`'s last reply, answered as `key`.
    pub fn ask(&self, key: K, name: String) {
        if let Some(asks) = &self.asks {
            let _ = asks.send((key, name));
        }
    }
}
impl<K> Drop for Replies<K> {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        self.asks.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

pub struct Poller {
    /// Each read, with when it began.
    pub updates: mpsc::Receiver<(Instant, Result<Vec<Agent>>)>,
    cancel: Arc<AtomicBool>,
    wake: mpsc::Sender<()>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Poller {
    pub fn start(client: Client, every: Duration) -> Self {
        Self::spawn(every, move |_, quitting| {
            Some(client.collect_until(quitting))
        })
    }

    /// Another machine's agents (DESIGN §13 P5-90): its own thread, looking every
    /// [`crate::remote::TICK`] whether a read is due.
    pub fn remote(link: Arc<crate::remote::Link>) -> Self {
        let mut rounds = crate::remote::Rounds::new(link);
        Self::spawn(crate::remote::TICK, move |forced, quitting| {
            rounds.round(forced, quitting)
        })
    }

    /// A thread calling `round` every `tick`, and at once when asked to refresh (`true`), sending
    /// on what it gives.
    fn spawn(
        tick: Duration,
        mut round: impl FnMut(bool, &AtomicBool) -> Option<Result<Vec<Agent>>> + Send + 'static,
    ) -> Self {
        let (tx, updates) = mpsc::channel();
        let (wake, wait) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let quitting = cancel.clone();
        let worker = thread::spawn(move || {
            let mut forced = true;
            while !quitting.load(Ordering::Relaxed) {
                let began = Instant::now();
                if let Some(update) = round(forced, &quitting)
                    && tx.send((began, update)).is_err()
                {
                    break;
                }
                forced = wait.recv_timeout(tick).is_ok();
            }
        });
        Self {
            updates,
            cancel,
            wake,
            worker: Some(worker),
        }
    }
    pub fn refresh(&self) {
        let _ = self.wake.send(());
    }
}
impl Drop for Poller {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        self.refresh();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
