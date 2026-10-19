//! The terminal pane: a Saddle PTY session drawn and driven by GPUI.
//!
//! Saddle's `Viewer` owns the shell / `corral attach` lifecycle and `Session` the PTY and parser,
//! unchanged. This file only reads the parser's grid, draws it, and turns window input into the
//! bytes Saddle's `input` module already encodes.
mod caret;
mod edge;
mod file_drop;
mod glide;
pub mod links;
mod notice;
mod press;

use crate::{
    damage,
    find::{self, Find},
    glyphs,
    grid::{self, Metrics, Scroll},
    ime::Composition,
    keys, menu,
    palette::{Rgb, Theme},
    rows::{self, Run, Span, Style},
    text_input::{self, Changed, TextInput},
    theme,
    to_tag::{self, Typing},
};
use crate::{
    pty::{Bell, Session},
    terminal::Size,
    viewer::{AgentMetadata, Shell, Viewer},
};
use alacritty_terminal::{
    grid::{Dimensions, Scroll as ViewScroll},
    index::{Column, Line, Point as GridPoint, Side},
    selection::{Selection, SelectionType},
    term::{TermMode, cell::Flags},
    vte::ansi::CursorShape,
};
use gpui::{
    App, Bounds, ClipboardItem, ContentMask, Context, CursorStyle, DispatchPhase,
    ElementInputHandler, Entity, EntityInputHandler, EventEmitter, ExternalPaths, FocusHandle,
    Focusable, Font, FontStyle, FontWeight, Hsla, KeyDownEvent, ModifiersChangedEvent, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, PathBuilder, Pixels, Point, Render, Rgba,
    ScrollDelta, ScrollWheelEvent, SharedString, Subscription, TextAlign, TextRun, UTF16Selection,
    UnderlineStyle, Window, canvas, div, fill, point, prelude::*, px, size,
};
use links::Link;
use std::{
    cell::RefCell,
    ops::Range,
    path::{Path, PathBuf},
    rc::Rc,
    time::Duration,
    time::Instant,
};

/// How long a pane whose attach ended says nothing, while corral is asked whether its agent ended.
const QUIET: Duration = Duration::from_secs(3);

/// How often a pane looks at what it cannot be told about: whether its program ended, how a spawn
/// goes, the quiet after an attach and its name tag starting to fade. Output and changes the
/// pane is told about ring its bell at once.
const TICK: Duration = Duration::from_millis(100);

/// Shown in a pane with nothing in it.
const EMPTY_NOTE: &str = "Choose an agent on the left, or open one here with + or Split.";

/// Only the empty-pane hint wraps; session messages keep their existing layout.
fn note_lines(note: &str, width: f32, mut measure: impl FnMut(&str) -> f32) -> Vec<&str> {
    if note != EMPTY_NOTE {
        return vec![note];
    }
    let mut lines = Vec::new();
    let (mut start, mut end) = (0, 0);
    for next in note
        .match_indices(' ')
        .map(|(index, _)| index)
        .chain(std::iter::once(note.len()))
    {
        if end > start && measure(&note[start..next]) > width {
            lines.push(&note[start..end]);
            start = end + 1;
        }
        end = next;
    }
    lines.push(&note[start..]);
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_note_wraps_at_words_to_fit_the_pane() {
        let measure = |text: &str| text.len() as f32 * 8.0;
        let lines = note_lines(EMPTY_NOTE, 240.0, measure);
        assert_eq!(
            lines,
            [
                "Choose an agent on the left,",
                "or open one here with + or",
                "Split."
            ]
        );
        assert_eq!(lines.join(" "), EMPTY_NOTE);
        assert!(lines.iter().all(|line| measure(line) <= 240.0));
        assert_eq!(note_lines(EMPTY_NOTE, 800.0, measure), [EMPTY_NOTE]);
        let other_note = "A session error keeps its existing layout.";
        assert_eq!(note_lines(other_note, 80.0, measure), [other_note]);
    }
}

/// What runs in the pane.
#[derive(Clone)]
pub enum Launch {
    /// Nothing yet: a hint until an agent or shell is opened here.
    Empty,
    /// The user's shell, as Saddle starts plain terminals, with the identity `paddock ctl` knows
    /// it by (`PADDOCK_INSTANCE`, `PADDOCK_PANE`) in its environment.
    Shell {
        program: String,
        cwd: String,
        env: Vec<(String, String)>,
        agent_shell: bool,
    },
    /// `corral attach NAME` through Saddle's viewer, for the instance public corral named
    /// (`viewer::public_metadata`), when it did.
    Agent {
        name: String,
        metadata: AgentMetadata,
    },
    /// Any program, straight on a PTY (synthetic output tests).
    Command {
        argv: Vec<String>,
        cwd: Option<PathBuf>,
    },
}

#[derive(Clone)]
pub struct Options {
    pub launch: Launch,
    /// The `corral` program the pane attaches with.
    pub corral: String,
    /// The machine whose corral it is, when not this Mac (DESIGN §13 P5-90).
    pub remote: Option<std::sync::Arc<crate::remote::Link>>,
    pub font_family: String,
    /// Families tried for glyphs the main font lacks (prompt icons, symbols).
    pub fallbacks: Vec<String>,
    pub font_size: f32,
    pub line_height: f32,
    /// Settings' `blink_cursor` and `animate_cursor` (P5-143c1).
    pub blink_cursor: bool,
    pub animate_cursor: bool,
    /// Settings' `smooth_scroll` (P5-143c2).
    pub smooth_scroll: bool,
    pub stats: bool,
}

pub struct TerminalView {
    viewer: Viewer,
    /// Set only for `Launch::Command`; the viewer stays idle then.
    direct: Option<Session>,
    /// The directory `Launch::Command` ran in, for the paths it writes.
    direct_cwd: Option<PathBuf>,
    /// On another machine: the paths written here are not this Mac's.
    remote: bool,
    /// A shell to start once the previous session has ended.
    queued_shell: Option<Shell>,
    label: String,
    subject: String,
    focus: FocusHandle,
    font: Font,
    font_size: Pixels,
    line_height_factor: f32,
    metrics: Option<Metrics>,
    size: Size,
    origin: Point<Pixels>,
    /// Where the grid was drawn last.
    bounds: Bounds<Pixels>,
    theme: Rc<theme::Theme>,
    ime: Composition,
    scroll: Scroll,
    /// What the left button is doing: selecting, going to the program, or on a link (P5-143b).
    mouse: press::Mouse,
    /// Where the mouse is over the grid while ⌘ is held.
    pointer: Option<Point<Pixels>>,
    /// The link under it, underlined, the pointer a hand.
    link: Option<Link>,
    /// What is under the mouse as far as the files have been looked at, off the UI thread.
    hover: links::Hover,
    /// A few words over the pane's bottom, for a while: why a link did not open.
    notice: Option<notice::Notice>,
    /// Cursor cell of the last frame, for the input method's candidate window.
    cursor_cell: (u16, u16),
    /// When the pane was typed in, for its name tag (`to_tag`).
    typing: Typing,
    note: String,
    title: String,
    stats: Option<Rc<RefCell<Stats>>>,
    /// The find bar, while it is open.
    find: Option<FindBar>,
    /// The last poll found an attach to an agent under way or running.
    attaching: bool,
    /// Since that attach ended by itself, until corral has been asked whether its agent ended
    /// too: the pane, about to close if so, says nothing meanwhile.
    quiet: Option<Instant>,
    /// Rung by the session shown when output comes, and by what changes what is shown.
    bell: Bell,
    /// The session the bell was last handed to, or none.
    shown: Option<u64>,
    /// A frame was asked for and is not drawn yet: more output waits for it.
    frame_pending: bool,
    /// The screen was redrawn as a whole since the last frame: the cursor goes straight to its
    /// place.
    redrawn: bool,
    /// The cursor blinking and sliding, and the wait for its next frame.
    caret: caret::Caret,
    caret_wake: Option<gpui::Task<()>>,
    blink_cursor: bool,
    animate_cursor: bool,
    /// The history scrolled by the pixel, and a wheel's slide (P5-143c2); Settings' `smooth_scroll`.
    glide: glide::Glide,
    smooth_scroll: bool,
    /// A selection dragged past the top or bottom, and where the mouse is (P5-143d2).
    edge: Option<(edge::Edge, Point<Pixels>)>,
    /// Device pixels to a pixel, as the last frame was drawn.
    scale: f32,
}

struct FindBar {
    input: Entity<TextInput>,
    /// The last search found nothing.
    missed: bool,
    _subscription: Subscription,
}

#[derive(Default)]
struct Stats {
    since: Option<Instant>,
    frames: u32,
    build: Duration,
    build_max: Duration,
    paint: Duration,
    paint_max: Duration,
}

