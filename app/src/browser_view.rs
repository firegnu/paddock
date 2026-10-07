//! The right sidebar's Browser tab (DESIGN §13, P5-28): the toolbar from the design (back,
//! forward, reload or stop while loading, the address, open in the default browser) over the page
//! (`browser.rs`), which it lays out. Before anything is opened it shows a quiet empty state; after
//! a load fails, a quiet error with Retry in the page's place, and for a site whose certificate is
//! not trusted, a way to open it in the default browser. ⌘F opens a find bar under the toolbar,
//! as a pane's, that looks through the page.
//!
//! It keeps the window's keyboard record ([`Keys`]): the page has a focus handle of its own, so
//! GPUI's focus says when the page has the keyboard as AppKit's does, and ⌘L, ⌘R, ⌘F, ⌘G and ⇧⌘G
//! are the Browser's there and in its address and find fields.
use crate::{
    browser::{self, Failure, Keys, Look, Owner, Page, Scene, Seen, Showing, State},
    fonts::UiFont,
    footer_icon::{self, Icon},
    menu,
    right_panel::{self, Tab, Tip},
    text_input::{self, Changed, TextInput},
    theme::Theme,
    view::hsla,
};
use gpui::{
    Bounds, ClickEvent, Context, Div, Entity, EventEmitter, FocusHandle, Focusable, FontWeight,
    HighlightStyle, KeyDownEvent, MouseButton, Pixels, Render, Stateful, StyledText, Subscription,
    Window, canvas, div, prelude::*, px, relative,
};
use std::{
    cell::Cell,
    net::IpAddr,
    ops::Range,
    path::{Path, PathBuf},
    rc::Rc,
    time::{Duration, Instant},
};

/// The page area's corners; the page sits inside its 1-point rim.
const RADIUS: f32 = 8.0;
/// How often the page's state is read.
const POLL: Duration = Duration::from_millis(50);
/// How long the toolbar shows a download that finished.
const SAVED: Duration = Duration::from_secs(6);

type Pick = fn(&crate::preset::Theme) -> crate::preset::Color;

/// Sent when the address the toolbar shows changes, for the layout to keep; none once the page
/// is closed.
pub struct Visited(pub Option<String>);

/// Sent when the keyboard's record needs the window to do something.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handoff {
    /// The page was clicked while a popup was open: it closes.
    Dismiss,
    /// The Browser went away with the keyboard: the active pane takes it.
    ToPane,
}

pub struct BrowserView {
    theme: Rc<Theme>,
    /// The page, made the first time the tab shows.
    page: Option<Page>,
    /// Its state as last read.
    state: State,
    /// What to open once the page is made: the saved address, or one typed before.
    pending: Option<String>,
    /// Something has been opened; until then the tab shows its empty state.
    opened: bool,
    /// An address typed that the system would not take.
    refused: Option<Failure>,
    /// The address last sent as [`Visited`].
    visited: Option<String>,
    address: Entity<TextInput>,
    /// The find bar, while it is open.
    find: Option<FindBar>,
    /// The text of the field taking the keyboard is to be selected once it shows (⌘L, ⌘F).
    select: bool,
    /// Where the page goes as laid out this frame, until [`BrowserView::place`] takes it.
    area: Rc<Cell<Option<Bounds<Pixels>>>>,
    /// GPUI's focus is here while the page has the keyboard: on its area, or its error state.
    page_focus: FocusHandle,
    /// Who has the window's keyboard.
    keys: Keys,
    /// How the page did at the last frame.
    showing: Showing,
    /// A popup was open at the last frame.
    dialog: bool,
    /// A download that finished, and when, while the toolbar shows it.
    saved: Option<(PathBuf, Instant)>,
}

struct FindBar {
    input: Entity<TextInput>,
    /// The last search found nothing.
    missed: bool,
    _subscription: Subscription,
}

impl EventEmitter<Visited> for BrowserView {}
impl EventEmitter<Handoff> for BrowserView {}

impl BrowserView {
    /// The tab, to open `url` when it first shows, as if typed ([`reachable`]).
    pub fn new(theme: Rc<Theme>, url: Option<String>, cx: &mut Context<Self>) -> Self {
        let colors = colors(&theme);
        let address = cx.new(|cx| TextInput::new("", "Enter an address", colors, cx));
        let url = url.map(|url| reachable(&url));
        BrowserView {
            theme,
            page: None,
            state: State::default(),
            opened: url.is_some(),
            visited: url.clone(),
            pending: url,
            refused: None,
            address,
            find: None,
            select: false,
            area: Rc::default(),
            page_focus: cx.focus_handle(),
            keys: Keys::default(),
            showing: Showing::default(),
            dialog: false,
            saved: None,
        }
    }

    /// Who has the window's keyboard.
    pub fn owner(&self) -> Owner {
        self.keys.owner()
    }

