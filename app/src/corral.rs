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
    /// The fence profile the agents it starts from now on run in (DESIGN §13 P5-141): `None`
    /// when corral is too old to say, `Some(None)` when it starts them outside.
    #[serde(deserialize_with = "present")]
    pub child_sandbox: Option<Option<Sandbox>>,
}

/// A field that is there, `null` or not: told apart from one left out, which `#[serde(default)]`
/// makes `None`.
fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
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
        self.deliver(name, text, cancel).map(|_| ())
    }

    /// [`Client::send`], and what corral said of a send it took: whether text already in the
    /// agent's input box went in with it (`merged_with_draft`, the Kanban Loop's reminder, DESIGN
    /// §13 P5-106).
    pub fn deliver(&self, name: &str, text: &str, cancel: &AtomicBool) -> Result<Taken, Refused> {
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
            return Ok(Taken {
                merged_with_draft: value.get("merged_with_draft") == Some(&Value::Bool(true)),
            });
        }
        let said = |key: &str| value.get(key).and_then(Value::as_str).map(str::to_owned);
        let code = said("error");
        let message = said("message")
            .or_else(|| code.clone())
            .unwrap_or_else(|| "corral send failed".into());
        Err(Refused { code, message })
    }

    /// Sets the fence profile the agents `name` starts from now on run in, or (`None`) clears it
    /// (DESIGN §13 P5-141): `corral child-sandbox`. What corral then holds for which run of
    /// `name`; or corral's refusal, word for word, the setting left as it was.
    pub fn set_child_sandbox(
        &self,
        name: &str,
        profile: Option<&str>,
        cancel: &AtomicBool,
    ) -> Result<ChildSandbox, Refused> {
        let unanswered = |message: String| Refused {
            code: None,
            message,
        };
        let args = child_sandbox_args(name, profile);
        let output =
            self.output(&args, CHILD_SANDBOX_TIMEOUT, cancel)
                .map_err(|error| match error.chain().nth(1) {
                    _ if self.remote.is_some() => unanswered(format!("{error}")),
                    Some(cause) => unanswered(format!("corral did not run: {cause}")),
                    None => unanswered("corral did not answer in time".into()),
                })?;
        let value: Value = serde_json::from_slice(&output.stdout)
            .map_err(|_| unanswered("corral did not say how it went".into()))?;
        let ok = value.get("ok");
        if output.status.success() && ok == Some(&Value::Bool(true)) {
            // What it holds and for which run must both be said: a field left out clears nothing.
            return serde_json::from_value(value).map_err(|error| {
                unanswered(format!("corral's answer could not be read: {error}"))
            });
        }
        if output.status.success() && ok != Some(&Value::Bool(false)) {
            return Err(unanswered(
                "corral's answer could not be read: it did not say ok".into(),
            ));
        }
        let said = |key: &str| value.get(key).and_then(Value::as_str).map(str::to_owned);
        let code = said("error");
        let message = said("message")
            .or_else(|| code.clone())
            .unwrap_or_else(|| "corral child-sandbox failed".into());
        Err(Refused { code, message })
    }
}

/// How long `corral child-sandbox` may take: setting a profile runs `fence check` first.
const CHILD_SANDBOX_TIMEOUT: Duration = Duration::from_secs(30);

/// `corral child-sandbox NAME <profile>`, or `corral child-sandbox NAME --clear`.
pub fn child_sandbox_args<'a>(name: &'a str, profile: Option<&'a str>) -> [&'a str; 3] {
    match profile {
        Some(profile) => ["child-sandbox", name, profile],
        None => ["child-sandbox", name, "--clear"],
    }
}

/// What `corral child-sandbox` answered: the run of the agent it holds the setting for, and the
/// setting, none when its agents start outside. Both must be there (`child_sandbox` may be
/// `null`); other fields are let be.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ChildSandbox {
    pub instance: String,
    #[serde(deserialize_with = "required")]
    pub child_sandbox: Option<Sandbox>,
}

/// A field that must be there, though it may be `null`: serde would take one left out as `None`.
fn required<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
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