impl TerminalView {
    pub fn new(
        options: Options,
        theme: Rc<theme::Theme>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let size = Size { rows: 24, cols: 80 };
        let remote = options.remote.is_some();
        let mut direct_cwd = None;
        let mut viewer = Viewer::with(crate::corral::Client {
            program: options.corral,
            remote: options.remote,
        });
        let bell = Bell::new();
        viewer.set_theme(*theme.terminal());
        viewer.ring = Some(bell.for_changes());
        let mut direct = None;
        let mut note = String::new();
        let (label, subject) = match options.launch {
            Launch::Empty => {
                viewer.note = EMPTY_NOTE.into();
                ("empty".to_owned(), String::new())
            }
            Launch::Shell {
                program,
                cwd,
                env,
                agent_shell,
            } => {
                viewer.start_shell(Shell {
                    program: program.clone(),
                    cwd: cwd.clone(),
                    state: "starting",
                    exit_code: None,
                    env,
                    agent_shell,
                });
                (format!("{program} · {cwd}"), format!("shell · {cwd}"))
            }
            Launch::Agent { name, metadata } => {
                if let Err(error) = viewer.select_agent(name.clone(), metadata) {
                    note = format!("{error:#}");
                }
                (format!("corral attach {name}"), name)
            }
            Launch::Command { argv, cwd } => {
                direct_cwd = cwd.clone();
                match Session::spawn(&argv, cwd.as_deref(), size, viewer.theme.clone()) {
                    Ok(session) => direct = Some(session),
                    Err(error) => note = format!("{error:#}"),
                }
                let command = argv.join(" ");
                (command.clone(), command)
            }
        };
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        let font = terminal_font(&options.font_family, &options.fallbacks);
        // Woken by the bell; asleep while nothing rings.
        let rings = bell.receiver();
        cx.spawn(async move |this, cx| {
            while rings.recv().await.is_ok() {
                if this.update(cx, |view, cx| view.woken(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(TICK).await;
                if this.update(cx, |view, cx| view.poll(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        // A press on a link ends with the window's keyboard.
        cx.observe_window_activation(window, |this, window, cx| {
            if !window.is_window_active() {
                let act = this.mouse.cancel();
                this.act(act, Point::default(), gpui::Modifiers::default(), cx);
                this.edge = None;
                this.left(cx);
            }
        })
        .detach();
        // The first look, at once.
        bell.kick();
        Self {
            viewer,
            direct,
            direct_cwd,
            remote,
            queued_shell: None,
            label,
            subject,
            focus,
            font,
            font_size: px(options.font_size),
            line_height_factor: options.line_height,
            metrics: None,
            size,
            origin: Point::default(),
            bounds: Bounds::default(),
            theme,
            ime: Composition::default(),
            scroll: Scroll::default(),
            mouse: press::Mouse::default(),
            pointer: None,
            link: None,
            hover: links::Hover::new(
                std::sync::Arc::new(|path: &Path| path.exists()),
                links::thread,
            ),
            notice: None,
            cursor_cell: (0, 0),
            typing: Typing::default(),
            note,
            title: String::new(),
            stats: options
                .stats
                .then(|| Rc::new(RefCell::new(Stats::default()))),
            find: None,
            attaching: false,
            quiet: None,
            bell,
            shown: None,
            frame_pending: false,
            redrawn: false,
            caret: caret::Caret::default(),
            caret_wake: None,
            blink_cursor: options.blink_cursor,
            animate_cursor: options.animate_cursor,
            glide: glide::Glide::default(),
            edge: None,
            smooth_scroll: options.smooth_scroll,
            scale: 1.0,
        }
    }

    /// New colours from Settings, at once; a program asking for colours from now on is told them.
    pub fn set_theme(&mut self, theme: Rc<theme::Theme>, cx: &mut Context<Self>) {
        self.theme = theme;
        // Shared with every session here, the ones starting too.
        self.viewer.set_theme(*self.theme.terminal());
        cx.notify();
    }

    /// A new font, size or line height from Settings, at once: the next frame measures the cells
    /// again and resizes the PTY, as dragging the window does.
    pub fn set_font(&mut self, options: &Options, cx: &mut Context<Self>) {
        self.font = terminal_font(&options.font_family, &options.fallbacks);
        self.font_size = px(options.font_size);
        self.line_height_factor = options.line_height;
        self.metrics = None;
        // Pixels past a line of the old height.
        self.glide.stop();
        cx.notify();
    }

    /// Whether the cursor blinks and slides, from Settings, at once.
    pub fn set_cursor(&mut self, options: &Options, cx: &mut Context<Self>) {
        self.blink_cursor = options.blink_cursor;
        self.animate_cursor = options.animate_cursor;
        cx.notify();
    }

    /// Whether the history scrolls by the pixel, from Settings, at once: turned off, the view goes
    /// to its whole line.
    pub fn set_smooth_scroll(&mut self, options: &Options, cx: &mut Context<Self>) {
        self.smooth_scroll = options.smooth_scroll;
        if !self.smooth_scroll {
            self.glide.stop();
        }
        cx.notify();
    }

    /// What the pane is connected to, for the title bar: `shell · <cwd>`, the agent, or the command.
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// A shell's directory.
    pub fn cwd(&self) -> Option<&str> {
        self.subject.strip_prefix("shell · ")
    }

    /// The directory and instance the pane attached to its agent with.
    pub fn agent_metadata(&self) -> AgentMetadata {
        self.viewer.target_metadata().clone()
    }

    /// The agent the pane shows or is attaching to.
    pub fn target(&self) -> Option<&str> {
        self.viewer.target()
    }

    /// Attaches the pane to an agent; whatever ran before ends, a shell included.
    pub fn attach(&mut self, name: String, metadata: AgentMetadata, cx: &mut Context<Self>) {
        self.retire_direct();
        self.queued_shell = None;
        self.quiet = None;
        self.viewer.shell = None;
        if let Err(error) = self.viewer.select_agent(name.clone(), metadata) {
            self.note = format!("{error:#}");
        }
        self.label = format!("corral attach {name}");
        self.subject = name;
        self.bell.kick();
        cx.notify();
    }

    /// Ends what the pane runs and starts an interactive shell once it is gone.
    pub fn start_shell(
        &mut self,
        program: String,
        cwd: String,
        env: Vec<(String, String)>,
        cx: &mut Context<Self>,
    ) {
        self.retire_direct();
        if let Err(error) = self.viewer.close() {
            self.note = format!("{error:#}");
        }
        // Ended here on purpose: no wait for corral.
        (self.attaching, self.quiet) = (false, None);
        self.label = format!("{program} · {cwd}");
        self.subject = format!("shell · {cwd}");
        self.queued_shell = Some(Shell {
            program,
            cwd,
            state: "starting",
            exit_code: None,
            env,
            agent_shell: false,
        });
        self.bell.kick();
        cx.notify();
    }

    /// A shell runs here, or is about to.
    pub fn shell_live(&self) -> bool {
        self.queued_shell.is_some() || self.viewer.shell_live()
    }

    /// What runs in this pane's shell besides the shell itself; empty at its prompt and while the
    /// shell is still queued.
    pub fn shell_programs(&self) -> Vec<String> {
        self.viewer.shell_programs()
    }

    /// The title the program in the pane last set for its terminal, for the pane's header.
    pub fn terminal_title(&self) -> Option<String> {
        let session = self.session()?;
        let screen = session.screen.lock().unwrap();
        screen.title().map(str::to_owned)
    }

    /// What `paddock ctl` reads of the pane.
    pub fn facts(&self) -> crate::control_ui::Facts {
        let shell = self.queued_shell.as_ref().or(self.viewer.shell.as_ref());
        let env = |key: &str| {
            shell.and_then(|s| s.env.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone()))
        };
        let identity =
            env("PADDOCK_INSTANCE").zip(env("PADDOCK_PANE").and_then(|p| p.parse().ok()));
        let state = match (&self.direct, shell) {
            (Some(session), _) if session.running() => "running",
            (Some(_), _) => "exited",
            (None, Some(_)) if self.queued_shell.is_some() => "starting",
            (None, Some(shell)) => shell.state,
            // Opened empty, or emptied by `close`.
            (None, None) if self.label == "empty" => "empty",
            (None, None) => self.viewer.state(),
        };
        let attached = (self.direct.is_none() && shell.is_none() && state == "running")
            .then(|| self.viewer.showing.clone())
            .flatten();
        crate::control_ui::Facts {
            state: state.into(),
            attached,
            instance: self.viewer.target_metadata().instance.clone(),
            shell_live: self.shell_live(),
            agent_shell: shell.is_some_and(|shell| shell.agent_shell),
            identity,
            cwd: shell
                .map(|s| s.cwd.clone())
                .or_else(|| self.viewer.target_metadata().cwd.clone()),
            program: match shell {
                Some(shell) => Some(shell.program.clone()),
                None => self.direct.is_some().then(|| self.label.clone()),
            },
            exit_code: shell
                .and_then(|s| s.exit_code)
                .or(self.direct.as_ref().and_then(Session::exit_code))
                .or(self.viewer.exit_code),
            command: self.direct.is_some(),
            note: if self.note.is_empty() {
                self.viewer.note.clone()
            } else {
                self.note.clone()
            },
        }
    }

    /// Hands live PTYs to the app's quit callback so every pane can hang up before any waits.
    pub fn take_sessions_for_quit(&mut self) -> impl Iterator<Item = Session> + use<> {
        self.viewer.cancel_pending();
        [self.direct.take(), self.viewer.session.take()]
            .into_iter()
            .flatten()
    }

    /// Ends what runs here, off the UI thread, and leaves the pane empty: closing a pane or tab,
    /// or moving its agent elsewhere. An attached agent is only let go; it keeps running.
    pub fn close(&mut self, cx: &mut Context<Self>) {
        self.retire_direct();
        self.queued_shell = None;
        (self.attaching, self.quiet) = (false, None);
        if let Err(error) = self.viewer.close() {
            self.note = format!("{error:#}");
        }
        if let Some(session) = self.viewer.session.take() {
            std::thread::spawn(move || drop(session));
        }
        self.viewer.shell = None;
        self.viewer.note = EMPTY_NOTE.into();
        self.label = "empty".into();
        self.subject.clear();
        self.bell.kick();
        cx.notify();
    }

    /// Agents still listed by corral; an attached agent missing from them is let go.
    pub fn disappeared(&mut self, names: &[&str]) {
        if let Err(error) = self.viewer.disappeared(names) {
            self.note = format!("{error:#}");
        }
    }

    /// Its machine's agents as just read, for another machine's pane (DESIGN §13 P5-90): a lost
    /// attach goes again, and one that only watched may type. `read_began`: when that read began.
    pub fn listed(
        &mut self,
        agents: &[crate::corral::Agent],
        read_began: Option<std::time::Instant>,
        cx: &mut Context<Self>,
    ) {
        let (read_only, lost) = (self.viewer.read_only, self.viewer.lost.is_some());
        if let Err(error) = self.viewer.listed(agents, read_began) {
            self.note = format!("{error:#}");
        }
        // A lost attach going again starts at once.
        if lost && self.viewer.lost.is_none() {
            self.bell.kick();
        }
        if read_only != self.viewer.read_only {
            cx.notify();
        }
    }

    /// Another machine's agent from the saved layout, shown by name until that machine answers
    /// and lists it as before; then it attaches (DESIGN §13 P5-90).
    pub fn wait(
        &mut self,
        name: String,
        metadata: AgentMetadata,
        machine: &str,
        cx: &mut Context<Self>,
    ) {
        self.viewer
            .wait(name.clone(), metadata, format!("Connecting to {machine}…"));
        self.label = format!("corral attach {name}");
        self.subject = name;
        cx.notify();
    }

    /// Over another machine's pane: it only watches, while another window there is attached.
    pub fn banner(&self) -> Option<&'static str> {
        (self.viewer.read_only && self.viewer.showing.is_some())
            .then_some("Read-only: attached elsewhere")
    }

    /// A `Launch::Command` session ends off the UI thread, since dropping one waits for its exit.
    fn retire_direct(&mut self) {
        if let Some(mut session) = self.direct.take() {
            let _ = session.interrupt();
            std::thread::spawn(move || drop(session));
        }
    }

    fn session(&self) -> Option<&Session> {
        self.direct.as_ref().or(self.viewer.session.as_ref())
    }

    fn session_mut(&mut self) -> Option<&mut Session> {
        self.direct.as_mut().or(self.viewer.session.as_mut())
    }

    fn state(&self) -> String {
        match &self.direct {
            Some(session) if session.running() => "running".into(),
            Some(session) => format!("exited ({})", session.exit_code().unwrap_or(0)),
            None if self
                .viewer
                .shell
                .as_ref()
                .is_some_and(|s| s.state == "exited") =>
            {
                format!(
                    "exited ({})",
                    self.viewer
                        .shell
                        .as_ref()
                        .and_then(|s| s.exit_code)
                        .unwrap_or(0)
                )
            }
            None => self.viewer.state().into(),
        }
    }

    /// An attach to an agent is under way or running here, and the pane is not letting it go for a
    /// shell or to be empty.
    pub fn attaching(&self) -> bool {
        self.direct.is_none()
            && self.queued_shell.is_none()
            && self.viewer.shell.is_none()
            && self.label != "empty"
            && !self.viewer.closed()
    }

    /// The bell rang: only output, or something else too.
    fn woken(&mut self, cx: &mut Context<Self>) {
        if self.bell.take_changed() {
            self.poll(cx);
        } else {
            self.output(cx);
        }
    }

    /// Output came: a frame, unless one is already on its way, which shows it too.
    fn output(&mut self, cx: &mut Context<Self>) {
        if !self.frame_pending && self.take_changed() {
            self.frame_pending = true;
            cx.notify();
        }
    }

    /// Whether the screen changed since the last look; not a bare cursor ([`damage`]).
    fn take_changed(&mut self) -> bool {
        let change = self.session().map_or(damage::Change::None, |s| {
            damage::take(&mut s.screen.lock().unwrap().term)
        });
        self.redrawn |= change == damage::Change::Whole;
        change != damage::Change::None
    }

    /// The session shown, when it is new: handed the bell, so the one before rings no more.
    fn adopt(&mut self) {
        let id = self.session().map(Session::id);
        if id == self.shown {
            return;
        }
        self.shown = id;
        match self.session() {
            Some(session) => session.set_ring(self.bell.for_session()),
            None => self.bell.silence(),
        }
    }

    /// Drives the session the way Saddle's main loop does, and repaints when output arrived: on
    /// every [`TICK`] and when the bell rings for a change.
    fn poll(&mut self, cx: &mut Context<Self>) {
        let size = self.size;
        if let Some(session) = &mut self.direct {
            let _ = session.poll_exit();
            let _ = session.resize(size);
        } else if let Err(error) = self.viewer.tick(size) {
            self.note = format!("{error:#}");
        }
        if self.queued_shell.is_some() && self.viewer.closed() {
            self.viewer.start_shell(self.queued_shell.take().unwrap());
            // Its spawn begins now, not a tick later.
            if let Err(error) = self.viewer.tick(size) {
                self.note = format!("{error:#}");
            }
        }
        self.adopt();
        let attaching = self.attaching();
        if self.attaching && !attaching {
            self.quiet = Some(Instant::now());
            cx.emit(AttachEnded);
            cx.notify();
        }
        self.attaching = attaching;
        if self.quiet.is_some_and(|since| since.elapsed() >= QUIET) {
            self.quiet = None;
            cx.notify();
        }
        // The name tag starts to fade with nothing else to draw it (`to_tag`).
        if self.typing.due(Instant::now()) {
            cx.notify();
        }
        self.output(cx);
        let title = format!("{} — {}", self.label, self.state());
        if title != self.title {
            self.title = title;
            cx.notify();
        }
    }

    fn mode(&self) -> TermMode {
        self.session()
            .map_or(TermMode::empty(), |s| *s.screen.lock().unwrap().term.mode())
    }

    /// Whether the agent shown is paused: the pane keeps its screen and takes no input.
    pub fn paused(&self) -> bool {
        self.viewer.paused
    }

    /// The agent shown was paused or resumed, as the window last heard from corral.
    pub fn set_paused(&mut self, paused: bool, cx: &mut Context<Self>) {
        if self.viewer.paused != paused {
            self.viewer.paused = paused;
            cx.notify();
        }
    }

    /// Bytes for the program; the viewer's drop them while its agent is paused.
    fn send(&self, bytes: Vec<u8>) -> anyhow::Result<()> {
        match &self.direct {
            Some(session) => {
                session.send(bytes);
                Ok(())
            }
            None => self.viewer.send(bytes).map(drop),
        }
    }

    /// Typed input: back to the live bottom, selection cleared, bytes to the PTY. Nothing while
    /// the agent is paused, not even the view moving.
    fn write(&mut self, bytes: Vec<u8>, cx: &mut Context<Self>) {
        // Nor while another machine's connection is down: it says so, and comes back by itself.
        if bytes.is_empty() || self.paused() || self.viewer.lost.is_some() {
            return;
        }
        let Some(session) = self.session() else {
            self.note = "no running session".into();
            return;
        };
        {
            let mut screen = session.screen.lock().unwrap();
            screen.term.selection = None;
            screen.term.scroll_display(ViewScroll::Bottom);
        }
        self.glide.stop();
        if let Err(error) = self.send(bytes) {
            self.note = format!("{error:#}");
        }
        // Shown solid while typing, blinking once it stops.
        self.caret.touch(Instant::now());
        cx.notify();
    }

    /// Text from the keyboard for the program: the pane's name tag is lit while it comes
    /// (`to_tag`).
    fn typed(&mut self, text: &str, cx: &mut Context<Self>) {
        self.typing.typed(text, Instant::now());
        cx.notify();
    }

    /// The typing goes to someone else than before through this pane: its lit name tag goes at
    /// once.
    pub fn tag_elsewhere(&mut self) {
        self.typing.elsewhere();
    }

    /// How lit the pane's name tag is at `now`, 0 at rest to 1; how many points below its place
    /// it is, sliding up; and whether it is on its way, to be drawn anew each frame.
    pub fn tag_lit(&self, now: Instant) -> (f32, f32, bool) {
        let typing = &self.typing;
        (typing.glow(now), typing.rise(now), typing.moving(now))
    }

    /// How far below the top of the terminal's area a name tag `width` wide and `tag` tall goes,
    /// kept `end` in from the terminal's right ([`to_tag::flush_right`]): on the nearest row
    /// above the cursor whose cells there hold nothing ([`to_tag::free_row`]); none for the
    /// terminal's lower right corner.
    pub fn tag_top(&self, width: f32, tag: f32, end: f32) -> Option<f32> {
        let Metrics {
            cell_width: cell,
            line_height,
        } = self.metrics?;
        let screen = self.session()?.screen.lock().unwrap();
        let term = &screen.term;
        let grid = term.grid();
        let offset = grid.display_offset();
        let cols = self.size.cols.min(term.columns() as u16);
        let row = to_tag::row_in_view(grid.cursor.point.line.0, offset, term.screen_lines());
        // The cells the tag would cover on a row: from its left to the terminal's right.
        let left = to_tag::flush_right(width, f32::from(cols) * cell, end);
        let first = (left / cell).floor() as u16;
        let free = |row: u16| {
            let line = Line(i32::from(row) - offset as i32);
            (first..cols).all(|col| to_tag::blank(grid[line][Column(usize::from(col))].c))
        };
        let on = to_tag::free_row(row, free)?;
        // With the rows, part of a line down.
        Some(to_tag::top_on(on, line_height, tag) + self.shift())
    }

    fn paste(&mut self, text: &str, cx: &mut Context<Self>) {
        let bracketed = self.mode().contains(TermMode::BRACKETED_PASTE);
        // Without bracketed paste a newline must arrive as Enter.
        let text = if bracketed {
            text.to_owned()
        } else {
            text.replace("\r\n", "\r").replace('\n', "\r")
        };
        self.write(crate::input::encode_paste(&text, bracketed), cx);
    }

    pub(crate) fn drop_files(
        &mut self,
        paths: &ExternalPaths,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let text = file_drop::text(paths.paths());
        if !text.is_empty() {
            window.focus(&self.focus, cx);
            self.paste(&text, cx);
        }
    }

    fn selection_text(&self) -> Option<String> {
        let screen = self.session()?.screen.lock().unwrap();
        screen.term.selection_to_string().filter(|s| !s.is_empty())
    }

    /// ⌘C: the selection, when there is one. Not while the Browser's page has the keyboard: the
    /// page copies its own.
    fn copy(&mut self, _: &menu::Copy, window: &mut Window, cx: &mut Context<Self>) {
        if crate::browser::page_has_keys(window) {
            cx.propagate();
            return;
        }
        if let Some(text) = self.selection_text() {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
    }

    /// ⌘V: the clipboard's text into the terminal. Not while the Browser's page has the keyboard:
    /// it goes into the page.
    fn paste_clipboard(&mut self, _: &menu::Paste, window: &mut Window, cx: &mut Context<Self>) {
        if crate::browser::page_has_keys(window) {
            cx.propagate();
            return;
        }
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.typed(&text, cx);
            self.paste(&text, cx);
        }
    }

    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        // Keys typed in the find bar are its own.
        if !self.focus.is_focused(window) {
            return;
        }
        let keystroke = &event.keystroke;
        // Command shortcuts are menu actions (`menu.rs`); none of them reach the terminal.
        if keystroke.modifiers.platform {
            return;
        }
        let application_cursor = self.mode().contains(TermMode::APP_CURSOR);
        if let Some(bytes) = keys::key_bytes(keystroke, application_cursor) {
            // Enter alone sends what was typed: the name tag fades (`to_tag`).
            let held = &keystroke.modifiers;
            if keystroke.key == "enter" && !(held.shift || held.alt || held.control) {
                self.typing.sent(Instant::now());
            }
            self.write(bytes, cx);
            cx.stop_propagation();
        }
    }

    /// ⌘F: the find bar, its field focused.
    fn open_find(&mut self, _: &menu::Find, window: &mut Window, cx: &mut Context<Self>) {
        let input = match &self.find {
            Some(bar) => bar.input.clone(),
            None => {
                let colors = text_input::Colors {
                    text: hsla(self.theme.fg(|t| t.agents_text), 1.0),
                    placeholder: hsla(self.theme.fg(|t| t.agents_dimmer), 1.0),
                    cursor: hsla(self.theme.fg(|t| t.focus), 1.0),
                    selection: hsla(self.theme.fg(|t| t.focus), 0.3),
                };
                let input = cx.new(|cx| TextInput::new("", "Find", colors, cx));
                // Typing searches again from the bottom of the view.
                let subscription = cx.subscribe(&input, |this, _, _: &Changed, cx| {
                    this.run_find(Find::First, cx)
                });
                self.find = Some(FindBar {
                    input: input.clone(),
                    missed: false,
                    _subscription: subscription,
                });
                input
            }
        };
        let focus = input.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        cx.notify();
    }

    fn find_next(&mut self, _: &menu::FindNext, window: &mut Window, cx: &mut Context<Self>) {
        if self.find.is_none() {
            self.open_find(&menu::Find, window, cx);
        }
        self.run_find(Find::Older, cx);
    }

    fn find_previous(
        &mut self,
        _: &menu::FindPrevious,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.find.is_none() {
            self.open_find(&menu::Find, window, cx);
        }
        self.run_find(Find::Newer, cx);
    }

    fn run_find(&mut self, how: Find, cx: &mut Context<Self>) {
        let Some(bar) = &self.find else { return };
        let query = bar.input.read(cx).text().to_owned();
        let found = match self.session() {
            Some(session) => find::find(&mut session.screen.lock().unwrap().term, &query, how),
            None => false,
        };
        // A match is shown on whole lines.
        if found {
            self.glide.stop();
        }
        if let Some(bar) = &mut self.find {
            bar.missed = !query.is_empty() && !found;
        }
        cx.notify();
    }

    /// Esc or ×: the bar goes, the view returns to the bottom and the keys to the terminal.
    fn close_find(&mut self, _: &menu::CloseFind, window: &mut Window, cx: &mut Context<Self>) {
        self.find = None;
        if let Some(session) = self.session() {
            let mut screen = session.screen.lock().unwrap();
            screen.term.selection = None;
            screen.term.scroll_display(ViewScroll::Bottom);
        }
        self.glide.stop();
        window.focus(&self.focus, cx);
        cx.notify();
    }

    /// Why a link did not open, over the pane's bottom.
    fn notice_bar(&self, text: String, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = crate::fonts::UiFont::get(cx);
        div()
            .absolute()
            .bottom(px(10.0))
            .left(px(10.0))
            .right(px(10.0))
            .flex()
            .justify_center()
            .child(
                div()
                    .px(px(10.0))
                    .py(px(5.0))
                    .rounded(px(7.0))
                    .border_1()
                    .border_color(hsla(self.theme.fg(|t| t.danger), 1.0))
                    .bg(hsla(self.theme.bg(|t| t.agents_bg), 1.0))
                    .shadow_md()
                    .text_size(ui.px(12.0))
                    .text_color(hsla(self.theme.fg(|t| t.agents_text), 1.0))
                    .child(text),
            )
    }

    fn find_bar(&self, bar: &FindBar, cx: &mut Context<Self>) -> impl IntoElement {
        let fg = |pick: fn(&crate::preset::Theme) -> crate::preset::Color| {
            hsla(self.theme.fg(pick), 1.0)
        };
        let ui = crate::fonts::UiFont::get(cx);
        let highlight = hsla(self.theme.bg(|t| t.agent_selected), 1.0);
        let button = |id: &'static str, label: &'static str| {
            div()
                .id(id)
                .px(px(5.0))
                .rounded(px(4.0))
                .text_color(fg(|t| t.muted))
                .cursor_pointer()
                .hover(move |style| style.bg(highlight))
                .child(label)
        };
        div()
            .id("find-bar")
            .key_context(menu::FIND)
            .absolute()
            .top(px(6.0))
            .right(px(14.0))
            .w(px(340.0))
            .flex()
            .items_center()
            .gap(px(6.0))
            .px(px(8.0))
            .py(px(4.0))
            .rounded(px(7.0))
            .border_1()
            .border_color(fg(|t| t.focus))
            .bg(hsla(self.theme.bg(|t| t.agents_bg), 1.0))
            .shadow_md()
            .text_size(ui.px(12.0))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_action(cx.listener(Self::close_find))
            .child(div().flex_1().min_w(px(0.0)).child(bar.input.clone()))
            .when(bar.missed, |row| {
                row.child(
                    div()
                        .flex_shrink_0()
                        .text_color(fg(|t| t.agents_red))
                        .child("No match"),
                )
            })
            .child(button("find-older", "↑").on_click(
                cx.listener(|this, _, window, cx| this.find_next(&menu::FindNext, window, cx)),
            ))
            .child(
                button("find-newer", "↓").on_click(cx.listener(|this, _, window, cx| {
                    this.find_previous(&menu::FindPrevious, window, cx)
                })),
            )
            .child(button("find-close", "×").on_click(
                cx.listener(|this, _, window, cx| this.close_find(&menu::CloseFind, window, cx)),
            ))
    }

    /// The grid cell under a window position, and the grid point counting the scrolled view.
    fn locate(&self, position: Point<Pixels>) -> Option<(u16, u16, GridPoint, Side)> {
        let offset = self
            .session()?
            .screen
            .lock()
            .unwrap()
            .term
            .grid()
            .display_offset();
        self.cell_at(position, offset)
    }

    /// How far down the rows are drawn, the view part of a line further back (P5-143c2).
    fn shift(&self) -> f32 {
        self.glide.shift(self.scale)
    }

    /// [`Self::locate`] with the view scrolled `offset` lines back, for a caller holding the grid.
    fn cell_at(
        &self,
        position: Point<Pixels>,
        offset: usize,
    ) -> Option<(u16, u16, GridPoint, Side)> {
        let metrics = self.metrics?;
        let (col, row, right) = grid::cell_at(
            f32::from(position.x - self.origin.x),
            f32::from(position.y - self.origin.y),
            metrics,
            self.size,
            self.shift(),
        );
        let point = GridPoint::new(Line(row - offset as i32), Column(usize::from(col)));
        Some((
            col,
            // The program is told of the rows it has, from the first.
            row.max(0) as u16,
            point,
            if right { Side::Right } else { Side::Left },
        ))
    }

    /// The directory this pane's running shell is in now, for a shell opened from the pane; `None`
    /// for anything else, and for a shell that has not yet said.
    pub fn shell_dir(&self) -> Option<PathBuf> {
        self.queued_shell
            .is_none()
            .then(|| self.viewer.shell_dir())
            .flatten()
    }

    /// The pane's directory, which relative paths written in it are from: a shell's, where it is now
    /// (else where it was opened), an agent's, or a command's.
    fn directory(&self) -> Option<PathBuf> {
        if let Some(now) = self.shell_dir() {
            return Some(now);
        }
        let shell = self.queued_shell.as_ref().or(self.viewer.shell.as_ref());
        let dir = match shell {
            Some(shell) => shell.cwd.clone(),
            None if self.direct.is_some() => return self.direct_cwd.clone(),
            None => self.viewer.target_metadata().cwd.clone()?,
        };
        let home = std::env::var_os("HOME").map(PathBuf::from);
        Some(match (dir.strip_prefix('~'), home) {
            (Some(""), Some(home)) => home,
            (Some(rest), Some(home)) if rest.starts_with('/') => home.join(&rest[1..]),
            _ => PathBuf::from(dir),
        })
    }

    /// Where the paths written in the pane are on this Mac; none on another machine.
    fn place(&self) -> Option<links::Place> {
        (!self.remote).then(|| links::Place {
            base: self.directory(),
            home: std::env::var_os("HOME").map(PathBuf::from),
            host: links::this_host(),
        })
    }

    /// The link under a window position, as far as known now: the grid is read, the files are
    /// not ([`Self::look_at_files`]).
    fn link_at(&mut self, position: Point<Pixels>) -> Option<Link> {
        if !self.bounds.contains(&position) {
            return None;
        }
        let screen = self.session()?.screen.clone();
        let offset = screen.lock().unwrap().term.grid().display_offset();
        let (.., point, _) = self.cell_at(position, offset)?;
        let place = self.place();
        links::look(&screen, point, place.as_ref(), &mut self.hover)
    }

    /// Starts a look at the files under the mouse on a thread of its own, when one is due; when
    /// it comes back, the link shows if the mouse is still over it.
    fn look_at_files(&mut self, cx: &mut Context<Self>) {
        let Some(look) = self.hover.start() else {
            return;
        };
        cx.spawn(async move |this, cx| {
            let answer = look.answer.recv().await;
            let _ = this.update(cx, |view, cx| {
                match answer {
                    Ok((candidate, link)) => {
                        if view.hover.checked(candidate, link) && view.pointer.is_some() {
                            view.link = view.hover.link();
                            cx.notify();
                        }
                    }
                    Err(_) => view.hover.lost(look.candidate),
                }
                view.look_at_files(cx);
            });
        })
        .detach();
    }

    /// The mouse at `position`, ⌘ held or not: with it, the link there is underlined and the
    /// pointer a hand; without it, or off the grid, nothing is.
    fn hover(&mut self, position: Point<Pixels>, command: bool, cx: &mut Context<Self>) {
        let pointer = (command && self.bounds.contains(&position)).then_some(position);
        let link = match pointer {
            Some(position) => self.link_at(position),
            None => {
                self.hover.forget();
                None
            }
        };
        self.look_at_files(cx);
        if pointer != self.pointer || link != self.link {
            (self.pointer, self.link) = (pointer, link);
            cx.notify();
        }
    }

    /// ⌘ pressed or let go over the pane with the keyboard.
    fn modifiers_changed(
        &mut self,
        event: &ModifiersChangedEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.mouse.pressed() {
            self.hover(window.mouse_position(), event.modifiers.platform, cx);
        }
    }

    /// The mouse went off the grid.
    fn left(&mut self, cx: &mut Context<Self>) {
        self.hover.forget();
        if self.pointer.is_some() || self.link.is_some() {
            (self.pointer, self.link) = (None, None);
            cx.notify();
        }
    }

    /// A few words over the pane's bottom, for [`notice::NOTICE`] once the pane shows: why a link
    /// did not open, seen where it was clicked whatever the sidebar shows, and not missed while
    /// another tab or machine is shown.
    pub fn say(&mut self, text: String, cx: &mut Context<Self>) {
        self.notice = Some(notice::Notice::new(text));
        cx.notify();
    }

    /// Reports a mouse event through `input::encode_mouse` when the program asked for the mouse,
    /// unless its agent is paused: then the mouse selects, as without reporting.
    fn report_mouse(
        &mut self,
        kind: crate::input::MouseEventKind,
        position: Point<Pixels>,
        modifiers: gpui::Modifiers,
        cx: &mut Context<Self>,
    ) -> bool {
        let mode = self.mode();
        if !mode.intersects(TermMode::MOUSE_MODE) || self.paused() {
            return false;
        }
        let Some((col, row, ..)) = self.locate(position) else {
            return false;
        };
        let event = crate::input::MouseEvent {
            kind,
            column: col,
            row,
            modifiers: crate::input::Modifiers {
                shift: modifiers.shift,
                alt: modifiers.alt,
                control: modifiers.control,
            },
        };
        let area = crate::input::Area::new(0, 0, self.size.cols, self.size.rows);
        let bytes = crate::input::encode_mouse(event, area, mode);
        if !bytes.is_empty() {
            let _ = self.send(bytes);
        }
        cx.notify();
        true
    }

    /// Does what [`press::Mouse`] says, the mouse at `position`.
    fn act(
        &mut self,
        act: press::Act,
        position: Point<Pixels>,
        modifiers: gpui::Modifiers,
        cx: &mut Context<Self>,
    ) {
        use press::Act;
        let selecting = |view: &Self, change: &dyn Fn(&mut Option<Selection>)| {
            if let Some(session) = view.session() {
                change(&mut session.screen.lock().unwrap().term.selection);
            }
        };
        match act {
            Act::Nothing => return,
            Act::Report(kind) => {
                self.report_mouse(kind, position, modifiers, cx);
                return;
            }
            Act::Select(kind, (point, side)) => {
                selecting(self, &|selection| {
                    *selection = Some(Selection::new(kind, point, side))
                });
            }
            Act::Extend((point, side)) => selecting(self, &|selection| {
                if let Some(selection) = selection {
                    selection.update(point, side);
                }
            }),
            Act::SelectFrom((at, side), (point, now)) => {
                selecting(self, &|selection| {
                    let mut from = Selection::new(SelectionType::Simple, at, side);
                    from.update(point, now);
                    *selection = Some(from);
                });
                self.left(cx);
            }
            Act::EndSelect => selecting(self, &|selection| {
                if selection.as_ref().is_some_and(|s| s.is_empty()) {
                    *selection = None;
                }
            }),
            Act::Open(target) => {
                cx.emit(OpenLink(target));
                return;
            }
        }
        cx.notify();
    }

    fn mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus, cx);
        let command = event.modifiers.platform;
        let link = if command {
            self.link_at(event.position)
        } else {
            None
        };
        if link.is_some() {
            (self.pointer, self.link) = (Some(event.position), link.clone());
        }
        let down = press::Down {
            link,
            command,
            clicks: event.click_count,
            shift: event.modifiers.shift,
            reports: self.mode().intersects(TermMode::MOUSE_MODE) && !self.paused(),
            cell: self
                .locate(event.position)
                .map(|(.., point, side)| (point, side)),
        };
        let act = self.mouse.down(down);
        self.act(act, event.position, event.modifiers, cx);
    }

    fn mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        use crate::input::MouseEventKind as K;
        if !self.mouse.pressed() {
            if event.pressed_button.is_none() {
                self.hover(event.position, event.modifiers.platform, cx);
                if self.mode().contains(TermMode::MOUSE_MOTION) {
                    self.report_mouse(K::Moved, event.position, event.modifiers, cx);
                }
            }
            return;
        }
        let cell = self
            .locate(event.position)
            .map(|(.., point, side)| (point, side));
        let left = event.pressed_button == Some(MouseButton::Left);
        let act = self.mouse.pane_move(cell, left);
        self.act(act, event.position, event.modifiers, cx);
    }

    fn mouse_up(&mut self, event: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        let act = self.mouse.pane_up();
        self.act(act, event.position, event.modifiers, cx);
        self.edge = None;
    }

    /// The mouse moved anywhere in the window: off the grid, a link under it is no longer; a
    /// press on a link is followed out of the pane too.
    fn window_move(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        let inside = self.bounds.contains(&event.position);
        if !inside {
            self.left(cx);
        }
        // The grid is only read while a press goes on.
        if !self.mouse.pressed() {
            return;
        }
        let cell = self
            .locate(event.position)
            .map(|(.., point, side)| (point, side));
        let left = event.pressed_button == Some(MouseButton::Left);
        let act = self.mouse.window_move(cell, inside, left);
        self.act(act, event.position, event.modifiers, cx);
        self.follow_edge(event.position, left, cx);
    }

    /// Let go anywhere in the window.
    fn window_up(&mut self, event: &MouseUpEvent, cx: &mut Context<Self>) {
        let inside = self.bounds.contains(&event.position);
        let cell = self
            .locate(event.position)
            .map(|(.., point, side)| (point, side));
        let act = self.mouse.window_up(cell, inside);
        self.act(act, event.position, event.modifiers, cx);
        self.edge = None;
    }

    /// A selection dragged above or below the grid goes on through the history, frame by frame
    /// ([`Self::build`]); back over the grid, or let go, it stops (P5-143d2).
    fn follow_edge(&mut self, position: Point<Pixels>, left: bool, cx: &mut Context<Self>) {
        let Some(metrics) = self.metrics else {
            return;
        };
        let y = f32::from(position.y - self.bounds.origin.y);
        let height = f32::from(self.bounds.size.height);
        let full_screen = self.mode().contains(TermMode::ALT_SCREEN);
        let selecting = left && self.mouse.selecting();
        if !edge::wanted(selecting, full_screen, y, height, metrics.line_height) {
            self.edge = None;
            return;
        }
        match &mut self.edge {
            Some((edge, at)) => {
                edge.aim(y, height);
                *at = position;
            }
            None => {
                self.edge = Some((edge::Edge::new(y, height, Instant::now()), position));
                cx.notify();
            }
        }
    }

    fn scroll_wheel(&mut self, event: &ScrollWheelEvent, _: &mut Window, cx: &mut Context<Self>) {
        let Some(metrics) = self.metrics else {
            return;
        };
        let mode = self.mode();
        // A paused agent takes no wheel either: the view scrolls its history.
        let goes = glide::wheel_goes(mode, event.modifiers.shift, self.paused());
        if goes == glide::Wheel::History && self.smooth_scroll {
            self.glide_by(event.delta, metrics.line_height, cx);
            return;
        }
        let delta = match event.delta {
            ScrollDelta::Lines(lines) => lines.y,
            ScrollDelta::Pixels(pixels) => f32::from(pixels.y) / metrics.line_height,
        };
        let lines = self.scroll.lines(delta);
        if lines == 0 {
            return;
        }
        use crate::input::MouseEventKind as K;
        if goes == glide::Wheel::Program {
            let kind = if lines > 0 {
                K::ScrollUp
            } else {
                K::ScrollDown
            };
            for _ in 0..lines.unsigned_abs().min(20) {
                self.report_mouse(kind, event.position, event.modifiers, cx);
            }
        } else if goes == glide::Wheel::Arrows {
            // Full-screen programs without mouse reporting scroll with arrow keys.
            let arrow: &[u8] = match (lines > 0, mode.contains(TermMode::APP_CURSOR)) {
                (true, true) => b"\x1bOA",
                (true, false) => b"\x1b[A",
                (false, true) => b"\x1bOB",
                (false, false) => b"\x1b[B",
            };
            let bytes = arrow.repeat(lines.unsigned_abs().min(20) as usize);
            let _ = self.send(bytes);
        } else if let Some(session) = self.session() {
            session
                .screen
                .lock()
                .unwrap()
                .term
                .scroll_display(ViewScroll::Delta(lines));
            cx.notify();
        }
    }

    /// The history by the pixel (`glide`): the trackpad's pixels, its momentum among them, move
    /// the view at once; a wheel's lines slide there, frame by frame (`build`), unless the system
    /// asks for less motion.
    fn glide_by(&mut self, delta: ScrollDelta, line: f32, cx: &mut Context<Self>) {
        let Some(shared) = self.session().map(|session| session.screen.clone()) else {
            return;
        };
        let by = match delta {
            ScrollDelta::Pixels(pixels) => f32::from(pixels.y),
            ScrollDelta::Lines(lines) if !cx.reduce_motion() => {
                self.glide.wheel(lines.y * line, Instant::now());
                cx.notify();
                return;
            }
            ScrollDelta::Lines(lines) => lines.y * line,
        };
        let mut screen = shared.lock().unwrap();
        let grid = screen.term.grid();
        let (lines, history) = (grid.display_offset(), grid.history_size());
        let shift = self.shift();
        let to = self.glide.drag(lines, by, history, line);
        if to != lines {
            screen
                .term
                .scroll_display(ViewScroll::Delta(to as i32 - lines as i32));
        }
        // Pushed on past either end, nothing to draw anew.
        if to != lines || self.shift() != shift {
            cx.notify();
        }
    }

    fn metrics(&mut self, window: &mut Window) -> Metrics {
        if let Some(metrics) = self.metrics {
            return metrics;
        }
        let text = window.text_system();
        let font_id = text.resolve_font(&self.font);
        let cell_width = text
            .advance(font_id, self.font_size, 'm')
            .map_or(f32::from(self.font_size) * 0.6, |advance| {
                f32::from(advance.width)
            });
        let metrics = Metrics {
            cell_width,
            line_height: (f32::from(self.font_size) * self.line_height_factor).round(),
        };
        self.metrics = Some(metrics);
        metrics
    }

    /// Sizes the PTY to the pane and copies what is visible out of the locked grid.
    fn build(
        &mut self,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Frame {
        let started = Instant::now();
        // This frame shows whatever output came; the next output asks for another.
        self.frame_pending = false;
        let metrics = self.metrics(window);
        self.origin = bounds.origin;
        self.scale = window.scale_factor();
        self.bounds = bounds;
        let size = grid::grid_size(
            f32::from(bounds.size.width),
            f32::from(bounds.size.height),
            metrics,
        );
        let resized = (size.rows, size.cols) != (self.size.rows, self.size.cols);
        if resized {
            self.size = size;
            if let Some(session) = self.session_mut() {
                let _ = session.resize(size);
            }
        }
        let focused = self.focus.is_focused(window) && window.is_window_active();
        // ⌘ let go over a pane without the keyboard, which is not told.
        if !window.modifiers().platform && !self.mouse.pressed() && self.pointer.is_some() {
            self.pointer = None;
            self.hover.forget();
        }
        let mut frame = Frame {
            metrics,
            origin: bounds.origin,
            theme: *self.theme.terminal(),
            note_color: self.theme.fg(|t| t.danger),
            font: self.font.clone(),
            font_size: self.font_size,
            rows: Vec::new(),
            cursor: None,
            marked: self.ime.marked().map(str::to_owned),
            focused,
            note: (!self.note.is_empty()).then(|| self.note.clone()),
            tip: None,
            shift: 0.0,
            above: None,
        };
        let Some(session) = self.session() else {
            // Without a session, the viewer's own message comes first: a hint or why the last one
            // ended, in the muted colour unless it failed.
            if !self.viewer.note.is_empty() && self.quiet.is_none() {
                frame.note = Some(self.viewer.note.clone());
                if self.viewer.state() != "failed" {
                    frame.note_color = self.theme.fg(|t| t.muted);
                }
            }
            return frame;
        };
        // Held apart from `self`, so the link can be shown while the grid is read.
        let shared = session.screen.clone();
        let mut screen = shared.lock().unwrap();
        // A selection dragged past the top or bottom goes on through the history (P5-143d2), its
        // end on the row under the mouse; not once let go, nor in a full-screen program.
        if self.edge.is_some()
            && (!self.mouse.selecting() || screen.term.mode().contains(TermMode::ALT_SCREEN))
        {
            self.edge = None;
        }
        if let Some((edge, position)) = &mut self.edge {
            let grid = screen.term.grid();
            let (lines, history) = (grid.display_offset(), grid.history_size());
            let (to, going) = edge.frame(
                &mut self.glide,
                self.smooth_scroll,
                lines,
                history,
                metrics.line_height,
                Instant::now(),
            );
            let position = *position;
            if to != lines {
                screen
                    .term
                    .scroll_display(ViewScroll::Delta(to as i32 - lines as i32));
            }
            if going {
                window.request_animation_frame();
            } else {
                self.edge = None;
            }
            if let Some((.., point, side)) = self.cell_at(position, to)
                && let Some(selection) = &mut screen.term.selection
            {
                selection.update(point, side);
            }
        }
        // A wheel's slide goes on, frame by frame, until it is there.
        let grid = screen.term.grid();
        let (lines, history) = (grid.display_offset(), grid.history_size());
        let (to, sliding) = self
            .glide
            .frame(lines, history, metrics.line_height, Instant::now());
        if to != lines {
            screen
                .term
                .scroll_display(ViewScroll::Delta(to as i32 - lines as i32));
        }
        if sliding {
            window.request_animation_frame();
        }
        // What changed is drawn now.
        let redrawn = damage::take(&mut screen.term) == damage::Change::Whole || self.redrawn;
        self.redrawn = false;
        let term = &screen.term;
        let theme = *self.theme.terminal();
        let offset = term.grid().display_offset() as i32;
        let selection = term.selection.as_ref().and_then(|s| s.to_range(term));
        let rows = size.rows.min(term.screen_lines() as u16);
        let cols = size.cols.min(term.columns() as u16);
        // Looked for again in the grid, which output may have changed; the files are not looked at
        // here (`look_at_files`).
        let place = self.place();
        let found = self
            .pointer
            .and_then(|position| self.cell_at(position, offset as usize))
            .and_then(|(.., point, _)| links::find(term, point, place.as_ref()));
        let link = self.hover.show(found);
        frame.tip = link.as_ref().and_then(|link| tip(link, offset, rows));
        // Part of a line down, the row above the first shows over them.
        frame.shift = self.shift();
        let first = if frame.shift > 0.0 { -1 } else { 0 };
        for y in first..i32::from(rows) {
            let mut cells = rows::read(term, y, cols, &theme, selection.as_ref());
            if let Some(link) = &link {
                let line = Line(y - offset);
                for (x, cell) in cells.iter_mut().enumerate() {
                    if link.contains(GridPoint::new(line, Column(x))) {
                        cell.style.underline = true;
                    }
                }
            }
            let row = rows::split(&cells, theme.background);
            if y < 0 {
                frame.above = Some(row);
            } else {
                frame.rows.push(row);
            }
        }
        let point = term.grid().cursor.point;
        let style = term.cursor_style();
        if offset == 0
            && term.mode().contains(TermMode::SHOW_CURSOR)
            && style.shape != CursorShape::Hidden
            && point.line.0 >= 0
            && (point.line.0 as u16) < rows
            && (point.column.0 as u16) < cols
        {
            let (col, row) = (point.column.0 as u16, point.line.0 as u16);
            let cell = &term.grid()[point];
            let wide = cell.flags.contains(Flags::WIDE_CHAR);
            frame.cursor = Some(Cursor {
                col,
                row,
                shape: style.shape,
                text: (cell.c != ' ').then(|| cell.c.to_string()),
                wide,
                color: screen.cursor_color(),
                alpha: 1.0,
                corners: None,
            });
        }
        let blinks = screen.cursor_blinks();
        // The input method's candidate window follows the cursor even when it is hidden.
        let cursor_cell = (point.column.0 as u16, point.line.0.max(0) as u16);
        drop(screen);
        self.cursor_cell = cursor_cell;
        let composing = frame.marked.is_some();
        let conditions = caret::Conditions {
            focused,
            blink: self.blink_cursor && blinks && !composing,
            slide: self.animate_cursor && !cx.reduce_motion(),
            snap: redrawn || resized || composing,
        };
        self.cursor_motion(&mut frame, conditions, window, cx);
        // The pointer follows on the next frame.
        self.link = link;
        self.look_at_files(cx);
        if let Some(stats) = &self.stats {
            let mut stats = stats.borrow_mut();
            let spent = started.elapsed();
            stats.build += spent;
            stats.build_max = stats.build_max.max(spent);
        }
        frame
    }

    /// How the cursor blinks or slides this frame (`caret`), and when the pane is drawn again for
    /// it: the next frame while it slides, a timer while it blinks, not at all when it is still.
    fn cursor_motion(
        &mut self,
        frame: &mut Frame,
        conditions: caret::Conditions,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let now = Instant::now();
        let target = frame.cursor.as_ref().map(|cursor| caret::Target {
            cell: (cursor.col, cursor.row),
            rect: frame.cursor_rect(cursor),
        });
        let look = self.caret.frame(target, conditions, now);
        if let Some(cursor) = &mut frame.cursor {
            cursor.alpha = look.alpha;
            cursor.corners = look.corners;
        }
        self.caret_wake = match look.next {
            caret::Next::Idle => None,
            caret::Next::Frame => {
                window.request_animation_frame();
                None
            }
            caret::Next::At(when) => {
                let wait = when.saturating_duration_since(now);
                Some(cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(wait).await;
                    let _ = this.update(cx, |_, cx| cx.notify());
                }))
            }
        };
    }
}

