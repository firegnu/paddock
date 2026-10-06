//! The right sidebar's Browser page (DESIGN §13, P5-28): the system's `WKWebView`, one per window,
//! an AppKit view beside GPUI's and over it, as the sidebar material is beside it under it
//! (`frost.rs`). GPUI cannot draw over it, so the page shows only where the Browser tab lays it
//! out, and only while nothing GPUI draws over the window meets it ([`Scene`]): every popup, menu
//! and hover text notes itself with [`cover`]. Hidden, it keeps its page and its history. It keeps
//! website data as the system keeps it for paddock, so sign-ins last.
//!
//! The keyboard is AppKit's to give to the page and GPUI's to give within paddock; [`Keys`] keeps
//! one record of who has it and brings the two in line, and [`route`] says where a shortcut goes.
use crate::menu;
use gpui::{
    Bounds, IntoElement, Keystroke, Modifiers, Pixels, Styled, Window, WindowId, canvas, px,
};
use objc2::{
    ClassType, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send,
    rc::{Retained, Weak},
    runtime::{AnyObject, NSObject, NSObjectProtocol, ProtocolObject},
    sel,
};
use objc2_app_kit::{
    NSApplication, NSEvent, NSEventModifierFlags, NSResponder, NSView, NSWindowOrderingMode,
};
use objc2_foundation::{NSError, NSPoint, NSRect, NSSize, NSString, NSURL, NSURLRequest};
use objc2_web_kit::{
    WKNavigation, WKNavigationDelegate, WKWebView, WKWebViewConfiguration, WKWebsiteDataStore,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{cell::RefCell, collections::HashMap};

/// What decides where the page shows in a frame, if anywhere.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scene {
    /// The right sidebar is open.
    pub open: bool,
    /// It is on the Browser tab.
    pub browser: bool,
    /// Where the tab lays the page out this frame; none while it shows its empty or failed state.
    pub area: Option<Bounds<Pixels>>,
    /// What GPUI draws over the window this frame: popups, menus, hover texts.
    pub covers: Vec<Bounds<Pixels>>,
    /// A divider is being dragged, which the page would take the mouse from.
    pub dragging: bool,
}

impl Scene {
    /// Where the page shows: its area, while the sidebar is open on Browser, nothing is being
    /// dragged and nothing drawn over the window meets it; else nowhere.
    pub fn page(&self) -> Option<Bounds<Pixels>> {
        let area = self.area?;
        let shown = self.open
            && self.browser
            && !self.dragging
            && area.size.width > px(0.0)
            && area.size.height > px(0.0)
            && !self.covers.iter().any(|cover| cover.intersects(&area));
        shown.then_some(area)
    }
}

thread_local! {
    /// What each window's overlays noted while its frame was laid out (see [`cover`]).
    static COVERS: RefCell<HashMap<WindowId, Vec<Bounds<Pixels>>>> = RefCell::default();
}

/// An empty layer over its parent that notes the parent as drawn over the window, so the page
/// keeps out of its way: one in every popup, menu and hover text that can meet the page.
pub fn cover() -> impl IntoElement {
    canvas(
        |bounds, window, _| {
            let id = Window::window_handle(window).window_id();
            COVERS.with_borrow_mut(|covers| covers.entry(id).or_default().push(bounds));
        },
        |_, _, _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// What overlays noted while `window`'s frame was laid out, taken, so the next frame starts
/// afresh. Read when painting: hover texts are laid out after everything else.
pub fn covers(window: &Window) -> Vec<Bounds<Pixels>> {
    let id = Window::window_handle(window).window_id();
    COVERS.with_borrow_mut(|covers| covers.remove(&id).unwrap_or_default())
}

/// Where `bounds`, in points from the top-left corner of GPUI's view, lies in that view's parent:
/// `host` is GPUI's view's frame there, and the parent counts up from its bottom edge unless
/// `flipped`. Both sides count points, so the screen's scale does not come into it.
pub fn appkit_frame(bounds: Bounds<Pixels>, host: NSRect, flipped: bool) -> NSRect {
    let points = |pixels: Pixels| f64::from(f32::from(pixels));
    let (left, top) = (points(bounds.origin.x), points(bounds.origin.y));
    let (width, height) = (points(bounds.size.width), points(bounds.size.height));
    let y = if flipped {
        host.origin.y + top
    } else {
        host.origin.y + host.size.height - top - height
    };
    NSRect::new(
        NSPoint::new(host.origin.x + left, y),
        NSSize::new(width, height),
    )
}

/// Who has the keyboard in a window: a pane or anything else of paddock's, the Browser's address
/// field, or its page.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Owner {
    #[default]
    Paddock,
    Address,
    Page,
}

/// How the Browser's page is doing, as far as the keyboard goes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Showing {
    Shown,
    /// The Browser tab shows, the page does not for now: something is drawn over it, a divider
    /// is being dragged, or the tab shows its empty or error state.
    Hidden,
    /// The right sidebar is closed or on another tab.
    #[default]
    Away,
}

/// The window as the record finds it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seen {
    /// What GPUI's focus is on: the page's focus handle, the address field, or else paddock's.
    pub focus: Owner,
    /// AppKit's keyboard is in the page.
    pub native: bool,
    pub showing: Showing,
    /// A popup, menu or palette is open, with keys of its own.
    pub dialog: bool,
}

/// What brings AppKit's keyboard and GPUI's focus in line with the record.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Moves {
    /// AppKit's keyboard to the page (`true`) or back to GPUI's view (`false`).
    pub native: Option<bool>,
    /// GPUI's focus to the page's focus handle.
    pub focus_page: bool,
    /// The Browser went away with the keyboard: GPUI's focus to the active pane.
    pub to_pane: bool,
    /// The page was clicked while a popup was open: the popup closes.
    pub dismiss: bool,
}

