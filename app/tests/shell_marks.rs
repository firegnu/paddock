//! Command marks from real zsh and bash (P5-149), under a made-up home: each command marked once,
//! the prompt drawn as without them, the user's files untouched.
mod common;
use common::{TempDir, tempdir};
use paddock::terminal::{Screen, Size};
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

const PROMPT: &str = "FIXTURE> ";
const ROWS: u16 = 30;
const COLS: u16 = 100;

/// The shells there are here to try: macOS's own bash is 3.2 (no `PS0`), Homebrew's is new.
fn shells() -> Vec<&'static str> {
    ["/bin/zsh", "/bin/bash", "/opt/homebrew/bin/bash"]
        .into_iter()
        .filter(|shell| Path::new(shell).exists())
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    /// No marks: the shell as it was before.
    Bare,
    Plain,
    Agent,
}

struct Home {
    home: TempDir,
    /// Where paddock writes its startup files, away from the user's.
    private: TempDir,
}

impl Home {
    /// A home whose startup files set a known prompt and hooks of the user's own; `other` adds
    /// an integration that already marks the prompt itself, as iTerm2's does.
    fn new(other: bool) -> Self {
        let home = tempdir();
        fs::set_permissions(home.path(), fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(home.path().join(".zshenv"), "export FROM_ZSHENV=1\n").unwrap();
        let mut zshrc = format!(
            "PS1='{PROMPT}'\nmine() {{ : }}\nprecmd_functions+=(mine)\npreexec_functions+=(mine)\n"
        );
        let mut bashrc = format!("PS1='{PROMPT}'\nmine() {{ :; }}\nPROMPT_COMMAND='mine'\n");
        if other {
            zshrc.push_str(
                r#"other_precmd() {
    local s=$?
    printf '\e]133;D;%s\a' $s
    other_saved=$PS1
    PS1=$'%{\e]133;A\a%}'$PS1$'%{\e]133;B\a%}'
}
other_preexec() { PS1=$other_saved; printf '\e]133;C\a'; }
precmd_functions+=(other_precmd)
preexec_functions+=(other_preexec)
"#,
            );
            bashrc.push_str(
                // Like bash-preexec (which iTerm2's uses): the trap leaves alone what
                // PROMPT_COMMAND runs.
                r#"other_status() { local s=$?; printf '\e]133;D;%s\a' $s; return $s; }
other_ready() { other_armed=1; }
other_debug() {
    [[ $other_armed == 1 ]] || return
    case $'\n'"$PROMPT_COMMAND"$'\n' in *$'\n'"$BASH_COMMAND"$'\n'*) return;; esac
    other_armed=0
    printf '\e]133;C\a'
}
PS1='\[\e]133;A\a\]'$PS1'\[\e]133;B\a\]'
PROMPT_COMMAND=other_status$'\n'"$PROMPT_COMMAND"$'\n'other_ready
trap 'other_debug' DEBUG
"#,
            );
        }
        fs::write(home.path().join(".zshrc"), zshrc).unwrap();
        fs::write(home.path().join(".bashrc"), bashrc).unwrap();
        Self {
            home,
            private: tempdir(),
        }
    }

    /// The user's startup files.
    fn files(&self) -> BTreeMap<PathBuf, Vec<u8>> {
        [".zshenv", ".zshrc", ".bashrc"]
            .into_iter()
            .map(|name| self.home.path().join(name))
            .map(|path| {
                let contents = fs::read(&path).unwrap();
                (path, contents)
            })
            .collect()
    }

    fn command(&self, shell: &str, kind: Kind) -> (Vec<String>, Vec<(String, String)>) {
        let dir = self.private.path().join("integration");
        let setup = match kind {
            Kind::Bare => return (vec![shell.into(), "-i".into()], Vec::new()),
            Kind::Plain => paddock::shell_marks::setup(shell, &dir, None).unwrap(),
            Kind::Agent => paddock::agent_shell::setup(
                shell,
                &dir,
                Path::new(env!("CARGO_BIN_EXE_paddock")),
                None,
            )
            .unwrap(),
        };
        (setup.command, setup.env)
    }
}

