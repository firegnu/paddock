//! Another machine's corral (DESIGN §13 P5-90), with no machine at all: a fake `ssh` takes the
//! arguments paddock gives it and hands the command to this Mac's `sh`, as the login shell there
//! would, under a home of its own whose PATH has no corral; a fake `corral` sits in that home's
//! `.local/bin`, where ranch puts it.
mod common;
use paddock::{
    config::Machine,
    corral::{Client, Poller},
    remote::{self, Link, Problem, Rounds},
    viewer::{AgentMetadata, Viewer},
};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicBool},
    thread,
    time::{Duration, Instant},
};

/// A name and a reply only a shell that keeps quoted words as written passes on unchanged.
const ODD: &str = "it's $(touch pwned) `id` \"q\" ~ *\n第二行 ;|&";

/// The fake `ssh`: skips the options as ssh would take them, notes the target and each command
/// (apart by a NUL), then fails, hangs or runs the command as the files beside it say.
const SSH: &str = r#"#!/bin/sh
root=$(dirname "$0")
while [ $# -gt 0 ]; do
  case "$1" in
    -S|-o|-e) shift 2 ;;
    -tt) shift ;;
    --) shift; break ;;
    *) break ;;
  esac
done
printf '%s\n' "$1" >> "$root/targets"
shift
printf '%s\0' "$*" >> "$root/calls"
if [ -e "$root/hang" ]; then exec sleep 30; fi
if [ -e "$root/refuse" ]; then cat "$root/refuse" >&2; exit 255; fi
HOME="$root/home" PATH=/usr/bin:/bin exec /bin/sh -c "$*"
"#;

/// The fake `corral` there: its contract from the file beside the test's root, two agents (one
/// named with every quoting hazard), each name it is asked about and each reply it is sent noted
/// as given.
fn corral(root: &Path) -> String {
    format!(
        r#"#!/bin/sh
root='{root}'
case "$1" in
  --version) printf '{{"version":"0.9.0","contract":"%s"}}\n' "$(cat "$root/contract")" ;;
  ls) cat "$root/listing" ;;
  status)
    printf '%s\0' "$2" >> "$root/asked"
    attached=0
    if [ -e "$root/attached" ]; then attached=1; fi
    printf '{{\n  "ok": true,\n  "state": "idle",\n  "attached": %s,\n  "instance": "i-a"\n}}\n' "$attached"
    ;;
  send) printf '%s\0%s\0' "$2" "$3" >> "$root/sent"; echo '{{"ok":true}}' ;;
  attach)
    trap 'exit 0' INT
    printf READY
    while :; do sleep 1 & wait $!; done
    ;;
  *) echo '{{"ok":false,"error":"unexpected_command"}}'; exit 1 ;;
esac
"#,
        root = root.display()
    )
}

/// The agents the fake lists, as JSON.
fn listing() -> String {
    serde_json::json!({"agents": [
        {"name": "p/a", "cwd": "/srv/a", "instance": "i-a"},
        {"name": ODD, "cwd": "/srv/b"},
    ]})
    .to_string()
}

struct Fake {
    dir: common::TempDir,
    link: Arc<Link>,
}

impl Fake {
    fn root(&self) -> &Path {
        self.dir.path()
    }

    /// The commands ssh was given, in order.
    fn calls(&self) -> Vec<String> {
        nul_separated(&self.root().join("calls"))
    }
}

fn nul_separated(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .split('\0')
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect()
}

/// A machine called `studio` behind the fake ssh, corral there speaking `contract`.
fn machine(contract: &str, configured: Option<&str>) -> Fake {
    let dir = common::tempdir();
    let root = dir.path().to_path_buf();
    let ssh = common::script(&root, "ssh", SSH);
    let bin = root.join("home/.local/bin");
    std::fs::create_dir_all(&bin).unwrap();
    common::script(&bin, "corral", &corral(&root));
    std::fs::write(root.join("contract"), contract).unwrap();
    std::fs::write(root.join("listing"), listing()).unwrap();
    let machine = Machine {
        name: "studio".into(),
        ssh: "me@studio".into(),
        corral: configured.map(str::to_owned),
    };
    let link = Link::with(&machine, ssh, Ok(root.join("socket")));
    Fake {
        dir,
        link: Arc::new(link),
    }
}

fn never() -> AtomicBool {
    AtomicBool::new(false)
}

#[test]
fn ssh_keeps_to_paddocks_own_connection_and_sets_the_target_apart() {
    let studio = machine("1", None);
    let args = studio.link.args("true").unwrap();
    let socket = studio.root().join("socket").display().to_string();
    assert_eq!(&args[..2], ["-S", socket.as_str()]);
    for option in [
        "ControlMaster=auto",
        "BatchMode=yes",
        "ServerAliveInterval=5",
        "ServerAliveCountMax=3",
    ] {
        assert!(args.iter().any(|arg| arg == option), "{option}: {args:?}");
    }
    assert!(args.iter().any(|arg| arg.starts_with("ControlPersist=")));
    assert!(args.iter().any(|arg| arg.starts_with("ConnectTimeout=")));
    // Every option before `--`, then the target and the command.
    assert_eq!(&args[args.len() - 3..], ["--", "me@studio", "true"]);
    // No attach before corral is found there.
    assert!(studio.link.attach_argv("p/a").is_err());
}

