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
use block2::{DynBlock, RcBlock};
use gpui::{
    Bounds, IntoElement, Keystroke, Modifiers, Pixels, Styled, Window, WindowId, canvas, px,
};
use objc2::{
    ClassType, DefinedClass, MainThreadMarker, MainThreadOnly, Message, define_class, msg_send,
    rc::{Retained, Weak},
    runtime::{AnyObject, Bool, NSObject, NSObjectProtocol, ProtocolObject},
    sel,
};
use objc2_app_kit::{
    NSAlert, NSAlertFirstButtonReturn, NSApplication, NSEvent, NSEventModifierFlags,
    NSModalResponse, NSModalResponseOK, NSOpenPanel, NSResponder, NSTextField, NSView,
    NSWindowOrderingMode, NSWorkspace,
};
use objc2_foundation::{
    NSArray, NSData, NSError, NSHTTPURLResponse, NSPoint, NSRect, NSSize, NSString, NSURL,
    NSURLRequest, NSURLResponse, ns_string,
};
use objc2_web_kit::{
    WKDownload, WKDownloadDelegate, WKFindConfiguration, WKFindResult, WKFrameInfo, WKNavigation,
    WKNavigationAction, WKNavigationActionPolicy, WKNavigationDelegate, WKNavigationResponse,
    WKNavigationResponsePolicy, WKOpenPanelParameters, WKUIDelegate, WKWebView,
    WKWebViewConfiguration, WKWebsiteDataStore, WKWindowFeatures,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    path::{Path, PathBuf},
    ptr::{self, NonNull},
    rc::Rc,
};

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
/// field, its find field, or its page.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Owner {
    #[default]
    Paddock,
    Address,
    Find,
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
    /// What GPUI's focus is on: the page's focus handle, the address or find field, or else
    /// paddock's.
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
/// click on a pane, the address field, ⌘L, ⌘F, Esc in the find field, switching panes or tabs as
/// GPUI's focus moving, closing the right sidebar or switching its tab as the Browser going
/// [`Showing::Away`], and anything drawn over the page as it hiding for a while.
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

    /// The page was closed: true when the Browser had the keyboard, for the active pane to take.
    pub fn close(&mut self) -> bool {
        std::mem::replace(&mut self.owner, Owner::Paddock) != Owner::Paddock
    }
}

/// Where a shortcut goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    /// One of paddock's menu commands: the window or the app runs it, whoever has the keyboard.
    Paddock,
    /// ⌘L, ⌘R, ⌘F, ⌘G or ⇧⌘G while the Browser has the keyboard: its address field, loading the
    /// page again, or its find bar.
    Browser,
    /// What has the keyboard: copying, pasting and the other standard edits, finding in a pane, and
    /// any shortcut paddock does not bind, such as a page's own.
    Keyboard(Owner),
}