struct Shell {
    writer: Box<dyn Write + Send>,
    output: Arc<Mutex<Vec<u8>>>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    _master: Box<dyn MasterPty + Send>,
}

impl Shell {
    fn start(home: &Home, shell: &str, kind: Kind) -> Self {
        let (command, env) = home.command(shell, kind);
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: ROWS,
                cols: COLS,
                pixel_width: 0,
                pixel_height: 0,
            })
            .unwrap();
        let mut builder = CommandBuilder::new(&command[0]);
        builder.args(&command[1..]);
        builder.env_clear();
        builder.env("HOME", home.home.path());
        builder.env("PATH", "/usr/bin:/bin");
        builder.env("TERM", "xterm-256color");
        for (key, value) in env {
            builder.env(key, value);
        }
        builder.cwd(home.home.path());
        let child = pair.slave.spawn_command(builder).unwrap();
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader().unwrap();
        let output: Arc<Mutex<Vec<u8>>> = Arc::default();
        let sink = output.clone();
        thread::spawn(move || {
            let mut buffer = [0; 4096];
            while let Ok(n) = reader.read(&mut buffer) {
                if n == 0 {
                    break;
                }
                sink.lock().unwrap().extend_from_slice(&buffer[..n]);
            }
        });
        Self {
            writer: pair.master.take_writer().unwrap(),
            output,
            child,
            _master: pair.master,
        }
    }

    fn prompts(&self) -> usize {
        let output = self.output.lock().unwrap();
        output
            .windows(PROMPT.len())
            .filter(|window| *window == PROMPT.as_bytes())
            .count()
    }

    /// Types `line` once the shell has drawn its `count`th prompt.
    fn type_at(&mut self, count: usize, line: &str) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while self.prompts() < count {
            assert!(
                Instant::now() < deadline,
                "no prompt {count}: {:?}",
                self.text()
            );
            thread::sleep(Duration::from_millis(10));
        }
        // Lets the shell settle at its prompt (zle, readline) before the line arrives.
        thread::sleep(Duration::from_millis(50));
        self.writer.write_all(line.as_bytes()).unwrap();
        self.writer.flush().unwrap();
    }

    fn text(&self) -> String {
        String::from_utf8_lossy(&self.output.lock().unwrap()).into_owned()
    }

    /// Runs `lines` one prompt at a time and returns everything the shell printed up to the
    /// prompt after the last.
    fn session(mut self, lines: &[&str]) -> Vec<u8> {
        for (index, line) in lines.iter().enumerate() {
            self.type_at(index + 1, &format!("{line}\n"));
        }
        self.type_at(lines.len() + 1, "");
        let output = self.output.lock().unwrap().clone();
        let _ = self.child.kill();
        let _ = self.child.wait();
        output
    }
}

/// The OSC 133 marks in `bytes`, in order (`A`, `B`, `C`, `D;0`).
fn marks(bytes: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(bytes);
    text.split("\x1b]133;")
        .skip(1)
        .map(|rest| {
            let mark = rest.split('\x07').next().unwrap();
            mark.strip_suffix(";paddock=1").unwrap_or(mark).to_owned()
        })
        .collect()
}

