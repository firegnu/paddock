//! The right sidebar's Browser page (DESIGN §13, P5-28): the system's `WKWebView`, one per window,
//! an AppKit view beside GPUI's and over it, as the sidebar material is beside it under it
//! (`frost.rs`). GPUI cannot draw over it, so the page shows only where the Browser tab lays it
//! out, and only while nothing GPUI draws over the window meets it ([`Scene`]): every popup, menu
//! and hover text notes itself with [`cover`]. Hidden, it keeps its page and its history. It keeps
//! website data as the system keeps it for paddock, so sign-ins last.
use gpui::{App, Bounds, IntoElement, Pixels, Styled, Window, WindowId, canvas, px};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, NSObject, NSObjectProtocol, ProtocolObject},
};
use objc2_app_kit::{NSResponder, NSView, NSWindowOrderingMode};
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
    view: Retained<WKWebView>,
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
        // store is the persistent one.
        let view = unsafe {
            let config = WKWebViewConfiguration::new(main);
            config.setWebsiteDataStore(&WKWebsiteDataStore::defaultDataStore(main));
            WKWebView::initWithFrame_configuration(WKWebView::alloc(main), NSRect::ZERO, &config)
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

    /// Shows the page at `bounds` in `window`, or hides it (`None`) with what it shows kept;
    /// hidden, it gives the keyboard back to GPUI's view if it had it.
    pub fn place(&mut self, bounds: Option<Bounds<Pixels>>, window: &Window, cx: &mut App) {
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
            None => {
                self.view.setHidden(true);
                if self.has_keys() {
                    // After this frame: AppKit asks GPUI's view about its text as it takes them.
                    window.defer(cx, |window, _| take_keys(window));
                }
            }
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
    fn has_keys(&self) -> bool {
        let responder = self
            .view
            .window()
            .and_then(|window| window.firstResponder());
        responder
            .and_then(|responder| responder.downcast::<NSView>().ok())
            .is_some_and(|view| view.isDescendantOf(&self.view))
    }
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
/// clicked: a click anywhere GPUI draws takes it back.
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
}