    /// The page has the keyboard again, once it shows: after a popup that borrowed it closes.
    pub fn give_back(&self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.page_focus, cx);
    }

    /// Opens `url`, a web address, as if typed and entered (`paddock ctl browse`); the keyboard
    /// stays where it is.
    pub fn visit(&mut self, url: String, cx: &mut Context<Self>) {
        self.go(url, cx);
    }

    /// New colours, after the theme changed.
    pub fn set_theme(&mut self, theme: Rc<Theme>, cx: &mut Context<Self>) {
        let colors = colors(&theme);
        self.address
            .update(cx, |input, cx| input.set_colors(colors, cx));
        self.theme = theme;
        cx.notify();
    }

    /// Puts the page where this frame leaves room for it, once everything drawn over the window
    /// has noted itself (`browser::cover`): `open` and `on_browser` say whether the sidebar is
    /// open on this tab, `dragging` whether a divider is being dragged, `dialog` whether a popup
    /// is open. The keyboard's record settles after the frame.
    pub fn place(
        &mut self,
        open: bool,
        on_browser: bool,
        dragging: bool,
        dialog: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let scene = Scene {
            open,
            browser: on_browser,
            area: self.area.take(),
            covers: browser::covers(window),
            dragging,
        };
        let shown = scene.page().filter(|_| self.page.is_some());
        if let Some(page) = &mut self.page {
            page.place(shown, window);
        }
        self.showing = match shown {
            _ if !(open && on_browser) => Showing::Away,
            Some(_) => Showing::Shown,
            None => Showing::Hidden,
        };
        self.dialog = dialog;
        // Outside GPUI's drawing: it may move AppKit's keyboard and GPUI's focus.
        cx.defer_in(window, |this, window, cx| this.settle(window, cx));
    }

    /// Brings the keyboard's record up to date with GPUI's focus, AppKit's keyboard, the page and
    /// popups, and makes the moves it says: AppKit's before GPUI's when the keyboard leaves the
    /// page.
    fn settle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let focus = if self.page_focus.is_focused(window) {
            Owner::Page
        } else if self.address.read(cx).focus_handle(cx).is_focused(window) {
            Owner::Address
        } else if self.finding(window, cx) {
            Owner::Find
        } else {
            Owner::Paddock
        };
        let seen = Seen {
            focus,
            native: self.page.as_ref().is_some_and(Page::has_keys),
            showing: self.showing,
            dialog: self.dialog,
        };
        let moves = self.keys.settle(seen);
        match moves.native {
            Some(true) => {
                if let Some(page) = &self.page {
                    page.focus();
                }
            }
            Some(false) => browser::take_keys(window),
            None => {}
        }
        if moves.focus_page {
            window.focus(&self.page_focus, cx);
        }
        if moves.dismiss {
            cx.emit(Handoff::Dismiss);
        }
        if moves.to_pane {
            cx.emit(Handoff::ToPane);
        }
    }

    /// Makes the page, opening what waits, and reads its state from then on.
    fn open_page(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.page.is_some() {
            return;
        }
        let Some(page) = Page::new(window, f64::from(RADIUS - 1.0)) else {
            return;
        };
        self.page = Some(page);
        if let Some(url) = self.pending.take() {
            self.go(url, cx);
        }
        cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor().timer(POLL).await;
                if this
                    .update_in(cx, |view, window, cx| view.poll(window, cx))
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        cx.notify();
    }

    /// Reads the page's state: drawn again when it changed, and the layout told when the address
    /// shown did; and whether a search found anything, and a download that finished, which the
    /// toolbar shows for a while. The keyboard's record settles too, as a click on the page shows
    /// only here while nothing else is drawn.
    fn poll(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(page) = &self.page else {
            return;
        };
        let state = page.state();
        let found = page.found();
        let saved = page.saved();
        if state != self.state {
            self.state = state;
            cx.notify();
        }
        if let Some(found) = found
            && let Some(bar) = &mut self.find
        {
            bar.missed = !found && !bar.input.read(cx).text().is_empty();
            cx.notify();
        }
        if let Some(path) = saved {
            self.saved = Some((path, Instant::now()));
            cx.notify();
        } else if self
            .saved
            .as_ref()
            .is_some_and(|(_, at)| at.elapsed() >= SAVED)
        {
            self.saved = None;
            cx.notify();
        }
        if let Some(url) = self.shown_url()
            && self.visited.as_deref() != Some(url)
        {
            let url = url.to_owned();
            self.visited = Some(url.clone());
            cx.emit(Visited(Some(url)));
        }
        self.settle(window, cx);
    }

    /// Why the page shows its error instead: an address refused, else a load that failed.
    fn failure(&self) -> Option<&Failure> {
        self.refused.as_ref().or(self.state.failure.as_ref())
    }

    /// The address the toolbar shows: the one that failed, else the page's.
    fn shown_url(&self) -> Option<&str> {
        self.failure()
            .and_then(|failure| failure.url.as_deref())
            .or(self.state.url.as_deref())
    }

    /// Opens `url`, in the page once there is one.
    fn go(&mut self, url: String, cx: &mut Context<Self>) {
        self.opened = true;
        self.refused = None;
        match &self.page {
            Some(page) => {
                if !page.load(&url) {
                    self.refused = Some(Failure {
                        url: Some(url),
                        reason: "This address cannot be opened.".into(),
                        certificate: false,
                    });
                }
            }
            None => self.pending = Some(url),
        }
        cx.notify();
    }

    /// A click on the address, or ⌘L (`select`): the field takes the keyboard, holding the address
    /// shown, all of it selected for ⌘L. After this event, AppKit's keyboard first: AppKit asks
    /// GPUI's view about its text as it takes it.
    fn edit(&mut self, select: bool, window: &mut Window, cx: &mut Context<Self>) {
        cx.defer_in(window, move |this, window, cx| {
            browser::take_keys(window);
            let editing = this.address.read(cx).focus_handle(cx).is_focused(window);
            if !editing {
                let text = this.shown_url().unwrap_or_default().to_owned();
                this.address
                    .update(cx, |input, cx| input.set_text(text, cx));
                let focus = this.address.read(cx).focus_handle(cx);
                window.focus(&focus, cx);
            }
            this.select = select;
            cx.notify();
        });
    }

    /// ⏎ in the address field: opens what was typed, and the page takes the keyboard (AppKit's
    /// as the record settles).
    fn commit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let typed = self.address.read(cx).text().to_owned();
        let Some(url) = address(&typed) else {
            return;
        };
        window.focus(&self.page_focus, cx);
        self.go(url, cx);
    }

    /// ⌘R: the page loads again, or tries a failed load again.
    fn reload_page(&mut self, cx: &mut Context<Self>) {
        let Some(page) = &self.page else {
            return;
        };
        if let Some(url) = self.failure().and_then(|failure| failure.url.clone()) {
            self.go(url, cx);
        } else if self.opened {
            page.reload();
            self.refused = None;
            cx.notify();
        }
    }

    /// The find field has the keyboard.
    fn finding(&self, window: &Window, cx: &Context<Self>) -> bool {
        self.find
            .as_ref()
            .is_some_and(|bar| bar.input.read(cx).focus_handle(cx).is_focused(window))
    }

    /// ⌘F: the find bar, its field taking the keyboard with what it holds selected; AppKit's
    /// keyboard first, after this event, as for the address field. Nothing before a page is
    /// opened.
    fn open_find(&mut self, _: &menu::Find, window: &mut Window, cx: &mut Context<Self>) {
        if !self.opened {
            return;
        }
        if self.find.is_none() {
            let input = cx.new(|cx| TextInput::new("", "Find", colors(&self.theme), cx));
            // Typing looks again from where the match shown starts.
            let subscription = cx.subscribe(&input, |this, _, _: &Changed, cx| {
                this.run_find(Look::Again, cx)
            });
            self.find = Some(FindBar {
                input,
                missed: false,
                _subscription: subscription,
            });
        }
        cx.defer_in(window, |this, window, cx| {
            let Some(bar) = &this.find else {
                return;
            };
            browser::take_keys(window);
            let focus = bar.input.read(cx).focus_handle(cx);
            window.focus(&focus, cx);
            this.select = true;
            cx.notify();
        });
    }

    /// ⏎ or ⌘G: the next match, the bar opened first if it is not.
    fn find_next(&mut self, _: &menu::FindNext, window: &mut Window, cx: &mut Context<Self>) {
        if self.find.is_none() {
            self.open_find(&menu::Find, window, cx);
        }
        self.run_find(Look::Next, cx);
    }

    /// ⇧⏎ or ⇧⌘G: the match before.
    fn find_previous(
        &mut self,
        _: &menu::FindPrevious,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.find.is_none() {
            self.open_find(&menu::Find, window, cx);
        }
        self.run_find(Look::Previous, cx);
    }

    /// Looks through the page for what the find field holds; whether it found anything comes back
    /// as the page is polled.
    fn run_find(&mut self, look: Look, cx: &mut Context<Self>) {
        let (Some(bar), Some(page)) = (&mut self.find, &self.page) else {
            return;
        };
        let query = bar.input.read(cx).text().to_owned();
        if query.is_empty() {
            bar.missed = false;
        } else {
            page.find(&query, look);
        }
        cx.notify();
    }

    /// Esc or ×: the bar goes and the page has the keyboard back.
    fn close_find(&mut self, _: &menu::CloseFind, window: &mut Window, cx: &mut Context<Self>) {
        self.find = None;
        window.focus(&self.page_focus, cx);
        cx.notify();
    }

    fn back(&mut self, cx: &mut Context<Self>) {
        self.refused = None;
        if let Some(page) = &self.page {
            page.back();
        }
        cx.notify();
    }

    fn forward(&mut self, cx: &mut Context<Self>) {
        self.refused = None;
        if let Some(page) = &self.page {
            page.forward();
        }
        cx.notify();
    }

    /// Reload, which stops a load under way and tries a failed one again.
    fn reload(&mut self, cx: &mut Context<Self>) {
        let Some(page) = &self.page else {
            return;
        };
        if self.state.loading {
            page.stop();
        } else if let Some(url) = self.failure().and_then(|failure| failure.url.clone()) {
            self.go(url, cx);
        } else {
            page.reload();
            self.refused = None;
            cx.notify();
        }
    }

    /// ×: back to the empty state. The page goes and a blank one takes its place, so nothing of it
    /// runs on and its history goes with it; the layout forgets its address, and the active pane
    /// takes the keyboard if the Browser had it. Sites' data stays.
    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.page = None;
        self.page = Page::new(window, f64::from(RADIUS - 1.0));
        self.state = State::default();
        self.opened = false;
        self.pending = None;
        self.refused = None;
        self.find = None;
        self.saved = None;
        self.visited = None;
        cx.emit(Visited(None));
        if self.keys.close() {
            cx.emit(Handoff::ToPane);
        }
        cx.notify();
    }

    /// Back, forward, reload or stop, the address, open in the default browser, and close.
    fn toolbar(&self, window: &Window, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let theme = &*self.theme;
        let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
        let highlight = hsla(theme.bg(|t| t.agent_selected), 1.0);
        // Faint while there is nothing for it to do.
        let button = |id: &'static str, icon: Icon, enabled: bool| {
            let color = if enabled {
                fg(|t| t.muted)
            } else {
                fg(|t| t.agents_border)
            };
            div()
                .id(id)
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .size(ui.px(26.0))
                .rounded(px(6.0))
                .when(enabled, |button| {
                    button
                        .cursor_pointer()
                        .hover(move |style| style.bg(highlight))
                })
                .child(footer_icon::icon(
                    icon,
                    color,
                    ui.scale(12.0 / footer_icon::SIZE),
                ))
        };
        let state = &self.state;
        let on_back = cx.listener(|this, _: &ClickEvent, _, cx| this.back(cx));
        let on_forward = cx.listener(|this, _: &ClickEvent, _, cx| this.forward(cx));
        let on_reload = cx.listener(|this, _: &ClickEvent, _, cx| this.reload(cx));
        let on_open = cx.listener(|this, _: &ClickEvent, _, cx| {
            if let Some(url) = this.shown_url() {
                cx.open_url(url);
            }
        });
        let on_close = cx.listener(|this, _: &ClickEvent, window, cx| this.close(window, cx));
        let reloads = self.page.is_some() && self.opened;
        let opens = self.shown_url().is_some();
        let tip = Tip::new("Open in default browser", theme, ui);
        let close_tip = Tip::new("Close page", theme, ui);
        div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(ui.px(4.0))
            .pt(ui.px(2.0))
            .px(ui.px(10.0))
            .pb(ui.px(10.0))
            .child(
                button("browser-back", Icon::Back, state.back)
                    .when(state.back, |b| b.on_click(on_back)),
            )
            .child(
                button("browser-forward", Icon::Forward, state.forward)
                    .when(state.forward, |b| b.on_click(on_forward)),
            )
            .child(
                button(
                    "browser-reload",
                    if state.loading {
                        Icon::Close
                    } else {
                        Icon::Reload
                    },
                    reloads,
                )
                .when(reloads, |b| b.on_click(on_reload)),
            )
            .child(self.address_field(window, ui, cx))
            .when_some(self.saved.as_ref(), |row, (path, _)| {
                row.child(self.saved_file(path, ui))
            })
            .child(
                button("browser-open", Icon::External, opens).when(opens, |b| {
                    b.tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
                        .on_click(on_open)
                }),
            )
            .child(
                button("browser-close", Icon::Close, self.opened).when(self.opened, |b| {
                    b.tooltip(move |_, cx| cx.new(|_| close_tip.clone()).into())
                        .on_click(on_close)
                }),
            )
    }

    /// A download that finished, by name, for a few seconds after: clicked, shown in Finder.
    fn saved_file(&self, path: &Path, ui: &UiFont) -> Stateful<Div> {
        let theme = &*self.theme;
        let highlight = hsla(theme.bg(|t| t.agent_selected), 1.0);
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let path = path.to_owned();
        div()
            .id("browser-saved")
            .flex_shrink_0()
            .max_w(ui.px(160.0))
            .h(ui.px(26.0))
            .px(ui.px(8.0))
            .flex()
            .items_center()
            .gap(ui.px(4.0))
            .rounded(px(6.0))
            .bg(highlight.opacity(0.5))
            .text_size(ui.px(12.0))
            .cursor_pointer()
            .hover(move |style| style.bg(highlight))
            .child(
                div()
                    .flex_shrink_0()
                    .text_color(hsla(theme.fg(|t| t.agents_accent), 1.0))
                    .child("↓"),
            )
            .child(
                div()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_color(hsla(theme.fg(|t| t.agents_text), 1.0))
                    .child(name),
            )
            .on_click(move |_, _, _| browser::reveal(&path))
    }

    /// The address: the page's while not typing, a local server's port or else the host bright;
    /// clicked, a field, which ⏎ opens and Esc leaves; a thin line along its foot while loading.
    fn address_field(&self, window: &Window, ui: &UiFont, cx: &mut Context<Self>) -> Stateful<Div> {
        let theme = &*self.theme;
        let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
        let highlight = hsla(theme.bg(|t| t.agent_selected), 1.0);
        let editing = self.address.read(cx).focus_handle(cx).is_focused(window);
        let field = div()
            .id("browser-address")
            .relative()
            .flex_1()
            .min_w(px(0.0))
            .h(ui.px(28.0))
            .flex()
            .items_center()
            .px(ui.px(10.0))
            .rounded(px(7.0))
            .border_1()
            .border_color(hsla(theme.fg(|t| t.agents_rule), 0.6))
            .bg(highlight.opacity(0.5))
            .overflow_hidden()
            .text_size(ui.px(12.5))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                match event.keystroke.key.as_str() {
                    "enter" => this.commit(window, cx),
                    // Back to the page, as typing had not started.
                    "escape" => {
                        window.focus(&this.page_focus, cx);
                        cx.notify();
                    }
                    _ => return,
                }
                cx.stop_propagation();
            }));
        let field = if editing {
            field.child(div().flex_1().min_w(px(0.0)).child(self.address.clone()))
        } else {
            let label = div()
                .flex_1()
                .min_w(px(0.0))
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .text_color(fg(|t| t.agents_dimmer));
            let label = match self.shown_url() {
                Some(url) => {
                    let (text, bright) = shown(url);
                    let style = HighlightStyle {
                        color: Some(fg(|t| t.agents_text)),
                        ..HighlightStyle::default()
                    };
                    label.child(StyledText::new(text).with_highlights([(bright, style)]))
                }
                None => label.child("Enter an address"),
            };
            field
                .cursor_text()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, window, cx| this.edit(false, window, cx)),
                )
                .child(label)
        };
        field.when(self.state.loading, |field| {
            let done = (self.state.progress as f32).clamp(0.05, 1.0);
            field.child(
                div()
                    .absolute()
                    .left(px(6.0))
                    .right(px(6.0))
                    .bottom_0()
                    .h(px(2.0))
                    .child(
                        div()
                            .h_full()
                            .w(relative(done))
                            .rounded(px(1.0))
                            .bg(fg(|t| t.agents_accent)),
                    ),
            )
        })
    }

    /// Under the toolbar while finding, the page shrunk to make room: a pane's find bar, laid out
    /// across the tab, with ↑ for the match before and ↓ for the next.
    fn find_bar(&self, bar: &FindBar, ui: &UiFont, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = &*self.theme;
        let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
        let highlight = hsla(theme.bg(|t| t.agent_selected), 1.0);
        let button = |id: &'static str, label: &'static str| {
            div()
                .id(id)
                .px(ui.px(5.0))
                .rounded(px(4.0))
                .text_color(fg(|t| t.muted))
                .cursor_pointer()
                .hover(move |style| style.bg(highlight))
                .child(label)
        };
        div()
            .id("browser-find-bar")
            .key_context(menu::FIND)
            .flex_shrink_0()
            .mx(ui.px(10.0))
            .mb(ui.px(10.0))
            .flex()
            .items_center()
            .gap(ui.px(6.0))
            .px(ui.px(8.0))
            .py(ui.px(4.0))
            .rounded(px(7.0))
            .border_1()
            .border_color(fg(|t| t.focus))
            .bg(hsla(theme.bg(|t| t.agents_bg), 1.0))
            .text_size(ui.px(12.0))
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
            .child(button("browser-find-previous", "↑").on_click(cx.listener(
                |this, _, window, cx| this.find_previous(&menu::FindPrevious, window, cx),
            )))
            .child(button("browser-find-next", "↓").on_click(
                cx.listener(|this, _, window, cx| this.find_next(&menu::FindNext, window, cx)),
            ))
            .child(button("browser-find-close", "×").on_click(
                cx.listener(|this, _, window, cx| this.close_find(&menu::CloseFind, window, cx)),
            ))
    }

    /// Under the toolbar: the empty state before anything is opened, the error after a failed
    /// load, else the area the page shows in, a faint rim round it. The page's focus handle is on
    /// the last two, so ⌘L and ⌘R stay the Browser's while its page is in error; a click on the
    /// page itself never reaches it.
    fn content(&self, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let theme = &*self.theme;
        if !self.opened {
            return right_panel::placeholder(theme, ui, Tab::Browser);
        }
        if let Some(failure) = self.failure() {
            return self.failed(failure, ui, cx).track_focus(&self.page_focus);
        }
        let area = self.area.clone();
        div()
            .flex_1()
            .min_h(px(0.0))
            .mx(ui.px(10.0))
            .mb(ui.px(10.0))
            .rounded(px(RADIUS))
            .border_1()
            .border_color(hsla(theme.fg(|t| t.agents_rule), 1.0))
            .track_focus(&self.page_focus)
            .child(canvas(move |bounds, _, _| area.set(Some(bounds)), |_, _, _, _| {}).size_full())
    }

    /// The error in the page's place: a line saying so, the system's reason, and Retry; for a site
    /// whose certificate is not trusted, that reason, and a way to open it in the default browser
    /// instead, as it is not let through here.
    fn failed(&self, failure: &Failure, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let theme = &*self.theme;
        let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
        let highlight = hsla(theme.bg(|t| t.agent_selected), 1.0);
        let button = |id: &'static str, label: &'static str| {
            div()
                .id(id)
                .h(ui.px(26.0))
                .px(ui.px(12.0))
                .flex()
                .items_center()
                .rounded(px(6.0))
                .border_1()
                .border_color(fg(|t| t.agents_rule))
                .text_size(ui.px(12.5))
                .text_color(fg(|t| t.agents_text))
                .cursor_pointer()
                .hover(move |style| style.bg(highlight))
                .child(label)
        };
        let reason = if failure.certificate {
            "This site’s certificate is not trusted.".to_owned()
        } else {
            failure.reason.clone()
        };
        let external = failure.url.clone().filter(|_| failure.certificate);
        div()
            .flex_1()
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(ui.px(6.0))
            .px(ui.px(24.0))
            .pb(ui.px(40.0))
            .child(
                div()
                    .text_size(ui.px(13.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(fg(|t| t.agents_text))
                    .child("Can’t open this page"),
            )
            .child(
                div()
                    .text_size(ui.px(12.0))
                    .text_center()
                    .text_color(fg(|t| t.agents_dim))
                    .child(reason),
            )
            .child(
                div()
                    .mt(ui.px(8.0))
                    .flex()
                    .flex_wrap()
                    .justify_center()
                    .gap(ui.px(8.0))
                    .child(
                        button("browser-retry", "Retry")
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.reload(cx))),
                    )
                    .when_some(external, |row, url| {
                        row.child(
                            button("browser-external", "Open in default browser")
                                .on_click(move |_, _, cx| cx.open_url(&url)),
                        )
                    }),
            )
    }
}