/// The one record of who has a window's keyboard. Brought up to date each frame and while the page
/// is polled ([`Keys::settle`]): a click on the page shows as AppKit's keyboard moving into it, a
/// click on a pane, the address field, ⌘L, switching panes or tabs as GPUI's focus moving, closing
/// the right sidebar or switching its tab as the Browser going [`Showing::Away`], and anything
/// drawn over the page as it hiding for a while.
#[derive(Clone, Debug, Default)]
pub struct Keys {
    owner: Owner,
    /// GPUI's focus and AppKit's keyboard as the last settle left them, to tell which one moved.
    last: Option<(Owner, bool)>,
    /// A popup was open then.
    dialog: bool,
}

impl Keys {
    pub fn owner(&self) -> Owner {
        self.owner
    }

    /// Takes in what moved since the last time and says what to move so AppKit and GPUI agree:
    /// the page has AppKit's keyboard exactly when it has the keyboard, shows, and no popup is
    /// open; it has it back once it shows again, or the popup that borrowed it closes.
    pub fn settle(&mut self, seen: Seen) -> Moves {
        let (focus, native) = self.last.unwrap_or((seen.focus, seen.native));
        let mut moves = Moves::default();
        if seen.native && !native {
            // AppKit gave the page the keyboard: it was clicked. GPUI's focus follows, and a popup
            // open beside the page closes, as a click anywhere else outside it would close it.
            self.owner = Owner::Page;
            moves.focus_page = seen.focus != Owner::Page;
            moves.dismiss = seen.dialog;
        } else if seen.focus != focus {
            // GPUI's focus moved and the keyboard with it, but for a popup's field, which only
            // borrows it from the page.
            if !(seen.dialog && seen.focus == Owner::Paddock && self.owner == Owner::Page) {
                self.owner = seen.focus;
            }
        } else if self.dialog
            && !seen.dialog
            && self.owner == Owner::Page
            && seen.focus != Owner::Page
        {
            // The popup closed into something of paddock's rather than giving the page its keys.
            self.owner = Owner::Paddock;
        }
        let dialog = seen.dialog && !moves.dismiss;
        let mut focus = if moves.focus_page {
            Owner::Page
        } else {
            seen.focus
        };
        if seen.showing == Showing::Away && self.owner != Owner::Paddock {
            // The Browser went away: the active pane has the keyboard, unless something else of
            // paddock's already does.
            self.owner = Owner::Paddock;
            if focus != Owner::Paddock && !dialog {
                moves.to_pane = true;
                focus = Owner::Paddock;
            }
        }
        let page = self.owner == Owner::Page
            && focus == Owner::Page
            && seen.showing == Showing::Shown
            && !dialog;
        if page != seen.native {
            moves.native = Some(page);
        }
        self.dialog = dialog;
        self.last = Some((focus, page));
        moves
    }
}

/// Where a shortcut goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    /// One of paddock's menu commands: the window or the app runs it, whoever has the keyboard.
    Paddock,
    /// ⌘L or ⌘R while the Browser has the keyboard: its address field, or loading the page again.
    Browser,
    /// What has the keyboard: copying, pasting and the other standard edits, finding, and any
    /// shortcut paddock does not bind, such as a page's own.
    Keyboard(Owner),
}

/// Where `keystroke` goes while `owner` has the keyboard, by paddock's shortcuts (`menu.rs`).
pub fn route(owner: Owner, keystroke: &Keystroke) -> Route {
    let modifiers = |m: &Modifiers| (m.control, m.alt, m.shift, m.platform);
    for binding in menu::bindings() {
        let [bound] = binding.keystrokes() else {
            continue;
        };
        let bound = bound.inner();
        if bound.key != keystroke.key
            || modifiers(&bound.modifiers) != modifiers(&keystroke.modifiers)
        {
            continue;
        }
        let action = binding.action();
        if binding.predicate().is_none() {
            return if menu::follows_keyboard(action) {
                Route::Keyboard(owner)
            } else {
                Route::Paddock
            };
        }
        if menu::browser(action) && owner != Owner::Paddock {
            return Route::Browser;
        }
    }
    Route::Keyboard(owner)
}

/// A standard edit, as the Edit menu of a Mac app sends it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edit {
    Copy,
    Paste,
    Cut,
    SelectAll,
    Undo,
    Redo,
}

/// The standard edit `keystroke` stands for: ⌘C, ⌘V, ⌘X, ⌘A, ⌘Z and ⇧⌘Z.
pub fn edit(keystroke: &Keystroke) -> Option<Edit> {
    let held = &keystroke.modifiers;
    if !held.platform || held.control || held.alt || held.function {
        return None;
    }
    Some(match (keystroke.key.as_str(), held.shift) {
        ("c", false) => Edit::Copy,
        ("v", false) => Edit::Paste,
        ("x", false) => Edit::Cut,
        ("a", false) => Edit::SelectAll,
        ("z", false) => Edit::Undo,
        ("z", true) => Edit::Redo,
        _ => return None,
    })
}

/// GPUI's name for the key that types `chars`: the character, lower case, or the name of a key
/// that types none (`enter`, `up`); none for anything else.
pub fn key_name(chars: &str) -> Option<String> {
    let mut each = chars.chars();
    let (Some(char), None) = (each.next(), each.next()) else {
        return None;
    };
    let name = match char {
        '\r' | '\u{3}' => "enter",
        '\t' | '\u{19}' => "tab",
        '\u{1b}' => "escape",
        ' ' => "space",
        '\u{7f}' => "backspace",
        // AppKit's function-key characters.
        '\u{f700}' => "up",
        '\u{f701}' => "down",
        '\u{f702}' => "left",
        '\u{f703}' => "right",
        char if char.is_control() || ('\u{f700}'..='\u{f8ff}').contains(&char) => return None,
        char => return Some(char.to_lowercase().collect()),
    };
    Some(name.to_owned())
}