/// Where `keystroke` goes while `owner` has the keyboard, by paddock's shortcuts (`menu.rs`).
pub fn route(owner: Owner, keystroke: &Keystroke) -> Route {
    let modifiers = |m: &Modifiers| (m.control, m.alt, m.shift, m.platform);
    let browser = owner != Owner::Paddock;
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
            return if browser && menu::finds(action) {
                Route::Browser
            } else if menu::follows_keyboard(action) {
                Route::Keyboard(owner)
            } else {
                Route::Paddock
            };
        }
        if menu::browser(action) && browser {
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
    /// The site's certificate is not trusted: self-signed, expired, for another name.
    pub certificate: bool,
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
/// `NSURLErrorServerCertificateHasBadDate`, `…Untrusted`, `…HasUnknownRoot` and `…NotYetValid`:
/// a site whose certificate the system does not trust.
const UNTRUSTED: [isize; 4] = [-1201, -1202, -1203, -1204];

/// What the page's delegate has heard.
#[derive(Default)]
struct Heard {
    /// Where the load under way set out for.
    attempt: Option<String>,
    failure: Option<Failure>,
    /// Downloads under way, and where each is being saved.
    downloads: Vec<(Retained<WKDownload>, PathBuf)>,
    /// Where a download that finished was saved, until the toolbar takes it.
    saved: Option<PathBuf>,
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
                certificate: false,
            });
        }

        // Where the page is about to go: a download, an address another app takes, or the page.
        #[unsafe(method(webView:decidePolicyForNavigationAction:decisionHandler:))]
        fn decide_action(
            &self,
            _view: &WKWebView,
            action: &WKNavigationAction,
            decide: &DynBlock<dyn Fn(WKNavigationActionPolicy)>,
        ) {
            decide.call((action_policy(action),));
        }

        // What came back: a download when it cannot be shown or asks to be saved.
        #[unsafe(method(webView:decidePolicyForNavigationResponse:decisionHandler:))]
        fn decide_response(
            &self,
            _view: &WKWebView,
            response: &WKNavigationResponse,
            decide: &DynBlock<dyn Fn(WKNavigationResponsePolicy)>,
        ) {
            decide.call((response_policy(response),));
        }

        #[unsafe(method(webView:navigationAction:didBecomeDownload:))]
        fn action_became_download(
            &self,
            _view: &WKWebView,
            _action: &WKNavigationAction,
            download: &WKDownload,
        ) {
            self.follow(download);
        }

        #[unsafe(method(webView:navigationResponse:didBecomeDownload:))]
        fn response_became_download(
            &self,
            _view: &WKWebView,
            _response: &WKNavigationResponse,
            download: &WKDownload,
        ) {
            self.follow(download);
        }
    }

    // SAFETY: each method has the signature WKDownloadDelegate gives it.
    unsafe impl WKDownloadDelegate for Delegate {
        // Saved in ~/Downloads under the page's name, never over a file already there.
        #[unsafe(method(download:decideDestinationUsingResponse:suggestedFilename:completionHandler:))]
        fn decide_destination(
            &self,
            download: &WKDownload,
            _response: &NSURLResponse,
            suggested: &NSString,
            done: &DynBlock<dyn Fn(*mut NSURL)>,
        ) {
            // No place for it (nil): the download stops.
            match self.destination(download, &suggested.to_string()) {
                Some(url) => done.call((Retained::as_ptr(&url).cast_mut(),)),
                None => done.call((ptr::null_mut(),)),
            }
        }

        #[unsafe(method(downloadDidFinish:))]
        fn download_did_finish(&self, download: &WKDownload) {
            let mut heard = self.ivars().borrow_mut();
            if let Some(at) = heard
                .downloads
                .iter()
                .position(|(each, _)| ptr::eq(&**each, download))
            {
                heard.saved = Some(heard.downloads.remove(at).1);
            }
        }

        #[unsafe(method(download:didFailWithError:resumeData:))]
        fn download_did_fail(
            &self,
            download: &WKDownload,
            _error: &NSError,
            _resume: Option<&NSData>,
        ) {
            self.ivars()
                .borrow_mut()
                .downloads
                .retain(|(each, _)| !ptr::eq(&**each, download));
        }
    }

    // SAFETY: the method has the signature WKUIDelegate gives it.
    unsafe impl WKUIDelegate for Delegate {
        // A link with `target=_blank`, or `window.open`: opened in the page itself instead, or
        // by the system (see `open_elsewhere`); no second web view is made.
        #[unsafe(method_id(webView:createWebViewWithConfiguration:forNavigationAction:windowFeatures:))]
        fn create_web_view(
            &self,
            view: &WKWebView,
            _configuration: &WKWebViewConfiguration,
            action: &WKNavigationAction,
            _features: &WKWindowFeatures,
        ) -> Option<Retained<WKWebView>> {
            // SAFETY: a plain getter on the main thread.
            open_elsewhere(view, &*unsafe { action.request() });
            None
        }

        // `alert()`, `confirm()` and `prompt()`: the system's alert as a sheet on the window.
        #[unsafe(method(webView:runJavaScriptAlertPanelWithMessage:initiatedByFrame:completionHandler:))]
        fn alert(
            &self,
            view: &WKWebView,
            message: &NSString,
            _frame: &WKFrameInfo,
            done: &DynBlock<dyn Fn()>,
        ) {
            let done = done.copy();
            ask(view, message, false, None, move |_, _| done.call(()));
        }

        #[unsafe(method(webView:runJavaScriptConfirmPanelWithMessage:initiatedByFrame:completionHandler:))]
        fn confirm(
            &self,
            view: &WKWebView,
            message: &NSString,
            _frame: &WKFrameInfo,
            done: &DynBlock<dyn Fn(Bool)>,
        ) {
            let done = done.copy();
            ask(view, message, true, None, move |ok, _| {
                done.call((Bool::new(ok),))
            });
        }

        #[unsafe(method(webView:runJavaScriptTextInputPanelWithPrompt:defaultText:initiatedByFrame:completionHandler:))]
        fn prompt(
            &self,
            view: &WKWebView,
            prompt: &NSString,
            default: Option<&NSString>,
            _frame: &WKFrameInfo,
            done: &DynBlock<dyn Fn(*mut NSString)>,
        ) {
            let done = done.copy();
            let default = NSString::from_str(&default.map(NSString::to_string).unwrap_or_default());
            ask(view, prompt, true, Some(&default), move |ok, typed| {
                match typed.filter(|_| ok) {
                    Some(typed) => done.call((Retained::as_ptr(&typed).cast_mut(),)),
                    None => done.call((ptr::null_mut(),)),
                }
            });
        }

        // `<input type=file>`: the system's open panel as a sheet on the window.
        #[unsafe(method(webView:runOpenPanelWithParameters:initiatedByFrame:completionHandler:))]
        fn choose_files(
            &self,
            view: &WKWebView,
            parameters: &WKOpenPanelParameters,
            _frame: &WKFrameInfo,
            done: &DynBlock<dyn Fn(*mut NSArray<NSURL>)>,
        ) {
            let done = done.copy();
            let Some(window) = view.window() else {
                done.call((ptr::null_mut(),));
                return;
            };
            let panel = NSOpenPanel::openPanel(self.mtm());
            panel.setCanChooseFiles(true);
            // SAFETY: plain getters on the main thread.
            unsafe {
                panel.setCanChooseDirectories(parameters.allowsDirectories());
                panel.setAllowsMultipleSelection(parameters.allowsMultipleSelection());
            }
            let chosen = panel.clone();
            let closed = RcBlock::new(move |response: NSModalResponse| {
                if response == NSModalResponseOK {
                    let urls = chosen.URLs();
                    done.call((Retained::as_ptr(&urls).cast_mut(),));
                } else {
                    done.call((ptr::null_mut(),));
                }
            });
            panel.beginSheetModalForWindow_completionHandler(&window, &closed);
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

    /// A load failed: noted, unless it was only stopped or gave way to another. A certificate the
    /// system does not trust is left at that: nothing here lets such a site through.
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
            certificate: domain == "NSURLErrorDomain" && UNTRUSTED.contains(&code),
        });
    }

    /// Hears how `download` goes from here.
    fn follow(&self, download: &WKDownload) {
        // SAFETY: WebKit keeps the delegate weakly; the page keeps it as long as the web view.
        unsafe { download.setDelegate(Some(ProtocolObject::from_ref(self))) };
    }

    /// Where `download`, which the page calls `suggested`, is saved: in ~/Downloads under its own
    /// name ([`download_name`]), or the first free numbered one ([`download_path`]), counting
    /// those taken by downloads still under way; none when there is no such folder to save in.
    fn destination(&self, download: &WKDownload, suggested: &str) -> Option<Retained<NSURL>> {
        let folder = PathBuf::from(std::env::var_os("HOME")?).join("Downloads");
        std::fs::create_dir_all(&folder).ok()?;
        let mut heard = self.ivars().borrow_mut();
        let path = download_path(&folder, &download_name(suggested), |path| {
            path.symlink_metadata().is_ok() || heard.downloads.iter().any(|(_, at)| at == path)
        });
        let url = NSURL::fileURLWithPath(&NSString::from_str(path.to_str()?));
        heard.downloads.push((download.retain(), path));
        Some(url)
    }
}