impl Render for BrowserView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The page is made after this frame is laid out, outside GPUI's drawing.
        if self.page.is_none() {
            cx.defer_in(window, |this, window, cx| this.open_page(window, cx));
        }
        // ⌘L's or ⌘F's selection, once the field it selects in is drawn.
        if self.select
            && (self.address.read(cx).focus_handle(cx).is_focused(window)
                || self.finding(window, cx))
        {
            self.select = false;
            cx.defer_in(window, |_, window, cx| {
                window.dispatch_action(Box::new(text_input::SelectAll), cx)
            });
        }
        let ui = UiFont::get(cx);
        div()
            .size_full()
            .flex()
            .flex_col()
            .key_context(menu::BROWSER)
            .on_action(
                cx.listener(|this, _: &menu::FocusAddress, window, cx| this.edit(true, window, cx)),
            )
            .on_action(cx.listener(|this, _: &menu::ReloadPage, _, cx| this.reload_page(cx)))
            .on_action(cx.listener(Self::open_find))
            .on_action(cx.listener(Self::find_next))
            .on_action(cx.listener(Self::find_previous))
            .child(self.toolbar(window, &ui, cx))
            .children(self.find.as_ref().map(|bar| self.find_bar(bar, &ui, cx)))
            .child(self.content(&ui, cx))
    }
}