/// What corral said of a `corral send` it took.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Taken {
    /// Text already typed in the agent's input box went in with it.
    pub merged_with_draft: bool,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{os::unix::fs::PermissionsExt, path::PathBuf};

    static NO: AtomicBool = AtomicBool::new(false);

    /// A fake `corral` in a directory of its own; never the real one.
    fn fake(name: &str, script: &str) -> (PathBuf, String) {
        let dir =
            std::env::temp_dir().join(format!("paddock-corral-test-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("corral");
        std::fs::write(&path, format!("#!/bin/sh\n{script}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        (dir, path.display().to_string())
    }

    #[test]
    fn child_sandbox_left_out_is_told_from_null() {
        let parse = |json: &str| serde_json::from_str::<Agent>(json).unwrap().child_sandbox;
        // An older corral says nothing of it.
        assert_eq!(parse(r#"{"name":"main"}"#), None);
        assert_eq!(parse(r#"{"name":"main","child_sandbox":null}"#), Some(None));
        assert_eq!(
            parse(r#"{"name":"main","child_sandbox":{"profile":"dev"}}"#),
            Some(Some(Sandbox {
                profile: "dev".into()
            }))
        );
        // And `corral status` taken in keeps what it said.
        let mut agent = Agent {
            name: "main".into(),
            ..Agent::default()
        };
        merge(
            &mut agent,
            Ok(serde_json::json!({"child_sandbox": {"profile": "dev"}})),
        );
        assert_eq!(
            agent.child_sandbox,
            Some(Some(Sandbox {
                profile: "dev".into()
            }))
        );
    }

    #[test]
    fn setting_and_clearing_ask_corral_child_sandbox() {
        assert_eq!(
            child_sandbox_args("p/main", Some("dev")),
            ["child-sandbox", "p/main", "dev"]
        );
        assert_eq!(
            child_sandbox_args("p/main", None),
            ["child-sandbox", "p/main", "--clear"]
        );
        let (dir, program) = fake(
            "child-sandbox",
            r#"echo "$@" >> "$(dirname "$0")/calls"
if [ "$3" = --clear ]; then child=null; else child="{\"profile\":\"$3\"}"; fi
printf '{"ok":true,"name":"%s","instance":"i-1","child_sandbox":%s}\n' "$2" "$child""#,
        );
        let corral = Client::local(program);
        assert_eq!(
            corral.set_child_sandbox("p/main", Some("dev"), &NO),
            Ok(ChildSandbox {
                instance: "i-1".into(),
                child_sandbox: Some(Sandbox {
                    profile: "dev".into()
                }),
            })
        );
        assert_eq!(
            corral.set_child_sandbox("p/main", None, &NO),
            Ok(ChildSandbox {
                instance: "i-1".into(),
                child_sandbox: None,
            })
        );
        let calls = std::fs::read_to_string(dir.join("calls")).unwrap();
        assert_eq!(
            calls,
            "child-sandbox p/main dev\nchild-sandbox p/main --clear\n"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn only_a_whole_answer_counts_as_saved() {
        let unreadable = |name: &str, answer: &str| {
            let (dir, program) = fake(name, &format!("echo '{answer}'"));
            let got = Client::local(program).set_child_sandbox("p/main", None, &NO);
            std::fs::remove_dir_all(&dir).unwrap();
            match got {
                Err(Refused {
                    code: None,
                    message,
                }) => {
                    assert!(
                        message.starts_with("corral's answer could not be read"),
                        "{answer}: {message}"
                    )
                }
                other => panic!("{answer}: {other:?}"),
            }
        };
        // Exit 0 but not saying it went well, or leaving out what it holds: nothing is cleared.
        unreadable("cs-empty", "{}");
        unreadable("cs-no-ok", r#"{"instance":"i-1","child_sandbox":null}"#);
        unreadable(
            "cs-ok-text",
            r#"{"ok":"yes","instance":"i-1","child_sandbox":null}"#,
        );
        unreadable(
            "cs-no-child",
            r#"{"ok":true,"name":"p/main","instance":"i-1"}"#,
        );
        unreadable(
            "cs-no-instance",
            r#"{"ok":true,"name":"p/main","child_sandbox":null}"#,
        );
        // `null` said is cleared; fields it does not know are let be.
        let (dir, program) = fake(
            "cs-whole",
            r#"echo '{"ok":true,"name":"p/main","instance":"i-1","child_sandbox":null,"new":1}'"#,
        );
        let told = Client::local(program)
            .set_child_sandbox("p/main", None, &NO)
            .unwrap();
        assert_eq!(told.child_sandbox, None);
        assert_eq!(told.instance, "i-1");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_refusal_is_corrals_own_words() {
        let (dir, program) = fake(
            "child-sandbox-refused",
            r#"echo '{"ok":false,"error":"sandbox_failed","message":"fence check failed: unknown_profile"}'; exit 1"#,
        );
        assert_eq!(
            Client::local(program).set_child_sandbox("p/main", Some("nope"), &NO),
            Err(Refused {
                code: Some("sandbox_failed".into()),
                message: "fence check failed: unknown_profile".into(),
            })
        );
        let (_, garbage) = fake("child-sandbox-garbage", "echo not json");
        assert_eq!(
            Client::local(garbage).set_child_sandbox("p/main", None, &NO),
            Err(Refused {
                code: None,
                message: "corral did not say how it went".into(),
            })
        );
        let missing = dir.join("no-such-corral").display().to_string();
        let refused = Client::local(missing)
            .set_child_sandbox("p/main", None, &NO)
            .unwrap_err();
        assert_eq!(refused.code, None);
        assert!(
            refused.message.starts_with("corral did not run"),
            "{refused:?}"
        );
        std::fs::remove_dir_all(&dir).unwrap();
        std::fs::remove_dir_all(dir.with_file_name(format!(
            "paddock-corral-test-{}-child-sandbox-garbage",
            std::process::id()
        )))
        .unwrap();
    }
}