/// What the page does about going to what `action` asks for: download it when asked to; give an
/// address another app takes ([`opens`]) to the system when the page itself goes there, and not
/// open it from a frame inside the page; go anywhere else.
fn action_policy(action: &WKNavigationAction) -> WKNavigationActionPolicy {
    // SAFETY: plain getters on the main thread.
    unsafe {
        if action.shouldPerformDownload() {
            return WKNavigationActionPolicy::Download;
        }
        let request = action.request();
        let Some(url) = request.URL() else {
            return WKNavigationActionPolicy::Allow;
        };
        let scheme = url.scheme().map(|scheme| scheme.to_string());
        if opens(scheme.as_deref().unwrap_or_default()) != Opens::System {
            return WKNavigationActionPolicy::Allow;
        }
        // No frame: a new window, which the page would not get anyway (see `create_web_view`).
        if action.targetFrame().is_none_or(|frame| frame.isMainFrame()) {
            NSWorkspace::sharedWorkspace().openURL(&url);
        }
        WKNavigationActionPolicy::Cancel
    }
}

/// What the page does with what came back: saves it as a download when it cannot show it or the
/// server asks for it to be saved (`Content-Disposition: attachment`); else shows it.
fn response_policy(response: &WKNavigationResponse) -> WKNavigationResponsePolicy {
    // SAFETY: plain getters on the main thread.
    let (shows, response) = unsafe { (response.canShowMIMEType(), response.response()) };
    let attachment = response
        .downcast::<NSHTTPURLResponse>()
        .ok()
        .and_then(|http| http.valueForHTTPHeaderField(ns_string!("Content-Disposition")))
        .is_some_and(|value| {
            value
                .to_string()
                .trim_start()
                .to_ascii_lowercase()
                .starts_with("attachment")
        });
    if shows && !attachment {
        WKNavigationResponsePolicy::Allow
    } else {
        WKNavigationResponsePolicy::Download
    }
}