/// The address field's colours.
fn colors(theme: &Theme) -> text_input::Colors {
    text_input::Colors {
        text: hsla(theme.fg(|t| t.agents_text), 1.0),
        placeholder: hsla(theme.fg(|t| t.agents_dimmer), 1.0),
        cursor: hsla(theme.fg(|t| t.focus), 1.0),
        selection: hsla(theme.fg(|t| t.focus), 0.3),
    }
}

/// What typing `typed` in the address field opens: as typed when it names its scheme (`…://`);
/// else `http://` before a local server's address (see `local`) and `https://` before anything
/// else; either way with an address that listens everywhere made [`reachable`]. Nothing for
/// nothing typed, or for words with spaces between them.
pub fn address(typed: &str) -> Option<String> {
    let typed = typed.trim();
    if typed.is_empty() || typed.contains(char::is_whitespace) {
        return None;
    }
    if let Some((scheme, _)) = typed.split_once("://")
        && is_scheme(scheme)
    {
        return Some(reachable(typed));
    }
    let authority = &typed[..typed.find(['/', '?', '#']).unwrap_or(typed.len())];
    let scheme = if local(host_port(authority).0) {
        "http"
    } else {
        "https"
    };
    Some(reachable(&format!("{scheme}://{typed}")))
}

/// `url` with a web page's host that listens everywhere, `0.0.0.0` or `[::]`, put as this
/// machine's loopback address, `127.0.0.1` or `[::1]`, all else kept: WebKit will not open the
/// first, and a server listening everywhere answers on the second.
pub fn reachable(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return url.to_owned();
    };
    if !(scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https")) {
        return url.to_owned();
    }
    let (authority, tail) = rest.split_at(rest.find(['/', '?', '#']).unwrap_or(rest.len()));
    let (user, host) = match authority.rsplit_once('@') {
        Some((user, host)) => (format!("{user}@"), host),
        None => (String::new(), authority),
    };
    let (host, port) = host_port(host);
    let loopback = match host.parse::<IpAddr>() {
        Ok(IpAddr::V4(ip)) if ip.is_unspecified() => "127.0.0.1",
        Ok(IpAddr::V6(ip)) if ip.is_unspecified() => "[::1]",
        _ => return url.to_owned(),
    };
    let port = port.map(|port| format!(":{port}")).unwrap_or_default();
    format!("{scheme}://{user}{loopback}{port}{tail}")
}

