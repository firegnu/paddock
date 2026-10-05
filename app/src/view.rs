//! The terminal pane: a Saddle PTY session drawn and driven by GPUI.
//!
//! Saddle's `Viewer` owns the shell / `corral attach` lifecycle and `Session` the PTY and parser,
//! unchanged. This file only reads the parser's grid, draws it, and turns window input into the
//! bytes Saddle's `input` module already encodes.
use crate::{
    damage, glyphs,
    grid::{self, Metrics, Scroll},
    ime::Composition,
    keys,
    palette::{Rgb, Theme},
    rows::{self, Run, Span, Style},
    theme,
};
use alacritty_terminal::{
    grid::{Dimensions, Scroll as ViewScroll},
    index::{Column, Line, Point as GridPoint, Side},
    selection::{Selection, SelectionType},
    term::{TermMode, cell::Flags},
    vte::ansi::CursorShape,
};
use gpui::{
    App, Bounds, ClipboardItem, Context, ElementInputHandler, EntityInputHandler, FocusHandle,
    Focusable, Font, FontStyle, FontWeight, Hsla, KeyDownEvent, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, Pixels, Point, Render, Rgba, ScrollDelta, ScrollWheelEvent,
    SharedString, TextAlign, TextRun, UTF16Selection, UnderlineStyle, Window, canvas, div, fill,
    point, prelude::*, px, size,
};
use saddle::{
    pty::Session,
    terminal::Size,
    viewer::{AgentMetadata, Shell, Viewer},
};
use std::{cell::RefCell, ops::Range, path::PathBuf, rc::Rc, time::Duration, time::Instant};

/// What runs in the pane.
pub enum Launch {
    /// The user's shell, as Saddle starts plain terminals.
    Shell { program: String, cwd: String },
    /// `corral attach NAME` through Saddle's viewer.
    Agent { name: String },
    /// Any program, straight on a PTY (synthetic output tests).
    Command {
        argv: Vec<String>,
        cwd: Option<PathBuf>,
    },
}

pub struct Options {
    pub launch: Launch,
    /// The `corral` program the pane attaches with.
    pub corral: String,
    pub font_family: String,
    /// Families tried for glyphs the main font lacks (prompt icons, symbols).
    pub fallbacks: Vec<String>,
    pub font_size: f32,
    pub line_height: f32,
    pub stats: bool,
}

pub struct TerminalView {
    viewer: Viewer,
    /// Set only for `Launch::Command`; the viewer stays idle then.
    direct: Option<Session>,
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
    theme: Rc<theme::Theme>,
    ime: Composition,
    scroll: Scroll,
    /// A left press that started a selection, or one reported to the program.
    pressed: Option<Press>,
    /// Cursor cell of the last frame, for the input method's candidate window.
    cursor_cell: (u16, u16),
    note: String,
    title: String,
    stats: Option<Rc<RefCell<Stats>>>,
}