/// Shows a page's `message` in the system's alert, as a sheet on the page's window, with OK, and
/// Cancel when the page `cancels`, and a field holding `field` for a prompt; `answer` hears once
/// whether OK was chosen, and what the field held then. Without a window, answered as cancelled.
fn ask(
    view: &WKWebView,
    message: &NSString,
    cancels: bool,
    field: Option<&NSString>,
    answer: impl Fn(bool, Option<Retained<NSString>>) + 'static,
) {
    let Some(window) = view.window() else {
        answer(false, None);
        return;
    };
    let main = view.mtm();
    let alert = NSAlert::new(main);
    alert.setMessageText(message);
    alert.addButtonWithTitle(ns_string!("OK"));
    if cancels {
        // AppKit gives a button called Cancel the Esc key.
        alert.addButtonWithTitle(ns_string!("Cancel"));
    }
    let field = field.map(|text| {
        let field = NSTextField::textFieldWithString(text, main);
        field.setFrame(NSRect::new(NSPoint::ZERO, NSSize::new(260.0, 24.0)));
        alert.setAccessoryView(Some(&field));
        field
    });
    // The handler keeps the alert, and with it the field, until the sheet closes.
    let shown = alert.clone();
    let closed = RcBlock::new(move |response: NSModalResponse| {
        let typed = shown
            .accessoryView()
            .and_then(|view| view.downcast::<NSTextField>().ok())
            .map(|field| field.stringValue());
        answer(response == NSAlertFirstButtonReturn, typed);
    });
    alert.beginSheetModalForWindow_completionHandler(&window, Some(&closed));
    if let Some(field) = field {
        alert.window().makeFirstResponder(Some(&field));
    }
}

/// Shows `path` selected in a Finder window.
pub fn reveal(path: &Path) {
    if let Some(path) = path.to_str() {
        NSWorkspace::sharedWorkspace()
            .selectFile_inFileViewerRootedAtPath(Some(&NSString::from_str(path)), ns_string!(""));
    }
}

/// Moves the page's selection to where it starts, if it has one.
const COLLAPSE: &str = "(s => s && s.rangeCount && s.collapseToStart())(getSelection())";

/// Which way the find bar looks through the page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Look {
    /// The text was typed: from where the match shown starts, so it grows in place.
    Again,
    Next,
    Previous,
}

