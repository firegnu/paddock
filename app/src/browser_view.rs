//! The right sidebar's Browser tab (DESIGN §13, P5-28): the toolbar from the design (back,
//! forward, reload or stop while loading, the address, open in the default browser) over the page
//! (`browser.rs`), which it lays out. Before anything is opened it shows a quiet empty state; after
//! a load fails, a quiet error with Retry in the page's place.
use crate::{
    browser::{self, Failure, Page, Scene, State},
    fonts::UiFont,
    footer_icon::{self, Icon},
    right_panel::{self, Tab, Tip},
    text_input::{self, TextInput},
    theme::Theme,
    view::hsla,
};
use gpui::{
    App, Bounds, ClickEvent, Context, Div, Entity, EventEmitter, Focusable, FontWeight,
    HighlightStyle, KeyDownEvent, MouseButton, Pixels, Render, Stateful, StyledText, Window,
    canvas, div, prelude::*, px, relative,
};
use std::{cell::Cell, net::IpAddr, ops::Range, rc::Rc, time::Duration};

/// The page area's corners; the page sits inside its 1-point rim.
const RADIUS: f32 = 8.0;
/// How often the page's state is read.
const POLL: Duration = Duration::from_millis(50);

type Pick = fn(&crate::preset::Theme) -> crate::preset::Color;

/// Sent when the address the toolbar shows changes, for the layout to keep.
pub struct Visited(pub String);

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
    /// Where the page goes as laid out this frame, until [`BrowserView::place`] takes it.
    area: Rc<Cell<Option<Bounds<Pixels>>>>,
}

impl EventEmitter<Visited> for BrowserView {}

impl BrowserView {
    /// The tab, to open `url` when it first shows.
    pub fn new(theme: Rc<Theme>, url: Option<String>, cx: &mut Context<Self>) -> Self {
        let colors = colors(&theme);
        let address = cx.new(|cx| TextInput::new("", "Enter an address", colors, cx));
        BrowserView {
            theme,
            page: None,
            state: State::default(),
            opened: url.is_some(),
            visited: url.clone(),
            pending: url,
            refused: None,
            address,
            area: Rc::default(),
        }
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
    /// open on this tab, `dragging` whether a divider is being dragged.
    pub fn place(
        &mut self,
        open: bool,
        on_browser: bool,
        dragging: bool,
        window: &Window,
        cx: &mut App,
    ) {
        let scene = Scene {
            open,
            browser: on_browser,
            area: self.area.take(),
            covers: browser::covers(window),
            dragging,
        };
        if let Some(page) = &mut self.page {
            page.place(scene.page(), window, cx);
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
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(POLL).await;
                if this.update(cx, |view, cx| view.poll(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        cx.notify();
    }

    /// Reads the page's state: drawn again when it changed, and the layout told when the address
    /// shown did.
    fn poll(&mut self, cx: &mut Context<Self>) {
        let Some(page) = &self.page else {
            return;
        };
        let state = page.state();
        if state != self.state {
            self.state = state;
            cx.notify();
        }
        if let Some(url) = self.shown_url()
            && self.visited.as_deref() != Some(url)
        {
            let url = url.to_owned();
            self.visited = Some(url.clone());
            cx.emit(Visited(url));
        }
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
                    });
                }
            }
            None => self.pending = Some(url),
        }
        cx.notify();
    }

    /// A click on the address: the field takes the keyboard, holding the address shown.
    fn edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.shown_url().unwrap_or_default().to_owned();
        self.address
            .update(cx, |input, cx| input.set_text(text, cx));
        let focus = self.address.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        cx.notify();
    }

    /// ⏎ in the address field: opens what was typed, and the page takes the keyboard.
    fn commit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let typed = self.address.read(cx).text().to_owned();
        let Some(url) = address(&typed) else {
            return;
        };
        window.blur(cx);
        self.go(url, cx);
        // After this key: AppKit asks GPUI's view about its text as it gives the keyboard up.
        cx.defer_in(window, |this, _, _| {
            if let Some(page) = &this.page {
                page.focus();
            }
        });
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

    /// Back, forward, reload or stop, the address, and open in the default browser.
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
        let reloads = self.page.is_some() && self.opened;
        let opens = self.shown_url().is_some();
        let tip = Tip::new("Open in default browser", theme, ui);
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
            .child(
                button("browser-open", Icon::External, opens).when(opens, |b| {
                    b.tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
                        .on_click(on_open)
                }),
            )
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
                    "escape" => {
                        window.blur(cx);
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
                    cx.listener(|this, _, window, cx| this.edit(window, cx)),
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

    /// Under the toolbar: the empty state before anything is opened, the error after a failed
    /// load, else the area the page shows in, a faint rim round it.
    fn content(&self, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let theme = &*self.theme;
        if !self.opened {
            return right_panel::placeholder(theme, ui, Tab::Browser);
        }
        if let Some(failure) = self.failure() {
            return self.failed(failure, ui, cx);
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
            .child(canvas(move |bounds, _, _| area.set(Some(bounds)), |_, _, _, _| {}).size_full())
    }

    /// The error in the page's place: a line saying so, the system's reason, and Retry.
    fn failed(&self, failure: &Failure, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let theme = &*self.theme;
        let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
        let highlight = hsla(theme.bg(|t| t.agent_selected), 1.0);
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
                    .child(failure.reason.clone()),
            )
            .child(
                div()
                    .id("browser-retry")
                    .mt(ui.px(8.0))
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
                    .child("Retry")
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.reload(cx))),
            )
    }
}

impl Render for BrowserView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The page is made after this frame is laid out, outside GPUI's drawing.
        if self.page.is_none() {
            cx.defer_in(window, |this, window, cx| this.open_page(window, cx));
        }
        let ui = UiFont::get(cx);
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(self.toolbar(window, &ui, cx))
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
/// else `http://` before a local server's address (localhost or a loopback IP) and `https://`
/// before anything else. Nothing for nothing typed, or for words with spaces between them.
pub fn address(typed: &str) -> Option<String> {
    let typed = typed.trim();
    if typed.is_empty() || typed.contains(char::is_whitespace) {
        return None;
    }
    if let Some((scheme, _)) = typed.split_once("://")
        && is_scheme(scheme)
    {
        return Some(typed.to_owned());
    }
    let authority = &typed[..typed.find(['/', '?', '#']).unwrap_or(typed.len())];
    let scheme = if local(host_port(authority).0) {
        "http"
    } else {
        "https"
    };
    Some(format!("{scheme}://{typed}"))
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

/// A name for this machine: `localhost` or a name under it, or a loopback IP.
fn local(host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    host == "localhost"
        || host.ends_with(".localhost")
        || host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
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
            address("192.168.1.20:3000").as_deref(),
            Some("https://192.168.1.20:3000")
        );
        assert_eq!(
            address("[2001:db8::1]:3000").as_deref(),
            Some("https://[2001:db8::1]:3000")
        );
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
