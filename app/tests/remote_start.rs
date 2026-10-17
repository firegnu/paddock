//! Starting an agent on another machine (DESIGN §13 P5-124), with no machine at all: a fake `ssh`
//! hands the command to this Mac's `sh`, as the shell sshd runs there would, under a home of its
//! own whose PATH has neither corral nor any agent; its login profile (`.profile`) puts an
//! nvm-like directory with some agents on the PATH, as on the user's 5090; a fake `corral` sits
//! in that home's `.local/bin`, where ranch puts it, and notes what it is asked.
mod common;
use paddock::{
    config::Machine,
    new_agent::{Started, Tool},
    remote::Link,
    remote_start::{self, Agent, Commands, Field, RecentDirs, Request},
};
use std::{
    path::Path,
    sync::{Arc, atomic::AtomicBool},
};

/// The fake `ssh`: skips the options as ssh would take them, notes each command, then runs it in
/// `sh` with the home there, a PATH that has no agent, and the fake login shell as `$SHELL`.
const SSH: &str = r#"#!/bin/sh
root=$(dirname "$0")
while [ $# -gt 0 ]; do
  case "$1" in
    -S|-o|-e|-O) shift 2 ;;
    -tt) shift ;;
    --) shift; break ;;
    *) break ;;
  esac
done
shift
printf '%s\0' "$*" >> "$root/calls"
HOME="$root/home" PATH=/usr/bin:/bin SHELL="$root/login-shell" exec /bin/sh -c "$*"
"#;

/// The user's shell there, as `$SHELL` names it: notes how it is asked; as a login shell (`-l`)
/// reads the profile, as an interactive one (`-i`) its rc, before the command, which `sh -c` then
/// runs. It stands in for the user's own zsh on the 5090, whose nvm is set up in `~/.zshrc`, which
/// zsh reads only when interactive (P5-124b); and it reads nothing of this Mac's.
const LOGIN_SHELL: &str = r#"#!/bin/sh
root=$(dirname "$0")
asked= login= interactive=
while [ $# -gt 0 ]; do
  case "$1" in
    -l) asked="$asked -l"; login=1; shift ;;
    -i) asked="$asked -i"; interactive=1; shift ;;
    -c) break ;;
    *) exit 64 ;;
  esac
done
printf '%s\n' "${asked# }" >> "$root/logins"
if [ -n "$login" ]; then . "$HOME/.profile"; fi
if [ -n "$interactive" ]; then . "$HOME/.shrc"; fi
[ "$1" = -c ] || exit 64
exec /bin/sh -c "$2"
"#;

/// The login profile there: a greeting.
const PROFILE: &str = "echo 'Welcome to studio'\nPROFILE_RAN=yes\nexport PROFILE_RAN\n";

/// The rc there, read by an interactive shell: the agents' directory on the PATH, as nvm's.
const SHRC: &str = "PATH=\"$HOME/nvm/bin:$PATH\"\nexport PATH\n";

/// The fake `corral` there: its version; `start` notes its arguments and whether the login
/// profile ran, refuses a directory that is not there as corral does, and starts the next free
/// name otherwise.
fn corral(root: &Path) -> String {
    format!(
        r#"#!/bin/sh
root='{root}'
case "$1" in
  --version) echo '{{"contract":"1","ok":true,"version":"0.9.0"}}' ;;
  start)
    shift
    printf '%s\0' "$@" > "$root/started"
    printf '%s' "${{PROFILE_RAN:-no}}" > "$root/profile"
    cwd=
    while [ $# -gt 0 ]; do
      case "$1" in --cwd) cwd=$2; shift 2 ;; --) break ;; *) shift ;; esac
    done
    if [ ! -d "$cwd" ]; then
      printf '{{"cwd":"%s","error":"bad_cwd","message":"no such directory","ok":false}}\n' "$cwd"
      exit 1
    fi
    echo '{{"instance":"i-9","name":"ranch/pi-2","ok":true}}'
    ;;
  *) echo '{{"ok":false,"error":"unexpected_command"}}'; exit 1 ;;
esac
"#,
        root = root.display()
    )
}

struct Fake {
    dir: common::TempDir,
    link: Arc<Link>,
}

impl Fake {
    fn root(&self) -> &Path {
        self.dir.path()
    }

    fn home(&self) -> String {
        self.root().join("home").display().to_string()
    }

    fn nul_separated(&self, file: &str) -> Vec<String> {
        std::fs::read_to_string(self.root().join(file))
            .unwrap_or_default()
            .split('\0')
            .filter(|part| !part.is_empty())
            .map(str::to_owned)
            .collect()
    }
}