/// Where the address of a link the program marked (OSC 8) shows, and what it says: on the row
/// under the link, or over it at the bottom of the view, from the link's column; none for other
/// links or one out of view.
fn tip(link: &Link, offset: i32, rows: u16) -> Option<(usize, usize, String)> {
    let uri = link.uri.as_ref()?;
    let rows = i32::from(rows);
    let (first, last) = (link.start.line.0 + offset, link.end.line.0 + offset);
    if last < 0 || first >= rows {
        return None;
    }
    let row = if last + 1 < rows {
        last + 1
    } else {
        (first - 1).max(0)
    };
    let col = if first == last {
        link.start.column.0
    } else {
        0
    };
    const LONGEST: usize = 120;
    let mut text = format!("Opens {uri}");
    if let Some((cut, _)) = text.char_indices().nth(LONGEST) {
        text.truncate(cut);
        text.push('…');
    }
    Some((row as usize, col, text))
}

/// The pane's attach to its agent has ended: the agent may have just ended too.
pub struct AttachEnded;

impl EventEmitter<AttachEnded> for TerminalView {}

/// A link ⌘-clicked in the pane, for the window to open (P5-143b).
pub struct OpenLink(pub links::Target);

impl EventEmitter<OpenLink> for TerminalView {}

impl Focusable for TerminalView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