/// Why the page did not load.
#[derive(Clone, Debug, PartialEq)]
pub struct Failure {
    /// The address that failed, when known.
    pub url: Option<String>,
    /// What the system says went wrong, a sentence.
    pub reason: String,
}

/// The page as the toolbar shows it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// Its address now, after any redirects; none before it has loaded anything.
    pub url: Option<String>,
    pub back: bool,
    pub forward: bool,
    pub loading: bool,
    /// How far the load has got, from 0 to 1.
    pub progress: f64,
    /// Why the last load failed, until the next one starts.
    pub failure: Option<Failure>,
}

/// `NSURLErrorCancelled`: a load stopped, or given up for another.
const CANCELLED: isize = -999;
/// `WebKitErrorFrameLoadInterruptedByPolicyChange`: a load the page itself turned into something
/// else, such as a download.
const INTERRUPTED: isize = 102;

/// What the navigation delegate has heard.
#[derive(Default)]
struct Heard {
    /// Where the load under way set out for.
    attempt: Option<String>,
    failure: Option<Failure>,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and this has no `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "PaddockPageDelegate"]
    #[ivars = RefCell<Heard>]
    struct Delegate;

    unsafe impl NSObjectProtocol for Delegate {}

    // SAFETY: each method has the signature WKNavigationDelegate gives it.
    unsafe impl WKNavigationDelegate for Delegate {
        #[unsafe(method(webView:didStartProvisionalNavigation:))]
        fn did_start(&self, view: &WKWebView, _navigation: Option<&WKNavigation>) {
            let mut heard = self.ivars().borrow_mut();
            heard.attempt = url_of(view);
            heard.failure = None;
        }

        #[unsafe(method(webView:didFailProvisionalNavigation:withError:))]
        fn did_fail_to_start(
            &self,
            _view: &WKWebView,
            _navigation: Option<&WKNavigation>,
            error: &NSError,
        ) {
            self.failed(error);
        }

        #[unsafe(method(webView:didFailNavigation:withError:))]
        fn did_fail(&self, _view: &WKWebView, _navigation: Option<&WKNavigation>, error: &NSError) {
            self.failed(error);
        }

        #[unsafe(method(webViewWebContentProcessDidTerminate:))]
        fn did_terminate(&self, view: &WKWebView) {
            self.ivars().borrow_mut().failure = Some(Failure {
                url: url_of(view),
                reason: "The page stopped unexpectedly.".into(),
            });
        }
    }
);

define_class!(
    // SAFETY: WKWebView may be subclassed; this only puts paddock's shortcuts and the standard
    // edits in front of and behind WebKit's own key handling, and has no `Drop`.
    #[unsafe(super(WKWebView, NSView, NSResponder, NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "PaddockWebView"]
    #[ivars = Weak<NSView>]
    struct WebView;

    impl WebView {
        /// A key with ⌘ or another key equivalent. AppKit asks each view in the window in turn,
        /// GPUI's (the ivar) as well as this one, whichever has the keyboard. While the page has
        /// it, paddock's own shortcuts go to GPUI's view and nowhere else, so the page never sees
        /// them and they run once; everything else goes to the page first. A standard edit the
        /// page passes on comes back here as WebKit sends the key round again, and is sent to the
        /// page as the Edit menu would send it.
        #[unsafe(method(performKeyEquivalent:))]
        fn perform_key_equivalent(&self, event: &NSEvent) -> bool {
            self.key_equivalent(event)
        }
    }
);

impl WebView {
    /// See `performKeyEquivalent:` above.
    fn key_equivalent(&self, event: &NSEvent) -> bool {
        let holds = holds_keys(self);
        let keystroke = keystroke(event);
        if holds
            && let Some(keystroke) = &keystroke
            && route(Owner::Page, keystroke) != Route::Keyboard(Owner::Page)
        {
            if let Some(gpui) = self.ivars().load() {
                gpui.performKeyEquivalent(event);
            }
            return true;
        }
        // SAFETY: WebKit's own, which gives the page the key first.
        if unsafe { msg_send![super(self), performKeyEquivalent: event] } {
            return true;
        }
        holds
            && keystroke
                .as_ref()
                .and_then(edit)
                .is_some_and(|edit| self.edit(edit))
    }

    /// Sends `edit` to the page, or what in it has the keyboard; false when nothing took it.
    fn edit(&self, edit: Edit) -> bool {
        let action = match edit {
            Edit::Copy => sel!(copy:),
            Edit::Paste => sel!(paste:),
            Edit::Cut => sel!(cut:),
            Edit::SelectAll => sel!(selectAll:),
            Edit::Undo | Edit::Redo => return self.undo(edit == Edit::Redo),
        };
        let app = NSApplication::sharedApplication(self.mtm());
        // SAFETY: a standard action, sent up the responder chain from the page as from the menu.
        unsafe { app.sendAction_to_from(action, None, Some(self.as_ref())) }
    }

    /// Undoes (or redoes) the page's last edit, which WebKit keeps with the window's undo manager;
    /// false when there is none. Sent by name rather than bringing in Foundation's undo manager
    /// for four calls.
    fn undo(&self, redo: bool) -> bool {
        // SAFETY: NSResponder's `undoManager`, an NSUndoManager or nil, and its methods.
        unsafe {
            let manager: Option<Retained<AnyObject>> = msg_send![self, undoManager];
            let Some(manager) = manager else {
                return false;
            };
            let can: bool = if redo {
                msg_send![&*manager, canRedo]
            } else {
                msg_send![&*manager, canUndo]
            };
            if can {
                if redo {
                    let () = msg_send![&*manager, redo];
                } else {
                    let () = msg_send![&*manager, undo];
                }
            }
            can
        }
    }
}

/// What a key event is, as paddock's shortcuts are written: the key ⌘ makes of it, as a menu
/// matches its shortcuts (so ⇧ and ⌥ do not change the letter), and the modifiers held.
fn keystroke(event: &NSEvent) -> Option<Keystroke> {
    let held = event.modifierFlags();
    let chars = event.charactersByApplyingModifiers(NSEventModifierFlags::Command)?;
    Some(Keystroke {
        modifiers: Modifiers {
            control: held.contains(NSEventModifierFlags::Control),
            alt: held.contains(NSEventModifierFlags::Option),
            shift: held.contains(NSEventModifierFlags::Shift),
            platform: held.contains(NSEventModifierFlags::Command),
            function: false,
        },
        key: key_name(&chars.to_string())?,
        key_char: None,
    })
}

impl Delegate {
    fn new(main: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(main).set_ivars(RefCell::default());
        // SAFETY: NSObject's `init`.
        unsafe { msg_send![super(this), init] }
    }

    /// A load failed: noted, unless it was only stopped or gave way to another.
    fn failed(&self, error: &NSError) {
        let (domain, code) = (error.domain().to_string(), error.code());
        if (domain == "NSURLErrorDomain" && code == CANCELLED)
            || (domain == "WebKitErrorDomain" && code == INTERRUPTED)
        {
            return;
        }
        let mut heard = self.ivars().borrow_mut();
        let url = heard.attempt.clone();
        heard.failure = Some(Failure {
            url,
            reason: error.localizedDescription().to_string(),
        });
    }
}

/// The page: the web view, owned here besides its place in the window's view, and what its
/// navigation delegate hears. Only made and used on the main thread.
pub struct Page {
    view: Retained<WebView>,
    /// Kept here: the web view only refers to it.
    delegate: Retained<Delegate>,
    /// Where it shows; none while hidden.
    shown: Option<NSRect>,
}

impl Page {
    /// A blank page over `window`'s GPUI view, hidden until [`Page::place`] puts it somewhere, its
    /// corners rounded `radius` points; none off the main thread or without an AppKit view to put
    /// it beside. Making it does not bring paddock to the front.
    pub fn new(window: &Window, radius: f64) -> Option<Self> {
        let main = MainThreadMarker::new()?;
        let gpui = gpui_view(window)?;
        // SAFETY: a plain getter; GPUI puts its view in the window's content view.
        let parent = unsafe { gpui.superview() }?;
        // SAFETY: configured on the main thread before the view is made from it; the default
        // store is the persistent one. WKWebView's designated initializer.
        let view: Retained<WebView> = unsafe {
            let config = WKWebViewConfiguration::new(main);
            config.setWebsiteDataStore(&WKWebsiteDataStore::defaultDataStore(main));
            let view = WebView::alloc(main).set_ivars(Weak::from_retained(&gpui));
            msg_send![super(view), initWithFrame: NSRect::ZERO, configuration: &*config]
        };
        let delegate = Delegate::new(main);
        // SAFETY: the delegate is kept as long as the view (see `Drop`).
        unsafe { view.setNavigationDelegate(Some(ProtocolObject::from_ref(&*delegate))) };
        view.setHidden(true);
        round(&view, radius);
        parent.addSubview_positioned_relativeTo(&view, NSWindowOrderingMode::Above, Some(&gpui));
        Some(Self {
            view,
            delegate,
            shown: None,
        })
    }

    /// Opens `url`; false when it is not an address the system takes.
    pub fn load(&self, url: &str) -> bool {
        let Some(url) = NSURL::URLWithString(&NSString::from_str(url)) else {
            return false;
        };
        // SAFETY: a plain load on the main thread.
        unsafe { self.view.loadRequest(&NSURLRequest::requestWithURL(&url)) };
        true
    }

    pub fn back(&self) {
        // SAFETY: on the main thread; nothing when there is nothing back.
        unsafe { self.view.goBack() };
    }

    pub fn forward(&self) {
        // SAFETY: as `back`.
        unsafe { self.view.goForward() };
    }

    pub fn reload(&self) {
        // SAFETY: on the main thread.
        unsafe { self.view.reload() };
    }

    pub fn stop(&self) {
        // SAFETY: on the main thread.
        unsafe { self.view.stopLoading() };
    }

    /// Its state now.
    pub fn state(&self) -> State {
        // SAFETY: plain getters on the main thread.
        unsafe {
            State {
                url: url_of(&self.view),
                back: self.view.canGoBack(),
                forward: self.view.canGoForward(),
                loading: self.view.isLoading(),
                progress: self.view.estimatedProgress(),
                failure: self.delegate.ivars().borrow().failure.clone(),
            }
        }
    }

    /// Shows the page at `bounds` in `window`, or hides it (`None`) with what it shows kept. A
    /// page hidden with the keyboard gives it back as the window's record settles ([`Keys`]).
    pub fn place(&mut self, bounds: Option<Bounds<Pixels>>, window: &Window) {
        let frame = bounds.and_then(|bounds| {
            let gpui = gpui_view(window)?;
            // SAFETY: as in `new`.
            let parent = unsafe { gpui.superview() }?;
            Some(appkit_frame(bounds, gpui.frame(), parent.isFlipped()))
        });
        if frame == self.shown {
            return;
        }
        match frame {
            Some(frame) => {
                self.view.setFrame(frame);
                self.view.setHidden(false);
            }
            None => self.view.setHidden(true),
        }
        self.shown = frame;
    }

    /// Gives the page the keyboard, when it shows.
    pub fn focus(&self) {
        if self.shown.is_none() {
            return;
        }
        if let Some(window) = self.view.window() {
            window.makeFirstResponder(Some(&self.view));
        }
    }

    /// The page, or something in it, has the keyboard.
    pub fn has_keys(&self) -> bool {
        holds_keys(&self.view)
    }
}

/// `view`, or something in it, has its window's keyboard.
fn holds_keys(view: &NSView) -> bool {
    let responder = view.window().and_then(|window| window.firstResponder());
    responder
        .and_then(|responder| responder.downcast::<NSView>().ok())
        .is_some_and(|responder| responder.isDescendantOf(view))
}

/// The Browser's page in `window`, or something in it, has the keyboard: what GPUI's view gets of
/// a shortcut then is not for paddock's pane.
pub fn page_has_keys(window: &Window) -> bool {
    let responder = gpui_view(window)
        .and_then(|gpui| gpui.window())
        .and_then(|native| native.firstResponder());
    let mut view = responder.and_then(|responder| responder.downcast::<NSView>().ok());
    while let Some(each) = view {
        if each.isKindOfClass(WebView::class()) {
            return true;
        }
        // SAFETY: a plain getter.
        view = unsafe { each.superview() };
    }
    false
}

impl Drop for Page {
    fn drop(&mut self) {
        // SAFETY: on the main thread, where it was made.
        unsafe {
            self.view.setNavigationDelegate(None);
            self.view.stopLoading();
        }
        // The window's view lets go of it too; a window already closed has nothing to remove from.
        self.view.removeFromSuperview();
    }
}

/// Gives the keyboard to GPUI's view in `window` when anything else has it, as the page does once
/// clicked: done before GPUI's focus moves whenever paddock takes the keyboard back.
pub fn take_keys(window: &Window) {
    let Some(gpui) = gpui_view(window) else {
        return;
    };
    let Some(native) = gpui.window() else {
        return;
    };
    let gpui: &NSResponder = &gpui;
    let has = native
        .firstResponder()
        .is_some_and(|responder| std::ptr::eq(&*responder, gpui));
    if !has {
        native.makeFirstResponder(Some(gpui));
    }
}

/// GPUI's AppKit view for `window`.
fn gpui_view(window: &Window) -> Option<Retained<NSView>> {
    let handle = HasWindowHandle::window_handle(window).ok()?;
    let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
        return None;
    };
    // SAFETY: GPUI's live NSView, retained on the main thread while `window` is borrowed.
    unsafe { Retained::retain(appkit.ns_view.as_ptr().cast::<NSView>()) }
}

/// The web view's address now.
fn url_of(view: &WKWebView) -> Option<String> {
    // SAFETY: a plain getter on the main thread.
    let url = unsafe { view.URL() }?;
    url.absoluteString().map(|text| text.to_string())
}

/// Rounds the page's corners `radius` points, clipping what it draws to them.
fn round(view: &WKWebView, radius: f64) {
    view.setWantsLayer(true);
    // SAFETY: `layer` is NSView's, a CALayer once the view wants one, and the two setters are
    // CALayer's; sent by name rather than bringing in Core Animation's bindings for two calls.
    unsafe {
        let layer: Option<Retained<AnyObject>> = msg_send![view, layer];
        if let Some(layer) = layer {
            let () = msg_send![&*layer, setCornerRadius: radius];
            let () = msg_send![&*layer, setMasksToBounds: true];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{point, size};

    fn rect(x: f32, y: f32, width: f32, height: f32) -> Bounds<Pixels> {
        Bounds::new(point(px(x), px(y)), size(px(width), px(height)))
    }

    fn appkit(x: f64, y: f64, width: f64, height: f64) -> NSRect {
        NSRect::new(NSPoint::new(x, y), NSSize::new(width, height))
    }

    /// The Browser tab open in a 1440 by 900 window, its page area at the right.
    fn browsing() -> Scene {
        Scene {
            open: true,
            browser: true,
            area: Some(rect(1012.0, 128.0, 400.0, 720.0)),
            covers: Vec::new(),
            dragging: false,
        }
    }

    #[test]
    fn the_page_shows_in_its_area_while_the_sidebar_is_open_on_browser() {
        let area = browsing().area;
        assert_eq!(browsing().page(), area);
        // Closed, on Changes, or showing its empty or failed state: nowhere.
        let closed = Scene {
            open: false,
            ..browsing()
        };
        assert_eq!(closed.page(), None);
        let changes = Scene {
            browser: false,
            ..browsing()
        };
        assert_eq!(changes.page(), None);
        let empty = Scene {
            area: None,
            ..browsing()
        };
        assert_eq!(empty.page(), None);
        // A content area with no room.
        let squashed = Scene {
            area: Some(rect(1012.0, 128.0, 400.0, 0.0)),
            ..browsing()
        };
        assert_eq!(squashed.page(), None);
        // Closed and on Changes at once.
        let both = Scene {
            open: false,
            browser: false,
            ..browsing()
        };
        assert_eq!(both.page(), None);
    }

    #[test]
    fn the_page_hides_while_anything_drawn_over_the_window_meets_it_or_a_divider_moves() {
        let area = browsing().area;
        // The palette dims the whole window.
        let palette = Scene {
            covers: vec![rect(0.0, 0.0, 1440.0, 900.0)],
            ..browsing()
        };
        assert_eq!(palette.page(), None);
        // The sidebar's menu, low on the left, leaves it be.
        let menu = Scene {
            covers: vec![rect(12.0, 600.0, 252.0, 280.0)],
            ..browsing()
        };
        assert_eq!(menu.page(), area);
        // A hover text hanging into its top hides it; one ending on its edge does not.
        let tip = Scene {
            covers: vec![rect(1300.0, 110.0, 150.0, 24.0)],
            ..browsing()
        };
        assert_eq!(tip.page(), None);
        let above = Scene {
            covers: vec![rect(1300.0, 104.0, 150.0, 24.0)],
            ..browsing()
        };
        assert_eq!(above.page(), area);
        // Any one of several is enough.
        let several = Scene {
            covers: vec![
                rect(12.0, 600.0, 252.0, 280.0),
                rect(900.0, 500.0, 200.0, 40.0),
            ],
            ..browsing()
        };
        assert_eq!(several.page(), None);
        // Dragging a divider, with or without anything over the window.
        let dragging = Scene {
            dragging: true,
            ..browsing()
        };
        assert_eq!(dragging.page(), None);
        let menu_dragging = Scene {
            dragging: true,
            ..menu.clone()
        };
        assert_eq!(menu_dragging.page(), None);
        // A cover is no reason once the sidebar is closed anyway.
        let closed = Scene {
            open: false,
            ..menu
        };
        assert_eq!(closed.page(), None);
    }

    #[test]
    fn gpui_points_turn_into_the_parent_views_points() {
        let bounds = rect(1012.0, 128.0, 400.0, 720.0);
        // The usual: GPUI's view fills an unflipped parent, which counts up from the bottom.
        let host = appkit(0.0, 0.0, 1440.0, 900.0);
        assert_eq!(
            appkit_frame(bounds, host, false),
            appkit(1012.0, 52.0, 400.0, 720.0)
        );
        // A flipped parent counts down, as GPUI does.
        assert_eq!(
            appkit_frame(bounds, host, true),
            appkit(1012.0, 128.0, 400.0, 720.0)
        );
        // GPUI's view set in from the parent's corner.
        let inset = appkit(20.0, 10.0, 1440.0, 900.0);
        assert_eq!(
            appkit_frame(bounds, inset, false),
            appkit(1032.0, 62.0, 400.0, 720.0)
        );
        assert_eq!(
            appkit_frame(bounds, inset, true),
            appkit(1032.0, 138.0, 400.0, 720.0)
        );
        // Fractions of a point stay as they are.
        assert_eq!(
            appkit_frame(rect(10.5, 20.25, 100.0, 50.5), host, false),
            appkit(10.5, 829.25, 100.0, 50.5)
        );
    }

    /// A window for the record: GPUI's focus and AppKit's keyboard as they are, moved as the
    /// record says, as `BrowserView` and the window move them.
    struct Desk {
        keys: Keys,
        focus: Owner,
        native: bool,
        showing: Showing,
        dialog: bool,
    }

    impl Desk {
        /// The terminal has the keyboard; the Browser tab shows its page.
        fn new() -> Self {
            let mut desk = Desk {
                keys: Keys::default(),
                focus: Owner::Paddock,
                native: false,
                showing: Showing::Shown,
                dialog: false,
            };
            assert_eq!(desk.settle(), Moves::default());
            desk
        }

        fn settle(&mut self) -> Moves {
            let moves = self.keys.settle(Seen {
                focus: self.focus,
                native: self.native,
                showing: self.showing,
                dialog: self.dialog,
            });
            if let Some(native) = moves.native {
                self.native = native;
            }
            if moves.focus_page {
                self.focus = Owner::Page;
            }
            if moves.to_pane {
                self.focus = Owner::Paddock;
            }
            if moves.dismiss {
                // The window closes the popup, giving the page its keys back.
                self.close_popup();
            }
            moves
        }

        fn owner(&self) -> Owner {
            self.keys.owner()
        }

        /// A click on the page: AppKit gives it the keyboard.
        fn click_page(&mut self) -> Moves {
            self.native = true;
            self.settle()
        }

        /// GPUI's focus moves to `focus`: a click on a pane, switching panes or tabs.
        fn focus(&mut self, focus: Owner) -> Moves {
            self.focus = focus;
            self.settle()
        }

        /// A click on the address field, or ⌘L: AppKit's keyboard first, then GPUI's focus.
        fn edit(&mut self) -> Moves {
            self.native = false;
            self.focus(Owner::Address)
        }

        fn show(&mut self, showing: Showing) -> Moves {
            self.showing = showing;
            self.settle()
        }

        /// A popup opens, its field taking GPUI's focus or not.
        fn open_popup(&mut self, field: bool) -> Moves {
            self.dialog = true;
            if field {
                self.focus = Owner::Paddock;
            }
            self.settle()
        }

        /// The popup closes: back to the page if it had the keyboard, else to the pane.
        fn close_popup(&mut self) {
            self.dialog = false;
            self.focus = if self.keys.owner() == Owner::Page {
                Owner::Page
            } else {
                Owner::Paddock
            };
        }
    }

    fn native(page: bool) -> Moves {
        Moves {
            native: Some(page),
            ..Moves::default()
        }
    }

    #[test]
    fn the_keyboard_follows_clicks_between_the_page_the_terminal_and_the_address_field() {
        let mut desk = Desk::new();
        assert_eq!(desk.owner(), Owner::Paddock);
        // A click on the page: GPUI's focus follows AppKit's keyboard into it.
        let clicked = desk.click_page();
        assert_eq!(desk.owner(), Owner::Page);
        assert_eq!(
            clicked,
            Moves {
                focus_page: true,
                ..Moves::default()
            }
        );
        // Nothing moves until something does.
        assert_eq!(desk.settle(), Moves::default());
        assert_eq!(desk.settle(), Moves::default());
        // A click on the terminal: AppKit's keyboard follows GPUI's focus out of the page.
        assert_eq!(desk.focus(Owner::Paddock), native(false));
        assert_eq!(desk.owner(), Owner::Paddock);
        // Into the page again, then the address field, which took AppKit's keyboard first.
        desk.click_page();
        assert_eq!(desk.edit(), Moves::default());
        assert_eq!(desk.owner(), Owner::Address);
        assert!(!desk.native);
        // A click on the page while typing an address: the field loses its cursor.
        let clicked = desk.click_page();
        assert_eq!(desk.owner(), Owner::Page);
        assert!(clicked.focus_page && desk.focus == Owner::Page);
        // ⌘L from the page, then ⏎ (or Esc): the page has the keyboard again.
        desk.edit();
        assert_eq!(desk.owner(), Owner::Address);
        assert_eq!(desk.focus(Owner::Page), native(true));
        assert_eq!(desk.owner(), Owner::Page);
        // Switching panes or tabs, or a click on another pane, takes it from the page.
        assert_eq!(desk.focus(Owner::Paddock), native(false));
        assert_eq!(desk.owner(), Owner::Paddock);
        // A click on the address field from the terminal, then Esc: the page has the keyboard.
        desk.edit();
        assert_eq!(desk.owner(), Owner::Address);
        assert_eq!(desk.focus(Owner::Page), native(true));
    }

    #[test]
    fn the_page_has_the_keyboard_back_once_it_shows_again() {
        let mut desk = Desk::new();
        desk.click_page();
        // A hover text over it, a divider dragged, its error state: AppKit's keyboard back to
        // GPUI's view, the record still the page's.
        assert_eq!(desk.show(Showing::Hidden), native(false));
        assert_eq!(desk.owner(), Owner::Page);
        assert_eq!(desk.settle(), Moves::default());
        // Shown again: the page has it back.
        assert_eq!(desk.show(Showing::Shown), native(true));
        assert_eq!(desk.owner(), Owner::Page);
        // Not when something else took the keyboard while it was hidden (⌘1, say).
        let mut desk = Desk::new();
        desk.click_page();
        desk.show(Showing::Hidden);
        desk.focus(Owner::Paddock);
        assert_eq!(desk.show(Showing::Shown), Moves::default());
        assert_eq!(desk.owner(), Owner::Paddock);
        // Nor when the terminal had it all along.
        let mut desk = Desk::new();
        assert_eq!(desk.show(Showing::Hidden), Moves::default());
        assert_eq!(desk.show(Showing::Shown), Moves::default());
    }

    #[test]
    fn closing_the_right_sidebar_or_leaving_its_browser_tab_gives_the_pane_the_keyboard() {
        // From the page.
        let mut desk = Desk::new();
        desk.click_page();
        let away = desk.show(Showing::Away);
        assert_eq!(
            away,
            Moves {
                native: Some(false),
                to_pane: true,
                ..Moves::default()
            }
        );
        assert_eq!(desk.owner(), Owner::Paddock);
        // Open again: the pane keeps it.
        assert_eq!(desk.show(Showing::Shown), Moves::default());
        assert_eq!(desk.owner(), Owner::Paddock);
        // From the address field.
        let mut desk = Desk::new();
        desk.edit();
        let away = desk.show(Showing::Away);
        assert_eq!(
            away,
            Moves {
                to_pane: true,
                ..Moves::default()
            }
        );
        assert_eq!(desk.owner(), Owner::Paddock);
        // The terminal had it: nothing moves.
        let mut desk = Desk::new();
        assert_eq!(desk.show(Showing::Away), Moves::default());
    }

    #[test]
    fn a_popup_borrows_the_keyboard_from_the_page_and_gives_it_back() {
        // A list or menu without a field (Attention, the split panel): AppKit's keyboard back to
        // GPUI's view for its keys while it is open, the page's again once it closes.
        let mut desk = Desk::new();
        desk.click_page();
        assert_eq!(desk.open_popup(false), native(false));
        assert_eq!(desk.owner(), Owner::Page);
        desk.close_popup();
        assert_eq!(desk.settle(), native(true));
        assert_eq!(desk.owner(), Owner::Page);
        // One with a field (the palette, the new tab panel) borrows GPUI's focus too.
        assert_eq!(desk.open_popup(true), native(false));
        assert_eq!(desk.owner(), Owner::Page);
        desk.close_popup();
        assert_eq!(desk.settle(), native(true));
        assert_eq!(desk.owner(), Owner::Page);
        // Unless what was chosen in it took the keyboard: an agent, a new shell.
        desk.open_popup(true);
        desk.dialog = false;
        desk.focus = Owner::Paddock;
        assert_eq!(desk.settle(), Moves::default());
        assert_eq!(desk.owner(), Owner::Paddock);
        desk.click_page();
        desk.open_popup(false);
        desk.dialog = false;
        assert_eq!(desk.focus(Owner::Paddock), Moves::default());
        assert_eq!(desk.owner(), Owner::Paddock);
        // From the terminal or the address field, a popup's field leaves the pane the keyboard.
        let mut desk = Desk::new();
        desk.edit();
        desk.open_popup(true);
        assert_eq!(desk.owner(), Owner::Paddock);
        desk.close_popup();
        assert_eq!(desk.settle(), Moves::default());
        assert_eq!(desk.owner(), Owner::Paddock);
    }

    #[test]
    fn a_click_on_the_page_closes_an_open_popup() {
        // The sidebar's menu, opened from the terminal.
        let mut desk = Desk::new();
        assert_eq!(desk.open_popup(false), Moves::default());
        let clicked = desk.click_page();
        assert_eq!(
            clicked,
            Moves {
                focus_page: true,
                dismiss: true,
                ..Moves::default()
            }
        );
        assert_eq!(desk.owner(), Owner::Page);
        assert!(desk.native && !desk.dialog);
        assert_eq!(desk.settle(), Moves::default());
        // The Attention list, opened from the page: the page keeps the keyboard it took back.
        desk.open_popup(false);
        assert!(!desk.native);
        let clicked = desk.click_page();
        assert!(clicked.dismiss && clicked.native.is_none());
        assert_eq!(desk.owner(), Owner::Page);
        assert!(desk.native);
        // A click on the page with nothing open closes nothing.
        desk.focus(Owner::Paddock);
        assert!(!desk.click_page().dismiss);
    }

    fn key(source: &str) -> Keystroke {
        Keystroke::parse(source).unwrap()
    }

    #[test]
    fn paddocks_shortcuts_are_paddocks_whoever_has_the_keyboard() {
        for owner in [Owner::Page, Owner::Address, Owner::Paddock] {
            for source in [
                "cmd-w",
                "cmd-t",
                "alt-cmd-b",
                "cmd-p",
                "cmd-b",
                "cmd-shift-p",
                "cmd-d",
                "cmd-shift-d",
                "cmd-shift-w",
                "cmd-n",
                "cmd-1",
                "cmd-9",
                "cmd-shift-]",
                "cmd-shift-[",
                "cmd-shift-a",
                "cmd-shift-n",
                "cmd-shift-enter",
                "cmd-,",
                "cmd-m",
                "cmd-q",
            ] {
                assert_eq!(route(owner, &key(source)), Route::Paddock, "{source}");
            }
        }
        // Every menu shortcut outside dialogs and fields, but the ones that act on what has
        // the keyboard.
        for binding in menu::bindings().iter().filter(|b| b.predicate().is_none()) {
            let [keystroke] = binding.keystrokes() else {
                continue;
            };
            let expected = if menu::follows_keyboard(binding.action()) {
                Route::Keyboard(Owner::Page)
            } else {
                Route::Paddock
            };
            assert_eq!(
                route(Owner::Page, keystroke.inner()),
                expected,
                "{}",
                binding.action().name()
            );
        }
    }

    #[test]
    fn l_and_r_are_the_browsers_while_it_has_the_keyboard_and_the_panes_otherwise() {
        for owner in [Owner::Page, Owner::Address] {
            assert_eq!(route(owner, &key("cmd-l")), Route::Browser);
            assert_eq!(route(owner, &key("cmd-r")), Route::Browser);
        }
        assert_eq!(
            route(Owner::Paddock, &key("cmd-l")),
            Route::Keyboard(Owner::Paddock)
        );
        assert_eq!(
            route(Owner::Paddock, &key("cmd-r")),
            Route::Keyboard(Owner::Paddock)
        );
        // With other modifiers they are not.
        assert_eq!(
            route(Owner::Page, &key("cmd-shift-r")),
            Route::Keyboard(Owner::Page)
        );
        assert_eq!(route(Owner::Page, &key("l")), Route::Keyboard(Owner::Page));
    }

    #[test]
    fn copying_pasting_and_editing_go_to_what_has_the_keyboard() {
        for source in ["cmd-c", "cmd-v", "cmd-x", "cmd-a", "cmd-z", "cmd-shift-z"] {
            for owner in [Owner::Page, Owner::Address, Owner::Paddock] {
                assert_eq!(
                    route(owner, &key(source)),
                    Route::Keyboard(owner),
                    "{source}"
                );
            }
        }
        // Finding in the terminal is the pane's own: the page has ⌘F for itself.
        for source in ["cmd-f", "cmd-g", "cmd-shift-g"] {
            assert_eq!(
                route(Owner::Page, &key(source)),
                Route::Keyboard(Owner::Page)
            );
            assert_eq!(
                route(Owner::Paddock, &key(source)),
                Route::Keyboard(Owner::Paddock)
            );
        }
        // The page's own shortcuts, such as a search box's ⌘K, and plain keys.
        for source in [
            "cmd-k",
            "cmd-shift-k",
            "cmd-e",
            "ctrl-c",
            "escape",
            "a",
            "enter",
        ] {
            assert_eq!(
                route(Owner::Page, &key(source)),
                Route::Keyboard(Owner::Page),
                "{source}"
            );
        }
    }

    #[test]
    fn the_standard_edits_are_command_c_v_x_a_z_and_shift_command_z() {
        assert_eq!(edit(&key("cmd-c")), Some(Edit::Copy));
        assert_eq!(edit(&key("cmd-v")), Some(Edit::Paste));
        assert_eq!(edit(&key("cmd-x")), Some(Edit::Cut));
        assert_eq!(edit(&key("cmd-a")), Some(Edit::SelectAll));
        assert_eq!(edit(&key("cmd-z")), Some(Edit::Undo));
        assert_eq!(edit(&key("cmd-shift-z")), Some(Edit::Redo));
        for source in [
            "c",
            "ctrl-c",
            "alt-cmd-c",
            "ctrl-cmd-v",
            "cmd-shift-c",
            "cmd-k",
            "cmd-l",
            "cmd-r",
        ] {
            assert_eq!(edit(&key(source)), None, "{source}");
        }
    }

    #[test]
    fn keys_are_named_as_paddocks_shortcuts_name_them() {
        assert_eq!(key_name("w").as_deref(), Some("w"));
        // What ⇧ makes of a letter is the letter.
        assert_eq!(key_name("P").as_deref(), Some("p"));
        assert_eq!(key_name("]").as_deref(), Some("]"));
        assert_eq!(key_name(",").as_deref(), Some(","));
        assert_eq!(key_name("1").as_deref(), Some("1"));
        assert_eq!(key_name("\r").as_deref(), Some("enter"));
        assert_eq!(key_name("\u{3}").as_deref(), Some("enter"));
        assert_eq!(key_name("\t").as_deref(), Some("tab"));
        assert_eq!(key_name("\u{1b}").as_deref(), Some("escape"));
        assert_eq!(key_name(" ").as_deref(), Some("space"));
        assert_eq!(key_name("\u{7f}").as_deref(), Some("backspace"));
        assert_eq!(key_name("\u{f700}").as_deref(), Some("up"));
        assert_eq!(key_name("\u{f703}").as_deref(), Some("right"));
        // Keys GPUI's bindings here do not name, nothing, or more than a key.
        assert_eq!(key_name("\u{f704}"), None);
        assert_eq!(key_name("\u{0}"), None);
        assert_eq!(key_name(""), None);
        assert_eq!(key_name("ab"), None);
        // A named key turns into a keystroke paddock binds.
        let enter = Keystroke {
            key: key_name("\r").unwrap(),
            ..key("cmd-shift-x")
        };
        assert_eq!(route(Owner::Page, &enter), Route::Paddock);
    }
}