/// What the screen shows after `bytes`.
fn screen_text(bytes: &[u8]) -> String {
    let mut screen = Screen::new(Size {
        rows: ROWS,
        cols: COLS,
    });
    screen.process(bytes);
    let grid = screen.term.grid();
    use alacritty_terminal::grid::Dimensions;
    use alacritty_terminal::index::{Column, Line};
    (grid.topmost_line().0..=grid.bottommost_line().0)
        .map(|line| {
            (0..grid.columns())
                .map(|col| grid[Line(line)][Column(col)].c)
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The commands a shell's terminal keeps from `bytes`: each prompt line and exit status.
fn segments(bytes: &[u8]) -> Vec<(String, Option<i32>)> {
    let mut screen = Screen::new(Size {
        rows: ROWS,
        cols: COLS,
    });
    screen.keep_segments();
    screen.process(bytes);
    let grid = screen.term.grid();
    use alacritty_terminal::grid::Dimensions;
    use alacritty_terminal::index::Column;
    screen
        .segments()
        .iter()
        .map(|segment| {
            let line = (0..grid.columns())
                .map(|col| grid[segment.prompt][Column(col)].c)
                .collect::<String>()
                .trim_end()
                .to_owned();
            (line, segment.exit)
        })
        .collect()
}

const LINES: &[&str] = &["true", "false", "", "printf 'a\\nb\\n'", "sh -c 'exit 3'"];

fn expected_segments() -> Vec<(String, Option<i32>)> {
    [
        ("true", 0),
        ("false", 1),
        ("printf 'a\\nb\\n'", 0),
        ("sh -c 'exit 3'", 3),
    ]
    .into_iter()
    .map(|(line, code)| (format!("{PROMPT}{line}"), Some(code)))
    .collect()
}

fn expected_marks(shell: &str) -> Vec<String> {
    let mut expected = Vec::new();
    if shell.ends_with("bash") {
        expected.push("D;0".into());
    }
    expected.extend(["A".into(), "B".into()]);
    for code in [Some(0), Some(1), None, Some(0), Some(3)] {
        if let Some(code) = code {
            expected.extend(["C".into(), format!("D;{code}")]);
        } else if shell.ends_with("bash") {
            // bash reports status again after an empty command line.
            expected.push("D;1".into());
        }
        expected.extend(["A".into(), "B".into()]);
    }
    expected
}

fn check_input_positions(bytes: &[u8], what: &str) {
    let mut screen = Screen::new(Size {
        rows: ROWS,
        cols: COLS,
    });
    let text = String::from_utf8_lossy(bytes);
    let mut from = 0;
    for (to, _) in text.match_indices("\x1b]133;B") {
        screen.process(&bytes[from..to]);
        let grid = screen.term.grid();
        let point = grid.cursor.point;
        assert_eq!(
            point.column.0,
            PROMPT.len(),
            "{what}: B is at the input column"
        );
        use alacritty_terminal::index::Column;
        let prompt: String = (0..PROMPT.len())
            .map(|col| grid[point.line][Column(col)].c)
            .collect();
        assert_eq!(prompt, PROMPT, "{what}: B follows the theme's prompt");
        from = to;
    }
}

fn check_theme(shell: &str, rc: &str, theme: &str) {
    let home = Home::new(false);
    let path = home.home.path().join(rc);
    let mut config = fs::read_to_string(&path).unwrap();
    config.push_str(theme);
    fs::write(&path, config).unwrap();
    let files = home.files();
    let bare = Shell::start(&home, shell, Kind::Bare).session(LINES);
    let theirs = marks(&bare);
    let (expected, expected_segments) = if theirs.is_empty() {
        (expected_marks(shell), expected_segments())
    } else {
        // A theme may supply prompt marks itself (as p10k can), without a complete integration.
        // Preserve precisely what it sends; paddock must not add a second set.
        (theirs, segments(&bare))
    };
    for kind in [Kind::Plain, Kind::Agent] {
        let output = Shell::start(&home, shell, kind).session(LINES);
        let what = format!("{shell} {kind:?}");
        assert_eq!(marks(&output), expected, "{what}: each mark exactly once");
        check_input_positions(&output, &what);
        assert_eq!(segments(&output), expected_segments, "{what}");
        assert_eq!(screen_text(&output), screen_text(&bare), "{what}");
        assert_eq!(home.files(), files, "{what}: user's files untouched");
    }
}

#[test]
fn zsh_theme_moving_its_precmd_last_keeps_command_marks() {
    let theme = r#"
theme_precmd() {
    precmd_functions=(${precmd_functions:#theme_precmd} theme_precmd)
    PS1='FIXTURE> '
    if [[ ${theme_integration-} == 1 ]]; then
        PS1=$'%{\e]133;A\a%}'$PS1$'%{\e]133;B\a%}'
    fi
}
precmd_functions+=(theme_precmd)
"#;
    check_theme("/bin/zsh", ".zshrc", theme);
    check_theme(
        "/bin/zsh",
        ".zshrc",
        &format!("{theme}\ntheme_integration=1\n"),
    );
}

#[test]
fn zsh_theme_resetting_the_prompt_in_line_init_keeps_command_marks() {
    check_theme(
        "/bin/zsh",
        ".zshrc",
        r#"
PS1='BEFORE> '
theme_line_init() {
    PS1='FIXTURE> '
    zle reset-prompt
}
zle -N zle-line-init theme_line_init
"#,
    );
    let home = Home::new(false);
    for kind in [Kind::Plain, Kind::Agent] {
        let output = Shell::start(&home, "/bin/zsh", kind).session(&["if true; then\ntrue\nfi"]);
        assert_eq!(
            marks(&output),
            ["A", "B", "C", "D;0", "A", "B"],
            "{kind:?}: continuation line-init is not another command prompt"
        );
    }
}

#[test]
fn bash_theme_resetting_ps1_after_paddock_keeps_command_marks() {
    for shell in shells().into_iter().filter(|shell| shell.ends_with("bash")) {
        check_theme(
            shell,
            ".bashrc",
            r#"
theme_prompt() { PS1='FIXTURE> '; }
mine() {
    PROMPT_COMMAND=${PROMPT_COMMAND//$'\n''theme_prompt'/}
    PROMPT_COMMAND+=$'\n''theme_prompt'
}
"#,
        );
        if shell == "/opt/homebrew/bin/bash" {
            check_theme(
                shell,
                ".bashrc",
                r#"
PROMPT_COMMAND=(mine)
theme_prompt() { PS1='FIXTURE> '; }
mine() {
    local entry
    local -a kept=()
    for entry in "${PROMPT_COMMAND[@]}"; do
        [[ $entry == theme_prompt ]] || kept+=("$entry")
    done
    PROMPT_COMMAND=("${kept[@]}" theme_prompt)
}
"#,
            );
        }
    }
}

#[test]
fn zsh_and_bash_mark_each_command_once_without_changing_the_prompt_or_the_users_files() {
    for shell in shells() {
        let home = Home::new(false);
        let files = home.files();
        let bare = Shell::start(&home, shell, Kind::Bare).session(LINES);
        assert_eq!(
            marks(&bare),
            Vec::<String>::new(),
            "{shell} sends none itself"
        );
        for kind in [Kind::Plain, Kind::Agent] {
            let output = Shell::start(&home, shell, kind).session(LINES);
            let what = format!("{shell} {kind:?}");
            assert_eq!(marks(&output), expected_marks(shell), "{what}");
            assert_eq!(segments(&output), expected_segments(), "{what}");
            assert_eq!(
                screen_text(&output),
                screen_text(&bare),
                "{what}: the screen reads the same"
            );
            assert_eq!(
                home.files(),
                files,
                "{what}: the user's files are untouched"
            );
            assert!(
                !home
                    .private
                    .path()
                    .join("integration/.zsh_history")
                    .exists(),
                "{what}: history is kept where the user keeps it"
            );
        }
    }
}

#[test]
fn another_integration_that_marks_the_prompt_is_not_marked_again() {
    for shell in shells() {
        let home = Home::new(true);
        let theirs = Shell::start(&home, shell, Kind::Bare).session(LINES);
        for kind in [Kind::Plain, Kind::Agent] {
            let output = Shell::start(&home, shell, kind).session(LINES);
            let what = format!("{shell} {kind:?}");
            let (ours, alone) = (marks(&output), marks(&theirs));
            assert_eq!(
                ours, alone,
                "{what}: even the first prompt is marked only by theirs"
            );
            assert_eq!(segments(&output), expected_segments(), "{what}");
        }
    }
}