/// `studio` behind the fake ssh, connected; pi and claude installed where only the rc of an
/// interactive login shell looks, as nvm's on the 5090.
fn studio() -> Fake {
    let dir = common::tempdir();
    let root = dir.path().to_path_buf();
    let ssh = common::script(&root, "ssh", SSH);
    common::script(&root, "login-shell", LOGIN_SHELL);
    let home = root.join("home");
    let bin = home.join(".local/bin");
    std::fs::create_dir_all(&bin).unwrap();
    common::script(&bin, "corral", &corral(&root));
    std::fs::write(home.join(".profile"), PROFILE).unwrap();
    std::fs::write(home.join(".shrc"), SHRC).unwrap();
    let agents = home.join("nvm/bin");
    std::fs::create_dir_all(&agents).unwrap();
    for kind in ["pi", "claude", "amp"] {
        common::script(&agents, kind, "#!/bin/sh\nexit 0\n");
    }
    let machine = Machine {
        name: "studio".into(),
        ssh: "me@studio".into(),
        corral: None,
        label: None,
        show: true,
    };
    let link = Arc::new(Link::with(&machine, ssh, Ok(root.join("socket"))));
    link.connect(&AtomicBool::new(false)).unwrap();
    Fake { dir, link }
}

#[test]
fn the_kinds_offered_are_those_the_login_shell_there_finds() {
    let studio = studio();
    assert_eq!(
        remote_start::look(&studio.link),
        Ok(vec![Tool::Claude, Tool::Pi])
    );
    // Through the login shell: a plain command there finds none of them.
    let plain = studio
        .link
        .run(
            "command -v pi claude || true",
            std::time::Duration::from_secs(5),
            &AtomicBool::new(false),
        )
        .unwrap();
    assert!(plain.stdout.is_empty());
    // The user's own shell there, as an interactive login shell.
    assert_eq!(
        std::fs::read_to_string(studio.root().join("logins")).unwrap(),
        "-l -i\n"
    );
}

#[test]
fn it_starts_through_the_login_shell_in_that_machines_home_with_the_words_typed() {
    let studio = studio();
    let odd = "my dir/it's $(touch pwned); `id` \"q\"";
    std::fs::create_dir_all(Path::new(&studio.home()).join(odd)).unwrap();
    let request = Request {
        agent: Agent::Kind(Tool::Codex),
        command: Tool::Codex.command().into(),
        dir: format!("~/{odd}"),
        name: "p/it's;$HOME".into(),
    };
    let started = remote_start::start(&studio.link, &request);
    assert_eq!(
        started,
        Ok(Started {
            name: "ranch/pi-2".into(),
            instance: Some("i-9".into()),
        })
    );
    assert_eq!(
        studio.nul_separated("started"),
        [
            "p/it's;$HOME",
            "--unique",
            "--cwd",
            &format!("{}/{odd}", studio.home()),
            "--label",
            "role=regular",
            "--",
            "codex",
            "--yolo",
        ]
    );
    assert_eq!(
        std::fs::read_to_string(studio.root().join("profile")).unwrap(),
        "yes"
    );
    assert!(!studio.root().join("pwned").exists());
    assert!(!Path::new(&studio.home()).join(odd).join("pwned").exists());
}

#[test]
fn an_empty_name_asks_for_the_directory_and_kind_and_corral_says_which() {
    let studio = studio();
    let request = Request {
        agent: Agent::Kind(Tool::Pi),
        command: Tool::Pi.command().into(),
        dir: "~".into(),
        name: String::new(),
    };
    let started = remote_start::start(&studio.link, &request).unwrap();
    assert_eq!(started.name, "ranch/pi-2");
    let asked = studio.nul_separated("started");
    assert_eq!(asked[..4], ["home/pi", "--unique", "--cwd", &studio.home()]);
    assert_eq!(asked[asked.len() - 1], "pi");
}

#[test]
fn a_directory_not_there_comes_back_as_corral_said_it() {
    let studio = studio();
    let request = Request {
        agent: Agent::Kind(Tool::Pi),
        command: Tool::Pi.command().into(),
        dir: "~/nope".into(),
        name: String::new(),
    };
    let failure = remote_start::start(&studio.link, &request).unwrap_err();
    assert_eq!(failure.field, Some(Field::Dir));
    assert_eq!(
        failure.text,
        format!(
            r#"{{"cwd":"{}/nope","error":"bad_cwd","message":"no such directory","ok":false}}"#,
            studio.home()
        )
    );
}