/// The page: the web view, owned here besides its place in the window's view, and what its
/// navigation delegate hears. Only made and used on the main thread.
pub struct Page {
    view: Retained<WebView>,
    /// Kept here: the web view only refers to it.
    delegate: Retained<Delegate>,
    /// Where it shows; none while hidden.
    shown: Option<NSRect>,
    /// Whether the last search found anything, once it has said, until read.
    found: Rc<Cell<Option<bool>>>,
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
        unsafe {
            view.setNavigationDelegate(Some(ProtocolObject::from_ref(&*delegate)));
            view.setUIDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        }
        // Safari's Develop menu can inspect the page where the system lets it (macOS 13.3 on).
        if view.respondsToSelector(sel!(setInspectable:)) {
            // SAFETY: a plain setter on the main thread, there since macOS 13.3.
            unsafe { view.setInspectable(true) };
        }
        view.setHidden(true);
        round(&view, radius);
        parent.addSubview_positioned_relativeTo(&view, NSWindowOrderingMode::Above, Some(&gpui));
        Some(Self {
            view,
            delegate,
            shown: None,
            found: Rc::default(),
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

    /// Looks through the page for `query`, ignoring case and going round past either end, and
    /// selects the match; whether there was one comes from [`Page::found`] once WebKit says.
    pub fn find(&self, query: &str, look: Look) {
        let found = self.found.clone();
        let done = RcBlock::new(move |result: NonNull<WKFindResult>| {
            // SAFETY: WebKit's result, alive while it calls this.
            found.set(Some(unsafe { result.as_ref().matchFound() }));
        });
        // SAFETY: on the main thread; both run in the page in the order sent.
        unsafe {
            if look == Look::Again {
                // A search starts after what is selected: from where the match shown starts
                // instead, so a match that grows as more is typed stays where it is.
                self.view
                    .evaluateJavaScript_completionHandler(ns_string!(COLLAPSE), None);
            }
            let config = WKFindConfiguration::new(self.view.mtm());
            config.setBackwards(look == Look::Previous);
            config.setCaseSensitive(false);
            config.setWraps(true);
            self.view.findString_withConfiguration_completionHandler(
                &NSString::from_str(query),
                Some(&config),
                &done,
            );
        }
    }

    /// Whether the last search found anything, once, after it has said.
    pub fn found(&self) -> Option<bool> {
        self.found.take()
    }

    /// Where a download that finished since the last call was saved.
    pub fn saved(&self) -> Option<PathBuf> {
        self.delegate.ivars().borrow_mut().saved.take()
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
            self.view.setUIDelegate(None);
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

/// Where an address a page opens in a new window goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opens {
    /// A web page: in the page itself, which can go back to where it came from.
    Here,
    /// An address another app takes, such as `mailto:` or `tel:`: to the system.
    System,
    /// Anything else, such as `about:blank` or `javascript:`: not opened.
    Nowhere,
}

/// Where an address with `scheme` goes when a page opens it in a new window.
pub fn opens(scheme: &str) -> Opens {
    match scheme.to_ascii_lowercase().as_str() {
        "http" | "https" => Opens::Here,
        "" | "about" | "blob" | "data" | "javascript" | "file" => Opens::Nowhere,
        _ => Opens::System,
    }
}

/// Opens what `request` asks for as [`opens`] says, `view` being the page.
fn open_elsewhere(view: &WKWebView, request: &NSURLRequest) {
    let Some(url) = request.URL() else {
        return;
    };
    let scheme = url.scheme().map(|scheme| scheme.to_string());
    match opens(scheme.as_deref().unwrap_or_default()) {
        // SAFETY: a plain load on the main thread.
        Opens::Here => unsafe {
            view.loadRequest(request);
        },
        Opens::System => {
            NSWorkspace::sharedWorkspace().openURL(&url);
        }
        Opens::Nowhere => {}
    }
}

/// What a download is saved as: the name the page gives it without any folders in it, and with
/// nothing that cannot be typed; `download` when that leaves no name.
pub fn download_name(suggested: &str) -> String {
    let last = suggested.rsplit(['/', '\\']).next().unwrap_or_default();
    let name: String = last.chars().filter(|c| !c.is_control()).collect();
    match name.trim() {
        "" | "." | ".." => "download".to_owned(),
        name => name.to_owned(),
    }
}

/// Where a download named `name` (as [`download_name`] gives it) is saved in `folder`: under that
/// name, else `name-2.ext`, `name-3.ext` and so on, the first that `taken` says is free, so
/// nothing already there is written over.
pub fn download_path(folder: &Path, name: &str, taken: impl Fn(&Path) -> bool) -> PathBuf {
    let (stem, extension) = match name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() && !extension.is_empty() => {
            (stem, Some(extension))
        }
        _ => (name, None),
    };
    let numbered = (2u64..).map(|n| match extension {
        Some(extension) => format!("{stem}-{n}.{extension}"),
        None => format!("{stem}-{n}"),
    });
    std::iter::once(name.to_owned())
        .chain(numbered)
        .map(|name| folder.join(name))
        .find(|path| !taken(path))
        .expect("a free name")
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

        /// ⌘F from the page or the address field: AppKit's keyboard first, then GPUI's focus to
        /// the find field.
        fn open_find(&mut self) -> Moves {
            self.native = false;
            self.focus(Owner::Find)
        }

        fn show(&mut self, showing: Showing) -> Moves {
            self.showing = showing;
            self.settle()
        }

        /// A click on the toolbar's ×: the page goes, with AppKit's keyboard if it had it, and the
        /// tab shows its empty state; the active pane takes the keyboard if the record says so.
        fn close_page(&mut self) -> Moves {
            self.native = false;
            self.settle();
            if self.keys.close() {
                self.focus = Owner::Paddock;
            }
            self.native = false;
            self.show(Showing::Hidden)
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
    fn the_find_field_takes_the_keyboard_from_the_page_and_esc_gives_it_back() {
        // ⌘F from the page: AppKit's keyboard back to GPUI's view, then the find field has it.
        let mut desk = Desk::new();
        desk.click_page();
        assert_eq!(desk.open_find(), Moves::default());
        assert_eq!(desk.owner(), Owner::Find);
        assert!(!desk.native && desk.focus == Owner::Find);
        assert_eq!(desk.settle(), Moves::default());
        // ⏎ and ⌘G look on in the field; Esc closes the bar and the page has it back.
        assert_eq!(desk.focus(Owner::Page), native(true));
        assert_eq!(desk.owner(), Owner::Page);
        // ⌘F from the address field.
        desk.edit();
        assert_eq!(desk.open_find(), Moves::default());
        assert_eq!(desk.owner(), Owner::Find);
        // A click on the page while finding: the field gives the keyboard up, the bar stays.
        let clicked = desk.click_page();
        assert_eq!(
            clicked,
            Moves {
                focus_page: true,
                ..Moves::default()
            }
        );
        assert_eq!(desk.owner(), Owner::Page);
        // ⌘L from the find field, and back with ⌘F.
        desk.open_find();
        desk.edit();
        assert_eq!(desk.owner(), Owner::Address);
        desk.open_find();
        assert_eq!(desk.owner(), Owner::Find);
        // The page hidden for a while (a hover text over it): the field keeps the keyboard, and
        // the page does not take it back when it shows again.
        assert_eq!(desk.show(Showing::Hidden), Moves::default());
        assert_eq!(desk.show(Showing::Shown), Moves::default());
        assert_eq!(desk.owner(), Owner::Find);
        // A click on the terminal from the find field: the terminal has it.
        assert_eq!(desk.focus(Owner::Paddock), Moves::default());
        assert_eq!(desk.owner(), Owner::Paddock);
        // The right sidebar closed from the find field: the pane has it.
        desk.click_page();
        desk.open_find();
        let away = desk.show(Showing::Away);
        assert_eq!(
            away,
            Moves {
                to_pane: true,
                ..Moves::default()
            }
        );
        assert_eq!(desk.owner(), Owner::Paddock);
    }

    #[test]
    fn finding_is_the_browsers_while_it_has_the_keyboard_and_the_panes_otherwise() {
        for source in ["cmd-f", "cmd-g", "cmd-shift-g"] {
            for owner in [Owner::Page, Owner::Address, Owner::Find] {
                assert_eq!(route(owner, &key(source)), Route::Browser, "{source}");
            }
            // The terminal's find bar, as before.
            assert_eq!(
                route(Owner::Paddock, &key(source)),
                Route::Keyboard(Owner::Paddock),
                "{source}"
            );
        }
        // In the find field, ⏎, ⇧⏎ and Esc are its own, and the others as in the address field.
        for source in ["enter", "shift-enter", "escape", "cmd-c", "cmd-v", "cmd-a"] {
            assert_eq!(
                route(Owner::Find, &key(source)),
                Route::Keyboard(Owner::Find),
                "{source}"
            );
        }
        for source in ["cmd-l", "cmd-r"] {
            assert_eq!(route(Owner::Find, &key(source)), Route::Browser);
        }
        for source in ["cmd-w", "cmd-t", "cmd-p", "alt-cmd-b"] {
            assert_eq!(route(Owner::Find, &key(source)), Route::Paddock);
        }
    }

    #[test]
    fn downloads_keep_the_pages_name_without_its_folders() {
        assert_eq!(download_name("report.pdf"), "report.pdf");
        assert_eq!(
            download_name("Quarterly report 2026.xlsx"),
            "Quarterly report 2026.xlsx"
        );
        assert_eq!(download_name("数据.csv"), "数据.csv");
        // Folders named, up or down, either way round.
        assert_eq!(download_name("../../.zshrc"), ".zshrc");
        assert_eq!(download_name("/etc/passwd"), "passwd");
        assert_eq!(download_name("assets/img/logo.png"), "logo.png");
        assert_eq!(download_name("..\\..\\evil.sh"), "evil.sh");
        assert_eq!(download_name("C:\\Users\\me\\notes.txt"), "notes.txt");
        // Nothing that cannot be typed, nor blanks round it.
        assert_eq!(download_name(" bad\u{0}na\nme.txt "), "badname.txt");
        // No name left: a plain one.
        for suggested in ["", "  ", ".", "..", "a/..", "a/b/", "../", "\u{7}"] {
            assert_eq!(download_name(suggested), "download", "{suggested:?}");
        }
    }

    #[test]
    fn downloads_never_write_over_a_file_already_there() {
        let folder = Path::new("/Users/someone/Downloads");
        let taken = |names: &'static [&'static str]| {
            move |path: &Path| {
                assert_eq!(path.parent(), Some(folder));
                names
                    .iter()
                    .any(|name| path.file_name() == Some(name.as_ref()))
            }
        };
        assert_eq!(
            download_path(folder, "report.pdf", taken(&[])),
            folder.join("report.pdf")
        );
        assert_eq!(
            download_path(folder, "report.pdf", taken(&["report.pdf"])),
            folder.join("report-2.pdf")
        );
        assert_eq!(
            download_path(
                folder,
                "report.pdf",
                taken(&["report.pdf", "report-2.pdf", "report-3.pdf"])
            ),
            folder.join("report-4.pdf")
        );
        // Another file's numbered name is no reason to number this one.
        assert_eq!(
            download_path(folder, "report.pdf", taken(&["report-2.pdf"])),
            folder.join("report.pdf")
        );
        // No extension, a hidden file, several dots.
        assert_eq!(
            download_path(folder, "README", taken(&["README"])),
            folder.join("README-2")
        );
        assert_eq!(
            download_path(folder, ".env", taken(&[".env"])),
            folder.join(".env-2")
        );
        assert_eq!(
            download_path(folder, "site.tar.gz", taken(&["site.tar.gz"])),
            folder.join("site.tar-2.gz")
        );
        assert_eq!(
            download_path(folder, "download", taken(&["download"])),
            folder.join("download-2")
        );
        // Whatever the page called it, it stays in the folder.
        for suggested in ["../../x", "/tmp/y", "..", "a\\..\\..\\z"] {
            let path = download_path(folder, &download_name(suggested), taken(&[]));
            assert_eq!(path.parent(), Some(folder), "{suggested}");
        }
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

    #[test]
    fn closing_the_page_gives_its_keyboard_to_the_pane() {
        // From the page, the address field or the find field: the pane has it after.
        for take in [Desk::click_page, Desk::edit, Desk::open_find] {
            let mut desk = Desk::new();
            take(&mut desk);
            assert_ne!(desk.owner(), Owner::Paddock);
            assert_eq!(desk.close_page(), Moves::default());
            assert_eq!(desk.owner(), Owner::Paddock);
            assert_eq!((desk.focus, desk.native), (Owner::Paddock, false));
            assert_eq!(desk.settle(), Moves::default());
        }
        // The terminal had it: it keeps it.
        let mut desk = Desk::new();
        assert_eq!(desk.close_page(), Moves::default());
        assert_eq!((desk.owner(), desk.focus), (Owner::Paddock, Owner::Paddock));
    }

    fn key(source: &str) -> Keystroke {
        Keystroke::parse(source).unwrap()
    }

    #[test]
    fn paddocks_shortcuts_are_paddocks_whoever_has_the_keyboard() {
        for owner in [Owner::Page, Owner::Address, Owner::Find, Owner::Paddock] {
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
        // the keyboard, and finding, which the Browser does in its find bar.
        for binding in menu::bindings().iter().filter(|b| b.predicate().is_none()) {
            let [keystroke] = binding.keystrokes() else {
                continue;
            };
            let expected = if menu::finds(binding.action()) {
                Route::Browser
            } else if menu::follows_keyboard(binding.action()) {
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
        for owner in [Owner::Page, Owner::Address, Owner::Find] {
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
            for owner in [Owner::Page, Owner::Address, Owner::Find, Owner::Paddock] {
                assert_eq!(
                    route(owner, &key(source)),
                    Route::Keyboard(owner),
                    "{source}"
                );
            }
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