struct Cursor {
    col: u16,
    row: u16,
    shape: CursorShape,
    text: Option<String>,
    wide: bool,
    /// What the program set (OSC 12), or else the theme's.
    color: Rgb,
    /// How opaque, as it blinks.
    alpha: f32,
    /// Mid-slide, its corners in the grid (`caret`).
    corners: Option<[caret::Corner; 4]>,
}

/// Everything one paint needs, copied so the grid lock is not held while drawing.
pub struct Frame {
    metrics: Metrics,
    origin: Point<Pixels>,
    theme: Theme,
    note_color: Rgb,
    font: Font,
    font_size: Pixels,
    rows: Vec<(Vec<Span>, Vec<Run>)>,
    cursor: Option<Cursor>,
    marked: Option<String>,
    focused: bool,
    note: Option<String>,
    /// Where a link the program marked really goes, shown at a row and column by it.
    tip: Option<(usize, usize, String)>,
    /// How far down the rows are drawn, under a line, the view scrolled by the pixel (P5-143c2).
    shift: f32,
    /// Then, the row above the first, showing over them.
    above: Option<(Vec<Span>, Vec<Run>)>,
}

fn terminal_font(family: &str, fallbacks: &[String]) -> Font {
    let mut font = gpui::font(family.to_owned());
    font.weight = FontWeight::NORMAL;
    if !fallbacks.is_empty() {
        font.fallbacks = Some(gpui::FontFallbacks::from_fonts(fallbacks.to_vec()));
    }
    font
}