#[test]
fn a_command_of_the_users_own_reaches_corral_word_for_word_and_runs_nothing_there() {
    let studio = studio();
    let typed = r#"~/bin/my-agent "two words" 'it''s $HOME' ';' 'touch pwned1;' "$(touch pwned2)" '`touch pwned3`' --cfg=x"#;
    let request = Request {
        agent: Agent::Custom,
        command: typed.into(),
        dir: "~".into(),
        name: String::new(),
    };
    assert!(remote_start::start(&studio.link, &request).is_ok());
    let asked = studio.nul_separated("started");
    assert_eq!(asked[0], "home/my-agent");
    let at = asked.iter().position(|word| word == "--").unwrap();
    assert_eq!(
        asked[at + 1..],
        [
            &format!("{}/bin/my-agent", studio.home()),
            "two words",
            "its $HOME",
            ";",
            "touch pwned1;",
            "$(touch pwned2)",
            "`touch pwned3`",
            "--cfg=x",
        ]
    );
    for pwned in ["pwned1", "pwned2", "pwned3"] {
        assert!(!studio.root().join(pwned).exists(), "{pwned}");
        assert!(!Path::new(&studio.home()).join(pwned).exists(), "{pwned}");
    }
    // Through the user's own login shell there, as the kinds are.
    assert_eq!(
        std::fs::read_to_string(studio.root().join("profile")).unwrap(),
        "yes"
    );
}

#[test]
fn a_kinds_edited_command_is_the_one_started() {
    let studio = studio();
    let request = Request {
        agent: Agent::Kind(Tool::Pi),
        command: "pi --model 'son net'".into(),
        dir: "~".into(),
        name: String::new(),
    };
    assert!(remote_start::start(&studio.link, &request).is_ok());
    let asked = studio.nul_separated("started");
    assert_eq!(asked[0], "home/pi");
    assert_eq!(asked[asked.len() - 3..], ["pi", "--model", "son net"]);
}

#[test]
fn an_unclosed_quote_is_said_at_once_and_nothing_goes_there() {
    let studio = studio();
    let calls = studio.nul_separated("calls").len();
    for command in ["amp 'oops", ""] {
        let request = Request {
            agent: Agent::Custom,
            command: command.into(),
            dir: "~".into(),
            name: String::new(),
        };
        let failure = remote_start::start(&studio.link, &request).unwrap_err();
        assert_eq!(failure.field, Some(Field::Command));
    }
    assert_eq!(studio.nul_separated("calls").len(), calls);
    assert!(!studio.root().join("started").exists());
}

#[test]
fn only_what_corral_started_is_kept_for_that_machine() {
    let studio = studio();
    let (mut recent, mut commands) = (RecentDirs::default(), Commands::default());
    let mut request = Request {
        agent: Agent::Custom,
        command: "~/bin/my-agent --fast".into(),
        dir: "~/nope".into(),
        name: String::new(),
    };
    let failed = remote_start::start(&studio.link, &request);
    assert!(failed.is_err());
    assert!(!remote_start::keep(
        &failed,
        "studio",
        &request,
        &[],
        &mut recent,
        &mut commands
    ));
    assert_eq!(commands, Commands::default());
    assert_eq!(recent, RecentDirs::default());
    request.dir = "~".into();
    let started = remote_start::start(&studio.link, &request);
    assert!(started.is_ok());
    assert!(remote_start::keep(
        &started,
        "studio",
        &request,
        &[],
        &mut recent,
        &mut commands
    ));
    let customs = commands.customs("studio");
    assert_eq!(customs.len(), 1);
    assert_eq!(customs[0].name, "my-agent");
    assert_eq!(customs[0].command, "~/bin/my-agent --fast");
    assert_eq!(recent.of("studio"), ["~"]);
    assert!(commands.customs("5090").is_empty());
}

#[test]
fn a_custom_command_named_as_a_kind_found_there_is_kept_as_that_kinds() {
    let studio = studio();
    // claude and pi are found on studio; omp is not.
    let found = remote_start::look(&studio.link).unwrap();
    let (mut recent, mut commands) = (RecentDirs::default(), Commands::default());
    let start = |command: &str, recent: &mut RecentDirs, commands: &mut Commands| {
        let request = Request {
            agent: Agent::Custom,
            command: command.into(),
            dir: "~".into(),
            name: String::new(),
        };
        let outcome = remote_start::start(&studio.link, &request);
        assert!(outcome.is_ok(), "{command}");
        remote_start::keep(&outcome, "studio", &request, &found, recent, commands);
    };
    start("pi --model sonnet", &mut recent, &mut commands);
    assert_eq!(commands.command("studio", Tool::Pi), "pi --model sonnet");
    assert!(commands.customs("studio").is_empty());
    // Not found there this time: a capsule of its own, as before.
    start("~/bin/omp --x", &mut recent, &mut commands);
    assert_eq!(commands.command("studio", Tool::Omp), "omp");
    let names: Vec<String> = commands
        .shown("studio", &found)
        .into_iter()
        .map(|c| c.name)
        .collect();
    assert_eq!(names, ["omp"]);
    // pi's own again: its edit forgotten, still no capsule.
    start("pi", &mut recent, &mut commands);
    assert_eq!(commands.command("studio", Tool::Pi), "pi");
    assert_eq!(commands.customs("studio").len(), 1);
}
