//! The right sidebar's Cairn tab: whether cairn is at work in the focused pane's directory, and
//! what the next session there would be given, as `cairn.rs` reads them. cairn is read in the
//! background while the tab shows: at once when it comes up or the pane's directory changes, then
//! every few seconds. Under the repository's name and whether it has adopted cairn, a line of which
//! agents have their hooks and how many saves wait to be taken in, then the text, drawn as
//! Markdown. A repository that has not adopted it offers Adopt, which asks first; nothing else
//! here changes anything (DESIGN §13 P5-55).
use crate::{
    cairn::{self, Adopting, Body, Found, Read},
    fonts::UiFont,
    footer_icon::{self, Icon},
    markdown,
    theme::Theme,
    view::hsla,
};
use gpui::{
    ClickEvent, Context, Div, Font, FontStyle, FontWeight, HighlightStyle, Hsla, IntoElement,
    Render, Stateful, StyledText, Task, UnderlineStyle, Window, div, prelude::*, px, relative,
};
use std::{
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

/// How often cairn is read again while the tab shows.
const EVERY: Duration = Duration::from_secs(5);
/// The ground of the small tags and boxes, the rule under the status line, and the ground of
/// words written as code and the rule after a section's label, in the text colour.
const SOFT: f32 = 0.06;
const RULE: f32 = 0.06;
const CODE: f32 = 0.07;
/// The confirmation card's face in the text colour, and its edge in the accent.
const CARD_FACE: f32 = 0.045;
const CARD_EDGE: f32 = 0.34;

/// What the window tells the tab every time it draws.
pub struct Frame {
    /// The focused pane's directory: `None` with no pane in focus, `Some(None)` for a pane with
    /// no directory to read.
    pub cwd: Option<Option<String>>,
    /// The right sidebar is open on this tab: read, and keep reading.
    pub active: bool,
    pub theme: Rc<Theme>,
    pub mono: Font,
}

/// The theme's colours the tab uses.
#[derive(Clone, Copy)]
struct Colors {
    ground: Hsla,
    text: Hsla,
    title: Hsla,
    bright: Hsla,
    muted: Hsla,
    dim: Hsla,
    accent: Hsla,
    green: Hsla,
    red: Hsla,
    yellow: Hsla,
}

impl Colors {
    fn of(theme: &Theme) -> Self {
        let fg =
            |pick: fn(&crate::preset::Theme) -> crate::preset::Color| hsla(theme.fg(pick), 1.0);
        Colors {
            ground: hsla(theme.terminal().background, 1.0),
            text: fg(|t| t.agents_text),
            title: fg(|t| t.bright),
            bright: fg(|t| t.agents_branch),
            muted: fg(|t| t.agents_dim),
            dim: fg(|t| t.agents_dimmer),
            accent: fg(|t| t.agents_accent),
            green: fg(|t| t.agents_green),
            red: fg(|t| t.agents_red),
            yellow: fg(|t| t.agents_yellow),
        }
    }
}

pub struct CairnView {
    theme: Rc<Theme>,
    colors: Colors,
    mono: Font,
    cwd: Option<Option<String>>,
    active: bool,
    /// The latest read, for whichever directory it was.
    read: Option<Read>,
    /// The reading while the tab shows, and what stops the command it has running.
    watching: Option<(Task<()>, Arc<AtomicBool>)>,
    adopting: Adopting,
    home: Option<PathBuf>,
}

impl CairnView {
    pub fn new(theme: Rc<Theme>, mono: Font) -> Self {
        CairnView {
            colors: Colors::of(&theme),
            theme,
            mono,
            cwd: None,
            active: false,
            read: None,
            watching: None,
            adopting: Adopting::default(),
            home: std::env::var_os("HOME").map(PathBuf::from),
        }
    }

    /// The window's latest: redraws only when something here changed.
    pub fn frame(&mut self, frame: Frame, cx: &mut Context<Self>) {
        let mut changed = false;
        let moved = frame.cwd != self.cwd;
        if moved || frame.active != self.active {
            self.cwd = frame.cwd;
            self.active = frame.active;
            // Left for another tab or closed: a question still up is taken back.
            if !self.active {
                changed |= self.adopting.follow(None);
            }
            self.watch(cx);
            changed |= moved;
        }
        if !Rc::ptr_eq(&frame.theme, &self.theme) {
            self.colors = Colors::of(&frame.theme);
            self.theme = frame.theme;
            changed = true;
        }
        if frame.mono != self.mono {
            self.mono = frame.mono;
            changed = true;
        }
        if changed {
            cx.notify();
        }
    }

    /// Reads the pane's directory now and every few seconds after, in the background, while the
    /// tab shows; any reading under way is dropped first.
    fn watch(&mut self, cx: &mut Context<Self>) {
        if let Some((_, cancel)) = self.watching.take() {
            cancel.store(true, Ordering::Relaxed);
        }
        let (true, Some(Some(cwd))) = (self.active, self.cwd.clone()) else {
            return;
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let stop = cancel.clone();
        let task = cx.spawn(async move |this, cx| {
            loop {
                let (dir, stop) = (cwd.clone(), stop.clone());
                let read = cx
                    .background_executor()
                    .spawn(async move { cairn::read(cairn::PROGRAM, "git", &dir, &stop) })
                    .await;
                if this.update(cx, |view, cx| view.accept(read, cx)).is_err() {
                    break;
                }
                cx.background_executor().timer(EVERY).await;
            }
        });
        self.watching = Some((task, cancel));
    }

    /// A read came back: drawn when it differs, and a confirmation card that is not about what
    /// it shows closes.
    fn accept(&mut self, read: Read, cx: &mut Context<Self>) {
        let repo = match &read {
            Read::Found(Found {
                body: Body::NotAdopted(repo),
                ..
            }) => Some(repo.as_path()),
            _ => None,
        };
        let closed = self.adopting.follow(repo);
        if closed || self.read.as_ref() != Some(&read) {
            self.read = Some(read);
            cx.notify();
        }
    }

    /// Adopt on the confirmation card: `cairn adopt` in the directory shown, in the background,
    /// and whatever comes of it, cairn read again at once.
    fn adopt(&mut self, cx: &mut Context<Self>) {
        let Some(Read::Found(Found {
            cwd,
            body: Body::NotAdopted(repo),
            ..
        })) = &self.read
        else {
            return;
        };
        let (cwd, repo) = (cwd.clone(), repo.clone());
        if !self.adopting.confirm() {
            return;
        }
        let task = cx.background_spawn(async move {
            cairn::adopt(cairn::PROGRAM, &cwd, &AtomicBool::new(false))
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |view, cx| {
                view.adopting.finish(&repo, result);
                view.watch(cx);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// The card Adopt has up for `repo`, if any.
    fn asking(&self, repo: &Path) -> Option<&Adopting> {
        match &self.adopting {
            Adopting::Asking(of)
            | Adopting::Running(of)
            | Adopting::Done(of)
            | Adopting::Refused(of, _)
                if of == repo =>
            {
                Some(&self.adopting)
            }
            _ => None,
        }
    }
}

impl Render for CairnView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = UiFont::get(cx);
        let c = self.colors;
        let root = div()
            .size_full()
            .flex()
            .flex_col()
            .text_color(c.text)
            .text_size(ui.px(13.0));
        let rule = || {
            div()
                .flex_shrink_0()
                .mt(ui.px(6.0))
                .h(px(1.0))
                .bg(c.text.opacity(RULE))
        };
        let follows = "Cairn follows the focused pane's directory";
        match (&self.cwd, &self.read) {
            (None, _) => root.child(
                self.middle(&ui)
                    .child(self.title("No pane in focus", &ui))
                    .child(self.sub(follows, &ui)),
            ),
            (Some(None), _) => root.child(
                self.middle(&ui)
                    .child(self.title("No directory to read", &ui))
                    .child(self.sub(follows, &ui)),
            ),
            // Still reading for the first time.
            (_, None) => root,
            (_, Some(Read::NotInstalled)) => root.child(rule()).child(
                self.middle(&ui)
                    .child(self.title("cairn is not installed", &ui))
                    .child(self.sub(
                        "This tab shows cairn's handover records once the `cairn` command is on \
                         this Mac.",
                        &ui,
                    )),
            ),
            (_, Some(Read::Failed(why))) => root.child(rule()).child(
                self.middle(&ui)
                    .gap(ui.px(10.0))
                    .child(self.title("Could not read cairn", &ui))
                    .child(self.refusal(why, &ui)),
            ),
            (_, Some(Read::Found(found))) => root
                .child(self.header(found, &ui))
                .child(self.hooks(found, &ui))
                .child(self.body(found, &ui, cx)),
        }
    }
}

impl CairnView {
    /// The repository's name, or the folder's, its branch, and whether it has adopted cairn.
    fn header(&self, found: &Found, ui: &UiFont) -> Div {
        let c = self.colors;
        let (words, color, ground, dot) = if found.adopted {
            ("Adopted", c.green, c.green.opacity(0.12), c.green)
        } else {
            ("Not adopted", c.muted, c.text.opacity(SOFT), c.dim)
        };
        div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(ui.px(8.0))
            .min_w(px(0.0))
            .pt(ui.px(6.0))
            .pb(ui.px(10.0))
            .px(ui.px(14.0))
            .child(footer_icon::icon(
                Icon::Folder,
                c.muted,
                ui.scale(15.0 / footer_icon::SIZE),
            ))
            .child(
                div()
                    .flex_shrink(1.0)
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_size(ui.px(14.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(found.name.clone()),
            )
            .children(found.branch.clone().map(|branch| {
                div()
                    .flex_shrink(1.0)
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .px(ui.px(7.0))
                    .py(ui.px(1.0))
                    .rounded_full()
                    .bg(c.text.opacity(SOFT))
                    .font(self.mono.clone())
                    .text_size(ui.px(11.0))
                    .text_color(c.muted)
                    .child(branch)
            }))
            .child(div().flex_1())
            .child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap(ui.px(6.0))
                    .px(ui.px(10.0))
                    .py(ui.px(3.0))
                    .rounded_full()
                    .bg(ground)
                    .whitespace_nowrap()
                    .text_size(ui.px(11.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(color)
                    .child(div().size(ui.px(6.0)).rounded_full().bg(dot))
                    .child(words),
            )
    }

    /// Which agents have cairn's hooks, and in an adopted repository how many saves cairn has
    /// not yet taken in.
    fn hooks(&self, found: &Found, ui: &UiFont) -> Div {
        let c = self.colors;
        let agent = |name: &'static str, installed: bool| {
            let mark = if installed {
                footer_icon::icon(Icon::Check, c.green, ui.scale(11.0 / footer_icon::SIZE))
                    .into_any_element()
            } else {
                div()
                    .flex_shrink_0()
                    .w(ui.px(6.0))
                    .h(ui.px(1.5))
                    .rounded_full()
                    .bg(c.dim)
                    .into_any_element()
            };
            div()
                .flex_shrink_0()
                .flex()
                .items_center()
                .gap(ui.px(5.0))
                .text_color(if installed { c.muted } else { c.dim })
                .child(name)
                .child(mark)
        };
        let waiting = found.adopted.then(|| {
            let words = format!("{} uncollected", found.uncollected);
            let words = div().flex_shrink_0().whitespace_nowrap().child(words);
            if found.uncollected == 0 {
                words.text_color(c.dim)
            } else {
                words
                    .px(ui.px(8.0))
                    .py(ui.px(1.0))
                    .rounded_full()
                    .bg(c.yellow.opacity(0.14))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(c.yellow)
            }
        });
        div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(ui.px(12.0))
            .px(ui.px(14.0))
            .pb(ui.px(11.0))
            .border_b_1()
            .border_color(c.text.opacity(RULE))
            .text_size(ui.px(12.0))
            .child(div().flex_shrink_0().text_color(c.dim).child("Hooks"))
            .child(agent("Claude", found.claude))
            .child(agent("Codex", found.codex))
            .child(div().flex_1())
            .children(waiting)
    }

    /// Under the header: what cairn has for the next session, or why there is nothing.
    fn body(&self, found: &Found, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let c = self.colors;
        match &found.body {
            Body::NoHooks => self
                .middle(ui)
                .child(self.title("cairn's hooks are not installed", ui))
                .child(self.sub(
                    "Nothing is recorded for Claude Code or Codex until they are. To install \
                     them, run in a terminal:",
                    ui,
                ))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(ui.px(4.0))
                        .max_w_full()
                        .px(ui.px(12.0))
                        .py(ui.px(7.0))
                        .rounded(ui.px(7.0))
                        .bg(c.text.opacity(SOFT))
                        .font(self.mono.clone())
                        .text_size(ui.px(11.5))
                        .line_height(relative(1.5))
                        .text_color(c.bright)
                        .child("cairn install --agent claude")
                        .child("cairn install --agent codex"),
                ),
            Body::Outside => self
                .middle(ui)
                .child(self.title("This folder has not adopted cairn", ui))
                .child(self.sub("Adopt is offered inside Git repositories only.", ui)),
            Body::NoRecords => self
                .middle(ui)
                .child(self.title("No records yet", ui))
                .child(self.sub(
                    "The first one is saved when an agent you talk to here finishes a turn.",
                    ui,
                )),
            Body::NotAdopted(repo) => match self.asking(repo) {
                Some(adopting) => self.confirm(found, repo, adopting, ui, cx),
                None => self.offer(repo, ui, cx),
            },
            Body::Records(text) => div()
                .flex_1()
                .min_h(px(0.0))
                .flex()
                .flex_col()
                .child(self.text(text, ui)),
        }
    }

    /// A repository that has not adopted cairn: what adopting does, where, and Adopt….
    fn offer(&self, repo: &Path, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let c = self.colors;
        let ask = repo.to_owned();
        self.middle(ui)
            .gap(ui.px(12.0))
            .pb(ui.px(48.0))
            .child(footer_icon::icon(
                Icon::Cairn,
                c.dim,
                ui.scale(34.0 / footer_icon::SIZE),
            ))
            .child(self.title("This repository has not adopted cairn", ui))
            .child(self.sub(
                "Once adopted, the agents you talk to here save a short handover note as each \
                 turn ends, and the next session starts from it.",
                ui,
            ))
            .child(self.place(repo, ui))
            .child(
                self.button("cairn-adopt-ask", "Adopt\u{2026}", true, ui)
                    .mt(ui.px(6.0))
                    .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                        view.adopting.ask(&ask);
                        cx.notify();
                    })),
            )
    }

    /// Adopt… was pressed: what it does and how to undo it, then Cancel and Adopt; while it runs
    /// and once it has, that, and after it failed, why.
    fn confirm(
        &self,
        found: &Found,
        repo: &Path,
        adopting: &Adopting,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Div {
        let c = self.colors;
        let words = |text: &str, color: Hsla| {
            div()
                .text_size(ui.px(12.5))
                .line_height(relative(1.55))
                .text_color(color)
                .child(self.words(text))
        };
        let foot = div()
            .flex()
            .items_center()
            .justify_end()
            .gap(ui.px(8.0))
            .mt(ui.px(4.0));
        let under_way = match adopting {
            Adopting::Running(_) => Some("Adopting\u{2026}"),
            Adopting::Done(_) => Some("Adopted"),
            _ => None,
        };
        let foot = if let Some(words) = under_way {
            foot.h(ui.px(30.0))
                .text_size(ui.px(12.5))
                .text_color(c.dim)
                .child(words)
        } else {
            foot.child(
                self.button("cairn-adopt-cancel", "Cancel", false, ui)
                    .on_click(cx.listener(|view, _: &ClickEvent, _, cx| {
                        if view.adopting.cancel() {
                            cx.notify();
                        }
                    })),
            )
            .child(
                self.button("cairn-adopt-confirm", "Adopt", true, ui)
                    .on_click(cx.listener(|view, _: &ClickEvent, _, cx| view.adopt(cx))),
            )
        };
        let card = div()
            .flex()
            .flex_col()
            .gap(ui.px(10.0))
            .pt(ui.px(16.0))
            .px(ui.px(16.0))
            .pb(ui.px(14.0))
            .rounded(ui.px(12.0))
            .border_1()
            .border_color(c.accent.opacity(CARD_EDGE))
            .bg(c.text.opacity(CARD_FACE))
            .child(
                div()
                    .text_size(ui.px(14.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(c.title)
                    .child(format!("Adopt cairn in {}?", found.name)),
            )
            .child(div().flex().child(self.place(repo, ui)))
            .child(words(
                "This takes effect right away. Claude and Codex sessions you talk to in this \
                 repository, the open ones included, are asked to save a handover note before \
                 each turn ends, starting with their next turn.",
                c.bright,
            ))
            .child(words(
                "Dispatched agents are not affected. To undo, run `cairn unadopt` in this \
                 repository.",
                c.muted,
            ))
            .children(match adopting {
                Adopting::Refused(_, why) => Some(self.refusal(why, ui)),
                _ => None,
            })
            .child(foot);
        div()
            .flex_1()
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .justify_center()
            .px(ui.px(20.0))
            .pb(ui.px(48.0))
            .child(card)
    }

    /// The text the next session would be given, as Markdown, scrolling. cairn writes its
    /// sections as third-level headings and the parts of a record as second-level ones: a section
    /// is a quiet label with a rule after it, a part a small bold heading.
    fn text(&self, text: &str, ui: &UiFont) -> Stateful<Div> {
        /// What the block before was, for the room between the two.
        #[derive(Clone, Copy, PartialEq)]
        enum Before {
            Nothing,
            Heading,
            Words,
            Item,
        }
        let c = self.colors;
        let mut out = div()
            .id("cairn-text")
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .pt(ui.px(4.0))
            .px(ui.px(16.0))
            .pb(ui.px(20.0))
            .line_height(relative(1.55));
        let mut before = Before::Nothing;
        for block in markdown::blocks_by_line(text) {
            let apart = |after_words: f32, after_item: f32| match before {
                Before::Nothing | Before::Heading => 0.0,
                Before::Words => after_words,
                Before::Item => after_item,
            };
            let (element, now) = match &block {
                markdown::Block::Heading(3, spans) => (
                    div()
                        .flex()
                        .items_center()
                        .gap(ui.px(10.0))
                        .mt(ui.px(if before == Before::Nothing {
                            14.0
                        } else {
                            20.0
                        }))
                        .mb(ui.px(8.0))
                        .text_size(ui.px(11.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(c.muted)
                        .child(
                            div()
                                .flex_shrink(1.0)
                                .min_w(px(0.0))
                                .child(self.styled(spans)),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w(ui.px(12.0))
                                .h(px(1.0))
                                .bg(c.text.opacity(CODE)),
                        ),
                    Before::Heading,
                ),
                markdown::Block::Heading(_, spans) => (
                    div()
                        .mt(ui.px(16.0))
                        .mb(ui.px(4.0))
                        .text_size(ui.px(13.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(c.title)
                        .child(self.styled(spans)),
                    Before::Heading,
                ),
                markdown::Block::Paragraph(spans) => (
                    div().mt(ui.px(apart(12.0, 12.0))).child(self.styled(spans)),
                    Before::Words,
                ),
                markdown::Block::Item {
                    depth,
                    marker,
                    spans,
                } => {
                    let mark = match marker {
                        Some(markdown::Marker::Bullet) => div()
                            .mt(ui.px(8.0))
                            .size(ui.px(4.0))
                            .rounded_full()
                            .bg(c.dim),
                        Some(markdown::Marker::Number(n)) => {
                            div().text_color(c.dim).child(format!("{n}."))
                        }
                        // A further paragraph of the item above, under its words.
                        None => div().w(ui.px(4.0)),
                    };
                    (
                        div()
                            .flex()
                            .gap(ui.px(8.0))
                            .mt(ui.px(apart(6.0, 4.0)))
                            .pl(ui.px(18.0 * *depth as f32))
                            .child(mark.flex_shrink_0())
                            .child(div().flex_1().min_w(px(0.0)).child(self.styled(spans))),
                        Before::Item,
                    )
                }
                markdown::Block::Code(code) => (
                    div()
                        .mt(ui.px(apart(8.0, 8.0)))
                        .px(ui.px(10.0))
                        .py(ui.px(8.0))
                        .rounded(ui.px(6.0))
                        .bg(c.text.opacity(SOFT))
                        .font(self.mono.clone())
                        .text_size(ui.px(11.5))
                        .child(code.clone()),
                    Before::Words,
                ),
            };
            out = out.child(element.flex_shrink_0());
            before = now;
        }
        out
    }

    /// The room under the header, what is in it in the middle.
    fn middle(&self, ui: &UiFont) -> Div {
        div()
            .flex_1()
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(ui.px(8.0))
            .px(ui.px(36.0))
            .pb(ui.px(20.0))
    }

    fn title(&self, text: &'static str, ui: &UiFont) -> Div {
        div()
            .text_size(ui.px(14.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(self.colors.title)
            .text_center()
            .child(text)
    }

    /// A quieter line or two under a title.
    fn sub(&self, text: &str, ui: &UiFont) -> Div {
        div()
            .text_size(ui.px(12.5))
            .line_height(relative(1.55))
            .text_color(self.colors.muted)
            .text_center()
            .child(self.words(text))
    }

    /// A repository's path, the home directory written `~`, as a small tag that breaks anywhere.
    fn place(&self, repo: &Path, ui: &UiFont) -> Div {
        let c = self.colors;
        div()
            .max_w_full()
            .min_w(px(0.0))
            .px(ui.px(8.0))
            .py(ui.px(2.0))
            .rounded(ui.px(6.0))
            .bg(c.text.opacity(SOFT))
            .font(self.mono.clone())
            .text_size(ui.px(11.0))
            .text_color(c.muted)
            .child(cairn::tilde(repo, self.home.as_deref()))
    }

    /// Why cairn could not be read, or would not adopt: the first line of what it said.
    fn refusal(&self, why: &str, ui: &UiFont) -> Div {
        let c = self.colors;
        div()
            .max_w_full()
            .min_w(px(0.0))
            .px(ui.px(10.0))
            .py(ui.px(6.0))
            .rounded(ui.px(7.0))
            .bg(c.red.opacity(0.1))
            .font(self.mono.clone())
            .text_size(ui.px(11.5))
            .line_height(relative(1.5))
            .text_color(c.red)
            .child(why.to_owned())
    }

    /// A button: the one that goes on in the accent, the other quiet.
    fn button(
        &self,
        id: &'static str,
        words: &'static str,
        goes_on: bool,
        ui: &UiFont,
    ) -> Stateful<Div> {
        let c = self.colors;
        let button = div()
            .id(id)
            .flex_shrink_0()
            .flex()
            .items_center()
            .h(ui.px(30.0))
            .rounded(ui.px(8.0))
            .whitespace_nowrap()
            .text_size(ui.px(12.5))
            .cursor_pointer()
            .child(words);
        if goes_on {
            button
                .px(ui.px(18.0))
                .bg(c.accent)
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(c.ground)
                .hover(move |style| style.bg(c.accent.opacity(0.88)))
        } else {
            button
                .px(ui.px(14.0))
                .bg(c.text.opacity(SOFT))
                .text_color(c.text)
                .hover(move |style| style.bg(c.text.opacity(0.1)))
        }
    }

    /// A line of the tab's own words, what is between backticks written as code.
    fn words(&self, text: &str) -> StyledText {
        match markdown::blocks(text).into_iter().next() {
            Some(markdown::Block::Paragraph(spans)) => self.styled(&spans),
            _ => StyledText::new(text.to_owned()),
        }
    }

    /// Markdown's words in their styles, as the Kanban task dialog's Preview draws them: bold,
    /// italic, code in the monospace font on a faint ground, and links underlined in the accent.
    fn styled(&self, spans: &[markdown::Span]) -> StyledText {
        let c = self.colors;
        let mut text = String::new();
        let (mut highlights, mut fonts) = (Vec::new(), Vec::new());
        for span in spans {
            let start = text.len();
            text.push_str(&span.text);
            let range = start..text.len();
            let mut style = HighlightStyle::default();
            if span.bold {
                style.font_weight = Some(FontWeight::BOLD);
            }
            if span.italic {
                style.font_style = Some(FontStyle::Italic);
            }
            if span.link {
                style.color = Some(c.accent);
                style.underline = Some(UnderlineStyle {
                    thickness: px(1.0),
                    color: Some(c.accent),
                    wavy: false,
                });
            }
            if span.code {
                style.background_color = Some(c.text.opacity(CODE));
                fonts.push((range.clone(), self.mono.family.clone()));
            }
            if style != HighlightStyle::default() {
                highlights.push((range, style));
            }
        }
        StyledText::new(text)
            .with_highlights(highlights)
            .with_font_family_overrides(fonts)
    }
}