pub fn hsla((r, g, b): Rgb, alpha: f32) -> Hsla {
    Rgba {
        r: f32::from(r) / 255.0,
        g: f32::from(g) / 255.0,
        b: f32::from(b) / 255.0,
        a: alpha,
    }
    .into()
}

impl Frame {
    /// Column edges on whole pixels, so neighbouring cells share an edge.
    fn x(&self, col: usize) -> f32 {
        (col as f32 * self.metrics.cell_width).floor()
    }
    /// A row's top, -1 for the one above the first; as if the view were on its whole line.
    fn y(&self, row: i32) -> f32 {
        row as f32 * self.metrics.line_height
    }
    /// Where a point of the grid is drawn: `shift` down.
    fn at(&self, x: f32, y: f32) -> Point<Pixels> {
        point(self.origin.x + px(x), self.origin.y + px(y + self.shift))
    }
    fn rect(&self, x: f32, y: f32, w: f32, h: f32, color: Hsla) -> gpui::PaintQuad {
        fill(Bounds::new(self.at(x, y), size(px(w), px(h))), color)
    }

    fn font(&self, style: &Style) -> Font {
        let mut font = self.font.clone();
        if style.bold {
            font.weight = FontWeight::BOLD;
        }
        if style.italic {
            font.style = FontStyle::Italic;
        }
        font
    }

