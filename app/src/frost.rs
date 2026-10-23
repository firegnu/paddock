//! The system's sidebar material under the main window (DESIGN §13, P5-19b, P5-95): one AppKit
//! `NSVisualEffectView`, the material Finder's sidebar uses, blurring what is behind the window
//! with the system's own frosting, brightening and saturation. It sits under GPUI's view, the
//! window's size, so it shows wherever GPUI leaves the window clear: the sidebar's column and the
//! frame around the panes' panels, which GPUI draws opaque over it; GPUI's view covers it whole,
//! so it never takes the mouse or the keyboard. It can be thinned, down to nothing, letting the
//! desktop behind show clearer (P5-169).
use objc2::{MainThreadMarker, MainThreadOnly, rc::Retained};
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
    NSAutoresizingMaskOptions, NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial,
    NSVisualEffectState, NSVisualEffectView, NSWindowOrderingMode,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// The material, owned here besides its place in the window's view: removed when this is dropped.
/// Only made and used on the main thread (`NSVisualEffectView` is main-thread only).
pub struct Frost {
    view: Retained<NSVisualEffectView>,
    /// It shows now.
    shown: bool,
    /// Whether it shows once the window has drawn what it gives up (see [`Frost::fit`]).
    target: bool,
    /// The appearance it was last given: dark or light.
    dark: Option<bool>,
    /// How opaque it was last made, from 0 to 1.
    opacity: f32,
}

impl Frost {
    /// Puts the material behind `window`'s GPUI view, hidden until [`Frost::fit`] shows it; none
    /// off the main thread or without an AppKit view to put it behind.
    pub fn install(window: &gpui::Window) -> Option<Self> {
        let main = MainThreadMarker::new()?;
        let handle = HasWindowHandle::window_handle(window).ok()?;
        let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
            return None;
        };
        // SAFETY: GPUI's live NSView, read on the main thread while `window` is borrowed.
        let gpui_view = unsafe { appkit.ns_view.cast::<NSView>().as_ref() };
        // SAFETY: a plain getter; GPUI puts its view in the window's content view.
        let parent = unsafe { gpui_view.superview() }?;
        // The whole window, which it follows as the window is resized.
        let view =
            NSVisualEffectView::initWithFrame(NSVisualEffectView::alloc(main), parent.bounds());
        view.setMaterial(NSVisualEffectMaterial::Sidebar);
        view.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
        // Frosted whether or not the window is in front: left to follow it, the system draws the
        // material a flat grey behind other windows (P5-24b).
        view.setState(NSVisualEffectState::Active);
        view.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable
                | NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        view.setHidden(true);
        parent.addSubview_positioned_relativeTo(
            &view,
            NSWindowOrderingMode::Below,
            Some(gpui_view),
        );
        // Left as it starts, a see-through window lets a click through to the window behind
        // wherever nothing is drawn; with the material thinned to nothing, that is the seams, the
        // title bar's empty stretches and a frame left clear. Set to false, AppKit gives the
        // window every click in its frame instead (P5-169).
        if let Some(window) = gpui_view.window() {
            window.setIgnoresMouseEvents(false);
        }
        Some(Self {
            view,
            shown: false,
            target: false,
            dark: None,
            // A new view's own.
            opacity: 1.0,
        })
    }

    /// Shows the material or hides it. Shown, it shows at once, under a frame GPUI is about to
    /// draw. Hidden, it waits for [`Frost::settle`] after that frame is drawn: AppKit can put a
    /// changed view on screen before GPUI's frame, and what GPUI leaves clear must never be
    /// without the material under it. True when it waits.
    pub fn fit(&mut self, shown: bool) -> bool {
        self.target = shown;
        self.set_shown(self.shown || shown);
        self.shown != self.target
    }

    /// The hiding [`Frost::fit`] put off, now that the window has drawn it.
    pub fn settle(&mut self) {
        self.set_shown(self.target);
    }

    fn set_shown(&mut self, shown: bool) {
        if shown != self.shown {
            self.view.setHidden(!shown);
            self.shown = shown;
        }
    }

    /// The material's dark or light look, after the theme.
    pub fn set_dark(&mut self, dark: bool) {
        if self.dark == Some(dark) {
            return;
        }
        // SAFETY: AppKit's constant appearance names.
        let name = unsafe {
            if dark {
                NSAppearanceNameDarkAqua
            } else {
                NSAppearanceNameAqua
            }
        };
        self.view
            .setAppearance(NSAppearance::appearanceNamed(name).as_deref());
        self.dark = Some(dark);
    }

    /// How opaque the material lies over the desktop behind the window, from 0 to 1 (P5-169):
    /// thinner, its frosting, brightening and tint fade together and the desktop shows clearer;
    /// at 0 it is gone, and the window is clear glass wherever GPUI draws nothing.
    pub fn set_opacity(&mut self, opacity: f32) {
        if self.opacity == opacity {
            return;
        }
        self.view.setAlphaValue(f64::from(opacity));
        self.opacity = opacity;
    }
}

impl Drop for Frost {
    fn drop(&mut self) {
        // The window's view lets go of it too; a window already closed has nothing to remove from.
        self.view.removeFromSuperview();
    }
}