#[test]
fn control_sockets_live_in_a_private_directory_short_enough_for_macos() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let temp = common::tempdir();
    let uid = unsafe { libc::getuid() };
    let dir = remote::control_dir(temp.path(), uid).unwrap();
    assert_eq!(std::fs::metadata(&dir).unwrap().mode() & 0o777, 0o700);
    // Opened up meanwhile: closed again.
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    let again = remote::control_dir(temp.path(), uid).unwrap();
    assert_eq!(std::fs::metadata(&again).unwrap().mode() & 0o777, 0o700);
    // Not another user's, and not a link to somewhere else.
    assert!(remote::control_dir(temp.path(), uid + 1).is_err());
    let other = common::tempdir();
    let linked = other.path().join(format!("paddock-{uid}"));
    std::os::unix::fs::symlink(temp.path(), &linked).unwrap();
    assert!(remote::control_dir(other.path(), uid).is_err());

    // Where paddock puts them, in `/tmp`, where a path stays short.
    let short = PathBuf::from(format!("/tmp/paddock-{uid}"));
    let a = remote::socket_path(&short, "studio", "me@studio").unwrap();
    let b = remote::socket_path(&short, "studio", "me@elsewhere").unwrap();
    assert_ne!(a, b);
    assert!(a.as_os_str().len() + 17 < 104);
    let deep = PathBuf::from(format!("/tmp/{}", "d".repeat(90)));
    assert!(remote::socket_path(&deep, "studio", "me@studio").is_err());
}

#[test]
fn quoted_words_reach_the_shell_as_written() {
    for word in [ODD, "", "'", "a b", "$HOME", "{a,b}", "!x", "^y", "中文"] {
        let output = std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg(format!("printf '%s' {}", remote::quote(word)))
            .output()
            .unwrap();
        assert_eq!(String::from_utf8(output.stdout).unwrap(), word);
    }
}

#[test]
fn corral_is_found_where_ranch_puts_it_and_its_home_noted() {
    let studio = machine("1", None);
    studio.link.connect(&never()).unwrap();
    let snapshot = studio.link.snapshot();
    let home = studio.root().join("home").display().to_string();
    assert_eq!(
        snapshot.corral.as_deref(),
        Some(format!("{home}/.local/bin/corral").as_str())
    );
    assert_eq!(snapshot.home.as_deref(), Some(home.as_str()));
    assert_eq!(snapshot.version, Some(("0.9.0".into(), "1".into())));
    // Configured with `~`: there, its home.
    let configured = machine("1", Some("~/.local/bin/corral"));
    configured.link.connect(&never()).unwrap();
    let home = configured.root().join("home").display().to_string();
    assert_eq!(
        configured.link.snapshot().corral,
        Some(format!("{home}/.local/bin/corral"))
    );
    // Configured where there is none.
    let missing = machine("1", Some("/nowhere/corral"));
    assert!(matches!(
        missing.link.connect(&never()),
        Err(Problem::NoCorral(_))
    ));
}

#[test]
fn another_contract_is_marked_and_its_agents_are_not_read() {
    let studio = machine("2", None);
    let mut rounds = Rounds::new(studio.link.clone());
    let read = rounds.round(true, &never()).unwrap();
    assert_eq!(read.unwrap_err().to_string(), "corral contract 2, needs 1");
    assert!(
        !studio.calls().iter().any(|call| call.contains("'ls'")),
        "{:?}",
        studio.calls()
    );
    match studio.link.snapshot().status {
        remote::Status::Down { problem, .. } => {
            assert_eq!(problem, Problem::Contract("2".into()))
        }
        status => panic!("{status:?}"),
    }
}

#[test]
fn a_round_asks_twice_whatever_the_agents_and_names_go_as_written() {
    let studio = machine("1", None);
    studio.link.connect(&never()).unwrap();
    let before = studio.calls().len();
    let agents = remote::collect(&studio.link, &never()).ok().unwrap();
    assert_eq!(studio.calls().len() - before, 2, "{:?}", studio.calls());
    let names: Vec<&str> = agents.iter().map(|a| a.name.as_str()).collect();
    let mut expected = vec!["p/a", ODD];
    expected.sort();
    assert_eq!(names, expected);
    // Every status was read from its own part of the one answer.
    assert!(agents.iter().all(|a| a.state.as_deref() == Some("idle")));
    assert!(agents.iter().all(|a| a.error.is_none()), "{agents:?}");
    // The names corral was asked about are the names it listed, byte for byte.
    let mut asked = nul_separated(&studio.root().join("asked"));
    asked.sort();
    assert_eq!(asked, expected);
}