    fn shape(
        &self,
        text: &str,
        style: &Style,
        color: Rgb,
        alpha: f32,
        cell: Option<f32>,
        window: &Window,
    ) -> gpui::ShapedLine {
        let run = TextRun {
            len: text.len(),
            font: self.font(style),
            color: hsla(color, alpha),
            background_color: None,
            underline: style.underline.then(|| UnderlineStyle {
                thickness: px(1.0),
                color: Some(hsla(color, alpha)),
                wavy: false,
            }),
            // Drawn per cell in `text`: GPUI skipped it on lone wide glyphs.
            strikethrough: None,
        };
        window.text_system().shape_line(
            SharedString::from(text.to_owned()),
            self.font_size,
            &[run],
            cell.map(px),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn text(
        &self,
        text: &str,
        style: &Style,
        (col, row): (usize, i32),
        cell_wide: bool,
        alpha: f32,
        window: &mut Window,
        cx: &mut App,
    ) {
        let color = style.fg;
        let line_height = px(self.metrics.line_height);
        let (x, y) = (self.x(col), self.y(row));
        if cell_wide {
            // Wide glyphs keep their own advance, centred in their two cells.
            let shaped = self.shape(text, style, color, alpha, None, window);
            let room = self.x(col + 2) - x;
            let inset = ((room - f32::from(shaped.width)) / 2.0).max(0.0);
            let _ = shaped.paint(
                self.at(x + inset, y),
                line_height,
                TextAlign::Left,
                None,
                window,
                cx,
            );
            self.strike(style, color, x, y, room, window);
        } else {
            let shaped = self.shape(
                text,
                style,
                color,
                alpha,
                Some(self.metrics.cell_width),
                window,
            );
            let _ = shaped.paint(
                self.at(x, y),
                line_height,
                TextAlign::Left,
                None,
                window,
                cx,
            );
            self.strike(style, color, x, y, f32::from(shaped.width).ceil(), window);
        }
    }

    fn strike(&self, style: &Style, color: Rgb, x: f32, y: f32, width: f32, window: &mut Window) {
        if style.strike {
            let middle = (y + self.metrics.line_height / 2.0).round();
            window.paint_quad(self.rect(x, middle, width, 1.0, hsla(color, 1.0)));
        }
    }

    fn glyph(&self, c: char, color: Rgb, col: usize, row: i32, window: &mut Window) {
        let (x, y) = (self.x(col), self.y(row));
        let w = self.x(col + 1) - x;
        // Rows are already whole pixels when the line height is.
        let h = self.metrics.line_height;
        for piece in glyphs::pieces(c, w, h).unwrap_or_default() {
            window.paint_quad(self.rect(
                x + piece.x,
                y + piece.y,
                piece.w,
                piece.h,
                hsla(color, piece.alpha),
            ));
        }
    }

    /// The rows drawn, by row: the one above the first, when it shows, as -1.
    fn lines(&self) -> impl Iterator<Item = (i32, &(Vec<Span>, Vec<Run>))> {
        let rows = self.rows.iter().enumerate();
        let above = self.above.iter().map(|row| (-1, row));
        above.chain(rows.map(|(row, line)| (row as i32, line)))
    }

    fn paint(&self, width: f32, window: &mut Window, cx: &mut App) {
        let lh = self.metrics.line_height;
        // Part of a line down, what is past the grid's edges is cut off.
        let mask = self.above.is_some().then(|| ContentMask {
            bounds: Bounds::new(
                self.origin,
                size(px(width), px(self.rows.len() as f32 * lh)),
            ),
        });
        window.with_content_mask(mask, |window| self.paint_grid(width, window, cx));
        if let Some(note) = &self.note {
            let style = Style {
                fg: self.note_color,
                ..Style::default()
            };
            let lines = note_lines(note, width, |text| {
                self.shape(text, &style, style.fg, 1.0, None, window)
                    .width
                    .into()
            });
            for (row, line) in lines.into_iter().enumerate() {
                let shaped = self.shape(line, &style, style.fg, 1.0, None, window);
                let y = self.origin.y + px(self.y((self.rows.len().max(1) + row) as i32));
                let at = point(self.origin.x, y);
                let _ = shaped.paint(at, px(lh), TextAlign::Left, None, window, cx);
            }
        }
    }

    fn paint_grid(&self, width: f32, window: &mut Window, cx: &mut App) {
        let lh = self.metrics.line_height;
        for (row, (spans, _)) in self.lines() {
            for span in spans {
                let (x0, x1) = (self.x(span.col), self.x(span.col + span.len));
                window.paint_quad(self.rect(x0, self.y(row), x1 - x0, lh, hsla(span.color, 1.0)));
            }
        }
        // Layered: every drawn line, then every text run, so GPUI batches them together.
        for (row, (_, runs)) in self.lines() {
            for run in runs {
                if let Run::Glyph { col, c, color } = run {
                    self.glyph(*c, *color, *col, row, window);
                }
            }
        }
        for (row, (_, runs)) in self.lines() {
            for run in runs {
                match run {
                    Run::Text { col, text, style } => {
                        self.text(text, style, (*col, row), false, 1.0, window, cx)
                    }
                    Run::Wide { col, text, style } => {
                        self.text(text, style, (*col, row), true, 1.0, window, cx)
                    }
                    Run::Glyph { .. } => {}
                }
            }
        }
        self.paint_cursor(window, cx);
        self.paint_tip(width, window, cx);
    }

    /// A marked link's address, dark on light, kept inside the pane.
    fn paint_tip(&self, width: f32, window: &mut Window, cx: &mut App) {
        let Some((row, col, text)) = &self.tip else {
            return;
        };
        let style = Style {
            fg: self.theme.background,
            ..Style::default()
        };
        let shaped = self.shape(text, &style, style.fg, 1.0, None, window);
        let (pad, lh) = (4.0, self.metrics.line_height);
        let w = f32::from(shaped.width).ceil() + 2.0 * pad;
        let x = self.x(*col).min(width - w).max(0.0);
        let y = self.y(*row as i32);
        window.paint_quad(
            self.rect(x, y, w, lh, hsla(self.theme.foreground, 0.92))
                .corner_radii(px(3.0)),
        );
        let _ = shaped.paint(
            self.at(x + pad, y),
            px(lh),
            TextAlign::Left,
            None,
            window,
            cx,
        );
    }

    /// Where `cursor` is drawn in its place, by its shape.
    fn cursor_rect(&self, cursor: &Cursor) -> caret::Rect {
        let (col, row) = (usize::from(cursor.col), i32::from(cursor.row));
        let (x, y) = (self.x(col), self.y(row));
        let w = self.x(col + if cursor.wide { 2 } else { 1 }) - x;
        let lh = self.metrics.line_height;
        let (y, w, h) = match cursor.shape {
            CursorShape::Beam => (y, 2.0, lh),
            CursorShape::Underline => (y + lh - 2.0, w, 2.0),
            _ => (y, w, lh),
        };
        caret::Rect { x, y, w, h }
    }

    fn paint_cursor(&self, window: &mut Window, cx: &mut App) {
        let Some(cursor) = &self.cursor else {
            return;
        };
        let (col, row) = (usize::from(cursor.col), i32::from(cursor.row));
        let (x, y) = (self.x(col), self.y(row));
        let w = self.x(col + if cursor.wide { 2 } else { 1 }) - x;
        let lh = self.metrics.line_height;
        let color = hsla(cursor.color, cursor.alpha);
        if let Some(corners) = cursor.corners {
            // On its way: the four corners, the leading ones ahead.
            let mut path = PathBuilder::fill();
            path.add_polygon(&corners.map(|(x, y)| self.at(x, y)), true);
            if let Ok(path) = path.build() {
                window.paint_path(path, color);
            }
            return;
        }
        if cursor.alpha <= 0.0 {
            return;
        }
        if let Some(marked) = &self.marked {
            // The composition replaces the cursor: drawn over the grid, underlined.
            let style = Style {
                fg: self.theme.foreground,
                underline: true,
                ..Style::default()
            };
            let shaped = self.shape(marked, &style, style.fg, 1.0, None, window);
            let width = f32::from(shaped.width).ceil();
            window.paint_quad(self.rect(x, y, width, lh, hsla(self.theme.background, 1.0)));
            let _ = shaped.paint(self.at(x, y), px(lh), TextAlign::Left, None, window, cx);
            window.paint_quad(self.rect(x + width, y, 2.0, lh, color));
            return;
        }
        match (cursor.shape, self.focused) {
            (CursorShape::Beam, _) => window.paint_quad(self.rect(x, y, 2.0, lh, color)),
            (CursorShape::Underline, _) => {
                window.paint_quad(self.rect(x, y + lh - 2.0, w, 2.0, color))
            }
            (_, true) => {
                window.paint_quad(self.rect(x, y, w, lh, color));
                if let Some(text) = &cursor.text {
                    let style = Style {
                        fg: self.theme.background,
                        ..Style::default()
                    };
                    self.text(
                        text,
                        &style,
                        (col, row),
                        cursor.wide,
                        cursor.alpha,
                        window,
                        cx,
                    );
                }
            }
            (_, false) => {
                for (rx, ry, rw, rh) in [
                    (x, y, w, 1.0),
                    (x, y + lh - 1.0, w, 1.0),
                    (x, y, 1.0, lh),
                    (x + w - 1.0, y, 1.0, lh),
                ] {
                    window.paint_quad(self.rect(rx, ry, rw, rh, color));
                }
            }
        }
    }
}

impl Render for TerminalView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // With several panes, the window is titled after the one with the keyboard.
        if !self.title.is_empty() && self.focus.is_focused(window) {
            window.set_window_title(&self.title);
        }
        let entity = cx.entity();
        let input = entity.clone();
        let leave = entity.downgrade();
        let focus = self.focus.clone();
        let stats = self.stats.clone();
        let drop_highlight = hsla(self.theme.fg(|t| t.focus), 0.08);
        // Only a shown pane is drawn: its notice's time starts here, and a repaint then ends it.
        let mut first = false;
        let notice = self.notice.as_mut().and_then(|notice| {
            first = !notice.seen();
            notice.drawn(Instant::now()).map(str::to_owned)
        });
        if notice.is_none() {
            self.notice = None;
        } else if first {
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(notice::NOTICE).await;
                let _ = this.update(cx, |_, cx| cx.notify());
            })
            .detach();
        }
        div()
            .relative()
            .size_full()
            .bg(hsla(self.theme.terminal().background, 1.0))
            .p(px(6.0))
            .track_focus(&self.focus)
            .key_context("Terminal")
            .on_key_down(cx.listener(Self::key_down))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::paste_clipboard))
            .on_action(cx.listener(Self::open_find))
            .on_action(cx.listener(Self::find_next))
            .on_action(cx.listener(Self::find_previous))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::mouse_up))
            .on_mouse_move(cx.listener(Self::mouse_move))
            .on_modifiers_changed(cx.listener(Self::modifiers_changed))
            .on_scroll_wheel(cx.listener(Self::scroll_wheel))
            .when(self.link.is_some(), |pane| {
                pane.cursor(CursorStyle::PointingHand)
            })
            .child(
                canvas(
                    move |bounds, window, cx| {
                        entity.update(cx, |view, cx| view.build(bounds, window, cx))
                    },
                    move |bounds, frame: Frame, window, cx| {
                        let started = Instant::now();
                        window.handle_input(&focus, ElementInputHandler::new(bounds, input), cx);
                        // Anywhere in the window: off the grid a link under the mouse is no
                        // longer, and a press on a link is followed out of the pane.
                        let pane = leave.clone();
                        window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                            if phase == DispatchPhase::Bubble {
                                let _ = pane.update(cx, |view, cx| view.window_move(event, cx));
                            }
                        });
                        let pane = leave.clone();
                        window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
                            if phase == DispatchPhase::Bubble && event.button == MouseButton::Left {
                                let _ = pane.update(cx, |view, cx| view.window_up(event, cx));
                            }
                        });
                        frame.paint(bounds.size.width.into(), window, cx);
                        if let Some(stats) = stats {
                            stats.borrow_mut().record_paint(started.elapsed());
                        }
                    },
                )
                .size_full(),
            )
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .drag_over::<ExternalPaths>(move |style, _, _, _| style.bg(drop_highlight)),
            )
            .children(self.find.as_ref().map(|bar| self.find_bar(bar, cx)))
            .children(notice.map(|text| self.notice_bar(text, cx)))
    }
}