#[derive(Clone, Copy, PartialEq)]
enum Press {
    Select,
    Reported,
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
        let mut viewer = Viewer::new(options.corral);
        let mut direct = None;
        let mut note = String::new();
        let (label, subject) = match options.launch {
            Launch::Shell { program, cwd } => {
                viewer.start_shell(Shell {
                    program: program.clone(),
                    cwd: cwd.clone(),
                    state: "starting",
                    exit_code: None,
                    env: Vec::new(),
                });
                (format!("{program} · {cwd}"), format!("shell · {cwd}"))
            }
            Launch::Agent { name } => {
                if let Err(error) = viewer.select_agent(name.clone(), AgentMetadata::default()) {
                    note = format!("{error:#}");
                }
                (format!("corral attach {name}"), name)
            }
            Launch::Command { argv, cwd } => {
                match Session::spawn(&argv, cwd.as_deref(), size) {
                    Ok(session) => direct = Some(session),
                    Err(error) => note = format!("{error:#}"),
                }
                let command = argv.join(" ");
                (command.clone(), command)
            }
        };
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        let mut font = gpui::font(options.font_family);
        font.weight = FontWeight::NORMAL;
        if !options.fallbacks.is_empty() {
            font.fallbacks = Some(gpui::FontFallbacks::from_fonts(options.fallbacks));
        }
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(8))
                    .await;
                if this.update(cx, |view, cx| view.poll(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        Self {
            viewer,
            direct,
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
            theme,
            ime: Composition::default(),
            scroll: Scroll::default(),
            pressed: None,
            cursor_cell: (0, 0),
            note,
            title: String::new(),
            stats: options
                .stats
                .then(|| Rc::new(RefCell::new(Stats::default()))),
        }
    }

    /// What the pane is connected to, for the title bar: `shell · <cwd>`, the agent, or the command.
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// The agent the pane shows or is attaching to.
    pub fn target(&self) -> Option<&str> {
        self.viewer.target()
    }

    /// Attaches the pane to an agent; whatever ran before ends, a shell included.
    pub fn attach(&mut self, name: String, metadata: AgentMetadata, cx: &mut Context<Self>) {
        self.retire_direct();
        self.queued_shell = None;
        self.viewer.shell = None;
        if let Err(error) = self.viewer.select_agent(name.clone(), metadata) {
            self.note = format!("{error:#}");
        }
        self.label = format!("corral attach {name}");
        self.subject = name;
        cx.notify();
    }

    /// Ends what the pane runs and starts an interactive shell once it is gone.
    pub fn start_shell(&mut self, program: String, cwd: String, cx: &mut Context<Self>) {
        self.retire_direct();
        if let Err(error) = self.viewer.close() {
            self.note = format!("{error:#}");
        }
        self.label = format!("{program} · {cwd}");
        self.subject = format!("shell · {cwd}");
        self.queued_shell = Some(Shell {
            program,
            cwd,
            state: "starting",
            exit_code: None,
            env: Vec::new(),
        });
        cx.notify();
    }

    /// Agents still listed by corral; an attached agent missing from them is let go.
    pub fn disappeared(&mut self, names: &[&str]) {
        if let Err(error) = self.viewer.disappeared(names) {
            self.note = format!("{error:#}");
        }
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

    /// Drives the session the way Saddle's main loop does, and repaints when output arrived.
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
        }
        let changed = self
            .session()
            .is_some_and(|s| damage::take_changed(&mut s.screen.lock().unwrap().term));
        let title = format!("{} — {}", self.label, self.state());
        if changed || title != self.title {
            self.title = title;
            cx.notify();
        }
    }

    fn mode(&self) -> TermMode {
        self.session()
            .map_or(TermMode::empty(), |s| *s.screen.lock().unwrap().term.mode())
    }

    /// Typed input: back to the live bottom, selection cleared, bytes to the PTY.
    fn write(&mut self, bytes: Vec<u8>, cx: &mut Context<Self>) {
        if bytes.is_empty() {
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
        if let Err(error) = session.send(bytes) {
            self.note = format!("{error:#}");
        }
        cx.notify();
    }

    fn paste(&mut self, text: &str, cx: &mut Context<Self>) {
        let bracketed = self.mode().contains(TermMode::BRACKETED_PASTE);
        // Without bracketed paste a newline must arrive as Enter.
        let text = if bracketed {
            text.to_owned()
        } else {
            text.replace("\r\n", "\r").replace('\n', "\r")
        };
        self.write(saddle::input::encode_paste(&text, bracketed), cx);
    }

    fn selection_text(&self) -> Option<String> {
        let screen = self.session()?.screen.lock().unwrap();
        screen.term.selection_to_string().filter(|s| !s.is_empty())
    }

    fn key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform {
            match keystroke.key.as_str() {
                "c" => {
                    if let Some(text) = self.selection_text() {
                        cx.write_to_clipboard(ClipboardItem::new_string(text));
                        cx.stop_propagation();
                    }
                }
                "v" => {
                    if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                        self.paste(&text, cx);
                    }
                    cx.stop_propagation();
                }
                "q" => cx.quit(),
                _ => {}
            }
            return;
        }
        let application_cursor = self.mode().contains(TermMode::APP_CURSOR);
        if let Some(bytes) = keys::key_bytes(keystroke, application_cursor) {
            self.write(bytes, cx);
            cx.stop_propagation();
        }
    }

    /// The grid cell under a window position, and the grid point counting the scrolled view.
    fn locate(&self, position: Point<Pixels>) -> Option<(u16, u16, GridPoint, Side)> {
        let metrics = self.metrics?;
        let (col, row, right) = grid::cell_at(
            f32::from(position.x - self.origin.x),
            f32::from(position.y - self.origin.y),
            metrics,
            self.size,
        );
        let offset = self
            .session()?
            .screen
            .lock()
            .unwrap()
            .term
            .grid()
            .display_offset();
        let point = GridPoint::new(
            Line(i32::from(row) - offset as i32),
            Column(usize::from(col)),
        );
        Some((
            col,
            row,
            point,
            if right { Side::Right } else { Side::Left },
        ))
    }

    /// Reports a mouse event through Saddle's encoder when the program asked for the mouse.
    fn report_mouse(
        &mut self,
        kind: crossterm::event::MouseEventKind,
        position: Point<Pixels>,
        modifiers: gpui::Modifiers,
        cx: &mut Context<Self>,
    ) -> bool {
        let mode = self.mode();
        if !mode.intersects(TermMode::MOUSE_MODE) {
            return false;
        }
        let Some((col, row, ..)) = self.locate(position) else {
            return false;
        };
        let mut mods = crossterm::event::KeyModifiers::NONE;
        mods.set(crossterm::event::KeyModifiers::SHIFT, modifiers.shift);
        mods.set(crossterm::event::KeyModifiers::ALT, modifiers.alt);
        mods.set(crossterm::event::KeyModifiers::CONTROL, modifiers.control);
        let event = crossterm::event::MouseEvent {
            kind,
            column: col,
            row,
            modifiers: mods,
        };
        let area = ratatui::layout::Rect::new(0, 0, self.size.cols, self.size.rows);
        let bytes = saddle::input::encode_mouse(event, area, mode);
        if let Some(session) = self.session()
            && !bytes.is_empty()
        {
            let _ = session.send(bytes);
        }
        cx.notify();
        true
    }

    fn mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus, cx);
        use crossterm::event::{MouseButton as B, MouseEventKind as K};
        if !event.modifiers.shift
            && self.report_mouse(K::Down(B::Left), event.position, event.modifiers, cx)
        {
            self.pressed = Some(Press::Reported);
            return;
        }
        let Some((.., point, side)) = self.locate(event.position) else {
            return;
        };
        let kind = match event.click_count {
            2 => SelectionType::Semantic,
            3.. => SelectionType::Lines,
            _ => SelectionType::Simple,
        };
        if let Some(session) = self.session() {
            session.screen.lock().unwrap().term.selection = Some(Selection::new(kind, point, side));
        }
        self.pressed = Some(Press::Select);
        cx.notify();
    }

    fn mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        use crossterm::event::{MouseButton as B, MouseEventKind as K};
        match (self.pressed, event.pressed_button) {
            (Some(Press::Reported), Some(MouseButton::Left)) => {
                self.report_mouse(K::Drag(B::Left), event.position, event.modifiers, cx);
            }
            (Some(Press::Select), Some(MouseButton::Left)) => {
                if let Some((.., point, side)) = self.locate(event.position)
                    && let Some(session) = self.session()
                {
                    let mut screen = session.screen.lock().unwrap();
                    if let Some(selection) = &mut screen.term.selection {
                        selection.update(point, side);
                    }
                    cx.notify();
                }
            }
            (None, None) if self.mode().contains(TermMode::MOUSE_MOTION) => {
                self.report_mouse(K::Moved, event.position, event.modifiers, cx);
            }
            _ => {}
        }
    }

    fn mouse_up(&mut self, event: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        use crossterm::event::{MouseButton as B, MouseEventKind as K};
        match self.pressed.take() {
            Some(Press::Reported) => {
                self.report_mouse(K::Up(B::Left), event.position, event.modifiers, cx);
            }
            Some(Press::Select) => {
                if let Some(session) = self.session() {
                    let mut screen = session.screen.lock().unwrap();
                    if screen.term.selection.as_ref().is_some_and(|s| s.is_empty()) {
                        screen.term.selection = None;
                    }
                }
                cx.notify();
            }
            None => {}
        }
    }

    fn scroll_wheel(&mut self, event: &ScrollWheelEvent, _: &mut Window, cx: &mut Context<Self>) {
        let Some(metrics) = self.metrics else {
            return;
        };
        let delta = match event.delta {
            ScrollDelta::Lines(lines) => lines.y,
            ScrollDelta::Pixels(pixels) => f32::from(pixels.y) / metrics.line_height,
        };
        let lines = self.scroll.lines(delta);
        if lines == 0 {
            return;
        }
        use crossterm::event::MouseEventKind as K;
        let mode = self.mode();
        if mode.intersects(TermMode::MOUSE_MODE) && !event.modifiers.shift {
            let kind = if lines > 0 {
                K::ScrollUp
            } else {
                K::ScrollDown
            };
            for _ in 0..lines.unsigned_abs().min(20) {
                self.report_mouse(kind, event.position, event.modifiers, cx);
            }
        } else if mode.contains(TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL) {
            // Full-screen programs without mouse reporting scroll with arrow keys.
            let arrow: &[u8] = match (lines > 0, mode.contains(TermMode::APP_CURSOR)) {
                (true, true) => b"\x1bOA",
                (true, false) => b"\x1b[A",
                (false, true) => b"\x1bOB",
                (false, false) => b"\x1b[B",
            };
            let bytes = arrow.repeat(lines.unsigned_abs().min(20) as usize);
            if let Some(session) = self.session() {
                let _ = session.send(bytes);
            }
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
    fn build(&mut self, bounds: Bounds<Pixels>, window: &mut Window) -> Frame {
        let started = Instant::now();
        let metrics = self.metrics(window);
        self.origin = bounds.origin;
        let size = grid::grid_size(
            f32::from(bounds.size.width),
            f32::from(bounds.size.height),
            metrics,
        );
        if (size.rows, size.cols) != (self.size.rows, self.size.cols) {
            self.size = size;
            if let Some(session) = self.session_mut() {
                let _ = session.resize(size);
            }
        }
        let focused = self.focus.is_focused(window) && window.is_window_active();
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
        };
        let Some(session) = self.session() else {
            frame.note = Some(self.viewer.note.clone())
                .filter(|n| !n.is_empty())
                .or(frame.note);
            return frame;
        };
        let screen = session.screen.lock().unwrap();
        let term = &screen.term;
        let theme = *self.theme.terminal();
        let offset = term.grid().display_offset() as i32;
        let selection = term.selection.as_ref().and_then(|s| s.to_range(term));
        let rows = size.rows.min(term.screen_lines() as u16);
        let cols = size.cols.min(term.columns() as u16);
        for y in 0..rows {
            let cells = rows::read(term, y, cols, &theme, selection.as_ref());
            frame.rows.push(rows::split(&cells, theme.background));
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
            });
        }
        // The input method's candidate window follows the cursor even when it is hidden.
        let cursor_cell = (point.column.0 as u16, point.line.0.max(0) as u16);
        drop(screen);
        self.cursor_cell = cursor_cell;
        if let Some(stats) = &self.stats {
            let mut stats = stats.borrow_mut();
            let spent = started.elapsed();
            stats.build += spent;
            stats.build_max = stats.build_max.max(spent);
        }
        frame
    }
}

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
    fn y(&self, row: usize) -> f32 {
        row as f32 * self.metrics.line_height
    }
    fn at(&self, x: f32, y: f32) -> Point<Pixels> {
        point(self.origin.x + px(x), self.origin.y + px(y))
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
        cell: Option<f32>,
        window: &Window,
    ) -> gpui::ShapedLine {
        let run = TextRun {
            len: text.len(),
            font: self.font(style),
            color: hsla(color, 1.0),
            background_color: None,
            underline: style.underline.then(|| UnderlineStyle {
                thickness: px(1.0),
                color: Some(hsla(color, 1.0)),
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

    fn text(
        &self,
        text: &str,
        style: &Style,
        (col, row): (usize, usize),
        cell_wide: bool,
        window: &mut Window,
        cx: &mut App,
    ) {
        let color = style.fg;
        let line_height = px(self.metrics.line_height);
        let (x, y) = (self.x(col), self.y(row));
        if cell_wide {
            // Wide glyphs keep their own advance, centred in their two cells.
            let shaped = self.shape(text, style, color, None, window);
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
            let shaped = self.shape(text, style, color, Some(self.metrics.cell_width), window);
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

    fn glyph(&self, c: char, color: Rgb, col: usize, row: usize, window: &mut Window) {
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

    fn paint(&self, window: &mut Window, cx: &mut App) {
        let lh = self.metrics.line_height;
        for (row, (spans, _)) in self.rows.iter().enumerate() {
            for span in spans {
                let (x0, x1) = (self.x(span.col), self.x(span.col + span.len));
                window.paint_quad(self.rect(x0, self.y(row), x1 - x0, lh, hsla(span.color, 1.0)));
            }
        }
        // Layered: every drawn line, then every text run, so GPUI batches them together.
        for (row, (_, runs)) in self.rows.iter().enumerate() {
            for run in runs {
                if let Run::Glyph { col, c, color } = run {
                    self.glyph(*c, *color, *col, row, window);
                }
            }
        }
        for (row, (_, runs)) in self.rows.iter().enumerate() {
            for run in runs {
                match run {
                    Run::Text { col, text, style } => {
                        self.text(text, style, (*col, row), false, window, cx)
                    }
                    Run::Wide { col, text, style } => {
                        self.text(text, style, (*col, row), true, window, cx)
                    }
                    Run::Glyph { .. } => {}
                }
            }
        }
        self.paint_cursor(window, cx);
        if let Some(note) = &self.note {
            let style = Style {
                fg: self.note_color,
                ..Style::default()
            };
            let shaped = self.shape(note, &style, style.fg, None, window);
            let y = self.y(self.rows.len().max(1));
            let _ = shaped.paint(self.at(0.0, y), px(lh), TextAlign::Left, None, window, cx);
        }
    }

    fn paint_cursor(&self, window: &mut Window, cx: &mut App) {
        let Some(cursor) = &self.cursor else {
            return;
        };
        let (col, row) = (usize::from(cursor.col), usize::from(cursor.row));
        let (x, y) = (self.x(col), self.y(row));
        let w = self.x(col + if cursor.wide { 2 } else { 1 }) - x;
        let lh = self.metrics.line_height;
        let color = hsla(self.theme.cursor, 1.0);
        if let Some(marked) = &self.marked {
            // The composition replaces the cursor: drawn over the grid, underlined.
            let style = Style {
                fg: self.theme.foreground,
                underline: true,
                ..Style::default()
            };
            let shaped = self.shape(marked, &style, style.fg, None, window);
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
                    self.text(text, &style, (col, row), cursor.wide, window, cx);
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
        if !self.title.is_empty() {
            window.set_window_title(&self.title);
        }
        let entity = cx.entity();
        let input = entity.clone();
        let focus = self.focus.clone();
        let stats = self.stats.clone();
        div()
            .size_full()
            .bg(hsla(self.theme.terminal().background, 1.0))
            .p(px(6.0))
            .track_focus(&self.focus)
            .key_context("Terminal")
            .on_key_down(cx.listener(Self::key_down))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::mouse_up))
            .on_mouse_move(cx.listener(Self::mouse_move))
            .on_scroll_wheel(cx.listener(Self::scroll_wheel))
            .child(
                canvas(
                    move |bounds, window, cx| {
                        entity.update(cx, |view, _| view.build(bounds, window))
                    },
                    move |bounds, frame: Frame, window, cx| {
                        let started = Instant::now();
                        window.handle_input(&focus, ElementInputHandler::new(bounds, input), cx);
                        frame.paint(window, cx);
                        if let Some(stats) = stats {
                            stats.borrow_mut().record_paint(started.elapsed());
                        }
                    },
                )
                .size_full(),
            )
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
            self.origin.y + px(f32::from(row) * metrics.line_height),
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