#[test]
fn a_reply_from_the_inbox_reaches_corral_there_as_typed() {
    let studio = machine("1", None);
    studio.link.connect(&never()).unwrap();
    let client = Client {
        program: "corral".into(),
        remote: Some(studio.link.clone()),
    };
    client.send(ODD, ODD, &never()).unwrap();
    assert_eq!(nul_separated(&studio.root().join("sent")), [ODD, ODD]);
}

#[test]
fn ssh_failures_say_what_the_user_can_do() {
    let studio = machine("1", None);
    std::fs::write(
        studio.root().join("refuse"),
        "me@studio: Permission denied (publickey).\n",
    )
    .unwrap();
    assert!(matches!(
        studio.link.connect(&never()),
        Err(Problem::SignIn(_))
    ));
    std::fs::write(
        studio.root().join("refuse"),
        "Host key verification failed.\n",
    )
    .unwrap();
    assert!(matches!(
        studio.link.connect(&never()),
        Err(Problem::HostKey(_))
    ));
    std::fs::write(
        studio.root().join("refuse"),
        "ssh: connect to host studio port 22: Connection refused\n",
    )
    .unwrap();
    let problem = studio.link.connect(&never()).unwrap_err();
    assert!(problem.transient());
    assert!(problem.advice("me@studio").contains("try now"));
    assert!(
        Problem::SignIn("x".into())
            .advice("me@studio")
            .contains("Run ssh me@studio once in Terminal")
    );
}

/// Polls `poller` until `count` updates came or `deadline`, returning how many came.
fn updates(poller: &Poller, count: usize, deadline: Instant) -> usize {
    let mut got = 0;
    while got < count && Instant::now() < deadline {
        got += poller.updates.try_iter().count();
        thread::sleep(Duration::from_millis(10));
    }
    got
}

#[test]
fn a_machine_that_hangs_or_fails_never_holds_this_macs_agents_back() {
    let temp = common::tempdir();
    let local = common::script(
        temp.path(),
        "corral",
        "#!/bin/sh\ncase \"$1\" in\n  ls) echo '{\"agents\":[]}' ;;\n  *) exit 1 ;;\nesac\n",
    );
    for trouble in ["hang", "refuse"] {
        let studio = machine("1", None);
        std::fs::write(studio.root().join(trouble), "Connection timed out\n").unwrap();
        let far = Poller::remote(studio.link.clone());
        let here = Poller::start(Client::local(local.clone()), Duration::from_millis(100));
        let started = Instant::now();
        // This Mac's list comes every tenth of a second all the same.
        assert_eq!(
            updates(&here, 5, started + Duration::from_secs(3)),
            5,
            "{trouble}"
        );
        assert!(started.elapsed() < Duration::from_secs(2), "{trouble}");
        drop(here);
        let stopping = Instant::now();
        drop(far);
        assert!(stopping.elapsed() < Duration::from_secs(5), "{trouble}");
    }
}

/// Ticks `viewer` until `done` or three seconds.
fn tick_until(viewer: &mut Viewer, mut done: impl FnMut(&Viewer) -> bool) {
    let size = paddock::terminal::Size { rows: 10, cols: 40 };
    let deadline = Instant::now() + Duration::from_secs(3);
    while !done(viewer) {
        assert!(Instant::now() < deadline, "{}", viewer.note);
        viewer.tick(size).unwrap();
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn attached_elsewhere_watches_there_and_is_refused_here() {
    let metadata = AgentMetadata {
        cwd: None,
        instance: Some("i-a".into()),
    };
    // Another machine: attached through ssh, watching only.
    let studio = machine("1", None);
    studio.link.connect(&never()).unwrap();
    std::fs::write(studio.root().join("attached"), "").unwrap();
    let mut viewer = Viewer::with(Client {
        program: "corral".into(),
        remote: Some(studio.link.clone()),
    });
    viewer.select_agent("p/a".into(), metadata.clone()).unwrap();
    tick_until(&mut viewer, |v| v.showing.as_deref() == Some("p/a"));
    assert!(viewer.read_only);
    assert!(
        studio
            .calls()
            .iter()
            .any(|call| call.ends_with("'attach' 'p/a'")),
        "{:?}",
        studio.calls()
    );
    // The other window went, as a read begun after the attach says: it may type.
    let alone = vec![paddock::corral::Agent {
        name: "p/a".into(),
        attached: 1,
        instance: Some("i-a".into()),
        ..Default::default()
    }];
    viewer
        .listed(&alone, Some(std::time::Instant::now()))
        .unwrap();
    assert!(!viewer.read_only);
    viewer.close().unwrap();

    // This Mac: refused, as before.
    let temp = common::tempdir();
    let local = common::script(temp.path(), "corral", &corral(studio.root()));
    let mut viewer = Viewer::new(local);
    viewer.select_agent("p/a".into(), metadata).unwrap();
    tick_until(&mut viewer, |v| v.note.contains("attached elsewhere"));
    assert!(viewer.showing.is_none());
    assert!(!viewer.read_only);
}