/// The address as the toolbar shows it while not typing: without `http://` or `https://`, or a
/// lone `/` after the host; and the part that tells pages apart, to draw bright: a local server's
/// port, else the host and its port.
fn shown(url: &str) -> (String, Range<usize>) {
    let Some(rest) = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
    else {
        return (url.to_owned(), 0..url.len());
    };
    let rest = rest
        .strip_suffix('/')
        .filter(|rest| !rest.contains(['/', '?', '#']))
        .unwrap_or(rest);
    let authority = &rest[..rest.find(['/', '?', '#']).unwrap_or(rest.len())];
    let bright = match host_port(authority) {
        (host, Some(port)) if local(host) => authority.len() - port.len()..authority.len(),
        _ => 0..authority.len(),
    };
    (rest.to_owned(), bright)
}

/// A URL scheme's name: a letter, then letters, digits, `+`, `-` or `.`.
fn is_scheme(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_alphabetic())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

/// The host in `authority` (`host[:port]`, an IPv6 host in brackets, after any user name) and its
/// port.
fn host_port(authority: &str) -> (&str, Option<&str>) {
    let authority = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    if let Some(bracketed) = authority.strip_prefix('[') {
        let (host, after) = bracketed.split_once(']').unwrap_or((bracketed, ""));
        return (host, after.strip_prefix(':'));
    }
    match authority.rsplit_once(':') {
        Some((host, port)) if port.bytes().all(|b| b.is_ascii_digit()) => (host, Some(port)),
        _ => (authority, None),
    }
}