impl Stats {
    fn record_paint(&mut self, spent: Duration) {
        let now = Instant::now();
        let since = *self.since.get_or_insert(now);
        self.frames += 1;
        self.paint += spent;
        self.paint_max = self.paint_max.max(spent);
        let elapsed = now - since;
        if elapsed >= Duration::from_secs(1) {
            let frames = self.frames.max(1);
            eprintln!(
                "stats: {:.1} frames/s, build avg {:.2} ms max {:.2} ms, paint avg {:.2} ms max {:.2} ms",
                f64::from(self.frames) / elapsed.as_secs_f64(),
                self.build.as_secs_f64() * 1000.0 / f64::from(frames),
                self.build_max.as_secs_f64() * 1000.0,
                self.paint.as_secs_f64() * 1000.0 / f64::from(frames),
                self.paint_max.as_secs_f64() * 1000.0,
            );
            *self = Stats::default();
        }
    }
}

impl EntityInputHandler for TerminalView {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        _adjusted: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        self.ime.text_for_range(range)
    }

    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.ime.selected_range(),
            reversed: false,
        })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.ime.marked_range()
    }

    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.ime.clear();
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        _: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.typed(text, cx);
        let bytes = self.ime.commit(text);
        self.write(bytes, cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        _: Option<Range<usize>>,
        text: &str,
        _: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.typed(text, cx);
        self.ime.mark(text);
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        _: Range<usize>,
        _element_bounds: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let metrics = self.metrics?;
        let (col, row) = self.cursor_cell;
        let origin = point(
            self.origin.x + px((f32::from(col) * metrics.cell_width).floor()),
            self.origin.y + px(f32::from(row) * metrics.line_height + self.shift()),
        );
        Some(Bounds::new(
            origin,
            size(px(metrics.cell_width), px(metrics.line_height)),
        ))
    }

    fn character_index_for_point(
        &mut self,
        _: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        None
    }
}

/// For the command palette's text search (`search.rs`), which looks through every open pane.
impl TerminalView {
    /// The screen the pane draws, to search off the UI thread; none without a session.
    pub fn screen(&self) -> Option<std::sync::Arc<std::sync::Mutex<crate::terminal::Screen>>> {
        self.session().map(|session| session.screen.clone())
    }

    /// A match chosen in the palette, selected and scrolled into view, with the find bar open on
    /// `query` so ⌘G and ⌘⇧G go on from it.
    pub fn show_match(
        &mut self,
        query: &str,
        hit: &crate::find::Hit,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_find(&menu::Find, window, cx);
        if let Some(bar) = &self.find {
            let query = query.to_owned();
            bar.input.update(cx, |input, cx| input.set_text(query, cx));
        }
        let found = match self.session() {
            Some(session) => find::reveal(&mut session.screen.lock().unwrap().term, query, hit),
            None => false,
        };
        if found {
            self.glide.stop();
        }
        if let Some(bar) = &mut self.find {
            bar.missed = !found;
        }
        cx.notify();
    }
}