/// An address a development server prints for itself: `localhost` or a name under it, a name on
/// the local network (`*.local`), a loopback IP, `0.0.0.0` or `::`, or a private IPv4 address on
/// the local network.
fn local(host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    host == "localhost"
        || host.ends_with(".localhost")
        || host.ends_with(".local")
        || host.parse::<IpAddr>().is_ok_and(|ip| {
            ip.is_loopback()
                || ip.is_unspecified()
                || matches!(ip, IpAddr::V4(ip) if ip.is_private())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_addresses_get_http_for_this_machine_and_https_for_anything_else() {
        // localhost, with or without a port, a path or a query, in any case, and names under it.
        assert_eq!(address("localhost").as_deref(), Some("http://localhost"));
        assert_eq!(
            address("localhost:5173/settings").as_deref(),
            Some("http://localhost:5173/settings")
        );
        assert_eq!(
            address("LocalHost:3000?tab=2").as_deref(),
            Some("http://LocalHost:3000?tab=2")
        );
        assert_eq!(
            address("localhost#top").as_deref(),
            Some("http://localhost#top")
        );
        assert_eq!(
            address("app.localhost:8080").as_deref(),
            Some("http://app.localhost:8080")
        );
        // Loopback IPs, IPv4 and IPv6.
        assert_eq!(
            address("127.0.0.1:8000").as_deref(),
            Some("http://127.0.0.1:8000")
        );
        assert_eq!(
            address("127.1.2.3/x").as_deref(),
            Some("http://127.1.2.3/x")
        );
        assert_eq!(
            address("[::1]:3000/").as_deref(),
            Some("http://[::1]:3000/")
        );
        assert_eq!(address("[::1]").as_deref(), Some("http://[::1]"));
        // What a development server prints when it listens everywhere, which WebKit will not
        // open: this machine's loopback address instead, port and path kept.
        assert_eq!(
            address("0.0.0.0:8000").as_deref(),
            Some("http://127.0.0.1:8000")
        );
        assert_eq!(
            address("0.0.0.0/app?x=1#top").as_deref(),
            Some("http://127.0.0.1/app?x=1#top")
        );
        assert_eq!(address("[::]:8000/").as_deref(), Some("http://[::1]:8000/"));
        assert_eq!(address("[::]").as_deref(), Some("http://[::1]"));
        // This machine's addresses on the local network, the private ranges.
        assert_eq!(
            address("192.168.1.20:3000").as_deref(),
            Some("http://192.168.1.20:3000")
        );
        assert_eq!(
            address("10.0.0.5:5173/app?x=1").as_deref(),
            Some("http://10.0.0.5:5173/app?x=1")
        );
        assert_eq!(
            address("172.16.0.1:8080").as_deref(),
            Some("http://172.16.0.1:8080")
        );
        assert_eq!(
            address("172.31.255.254").as_deref(),
            Some("http://172.31.255.254")
        );
        // Names on the local network.
        assert_eq!(
            address("my-mac.local:3000/").as_deref(),
            Some("http://my-mac.local:3000/")
        );
        assert_eq!(
            address("Printer.LOCAL").as_deref(),
            Some("http://Printer.LOCAL")
        );
        // Anything else, near misses too.
        assert_eq!(
            address("example.com").as_deref(),
            Some("https://example.com")
        );
        assert_eq!(
            address("example.com:8443/a").as_deref(),
            Some("https://example.com:8443/a")
        );
        assert_eq!(
            address("localhost.example.com").as_deref(),
            Some("https://localhost.example.com")
        );
        assert_eq!(
            address("127.0.0.1.nip.io").as_deref(),
            Some("https://127.0.0.1.nip.io")
        );
        assert_eq!(
            address("[2001:db8::1]:3000").as_deref(),
            Some("https://[2001:db8::1]:3000")
        );
        for public in [
            "172.15.0.1",
            "172.32.0.1",
            "192.169.1.20:3000",
            "11.0.0.1",
            "8.8.8.8",
            "0.0.0.1",
            "local",
            "my-mac.local.example.com",
            "example.localdomain",
        ] {
            assert_eq!(
                address(public),
                Some(format!("https://{public}")),
                "{public}"
            );
        }
    }

    #[test]
    fn typed_addresses_keep_their_scheme_and_lose_the_blanks_around_them() {
        assert_eq!(
            address("http://example.com/a").as_deref(),
            Some("http://example.com/a")
        );
        assert_eq!(
            address("https://localhost:3000").as_deref(),
            Some("https://localhost:3000")
        );
        assert_eq!(
            address("HTTP://Example.com").as_deref(),
            Some("HTTP://Example.com")
        );
        assert_eq!(
            address("file:///tmp/index.html").as_deref(),
            Some("file:///tmp/index.html")
        );
        // Pasted as a development server prints it: the loopback address all the same.
        assert_eq!(
            address("http://0.0.0.0:3000/").as_deref(),
            Some("http://127.0.0.1:3000/")
        );
        assert_eq!(
            address("HTTPS://user@0.0.0.0:8443/a").as_deref(),
            Some("HTTPS://user@127.0.0.1:8443/a")
        );
        assert_eq!(
            address("http://[::]:8000/x?to=http://0.0.0.0").as_deref(),
            Some("http://[::1]:8000/x?to=http://0.0.0.0")
        );
        // Only the host as a whole, and only for web pages.
        assert_eq!(
            address("http://0.0.0.0.nip.io:3000").as_deref(),
            Some("http://0.0.0.0.nip.io:3000")
        );
        assert_eq!(
            address("http://10.0.0.0:3000").as_deref(),
            Some("http://10.0.0.0:3000")
        );
        assert_eq!(
            address("ftp://0.0.0.0/file").as_deref(),
            Some("ftp://0.0.0.0/file")
        );
        // `://` further on is not a scheme.
        assert_eq!(
            address("localhost:3000/go?to=http://x").as_deref(),
            Some("http://localhost:3000/go?to=http://x")
        );
        assert_eq!(
            address("  localhost:3000 \n").as_deref(),
            Some("http://localhost:3000")
        );
        // Nothing typed, or words: nothing to open.
        assert_eq!(address("").as_deref(), None);
        assert_eq!(address("   ").as_deref(), None);
        assert_eq!(address("two words").as_deref(), None);
    }
}
