//! The system's sidebar material under the main window's left column (DESIGN §13, P5-19b): one
//! AppKit `NSVisualEffectView`, the material Finder's sidebar uses, blurring what is behind the
//! window with the system's own frosting, brightening and saturation. It sits under GPUI's view,
//! which draws everything else opaque over it, so it only shows where GPUI leaves the column clear;
//! GPUI's view covers it whole, so it never takes the mouse or the keyboard.
use objc2::{MainThreadMarker, MainThreadOnly, rc::Retained};
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
    NSAutoresizingMaskOptions, NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial,
    NSVisualEffectView, NSWindowOrderingMode,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// The material, owned here besides its place in the window's view: removed when this is dropped.
/// Only made and used on the main thread (`NSVisualEffectView` is main-thread only).
pub struct Frost {
    view: Retained<NSVisualEffectView>,
    /// How wide it is now; 0 while hidden.
    width: f32,
    /// How wide it is to be once the window has drawn its narrower column (see [`Frost::fit`]).
    target: f32,
    /// The appearance it was last given: dark or light.
    dark: Option<bool>,
}

impl Frost {
    /// Puts the material behind `window`'s GPUI view, hidden until [`Frost::fit`] gives it a width;
    /// none off the main thread or without an AppKit view to put it behind.
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
        // From the left edge, the full height, which follows the window as it is resized.
        let mut frame = parent.bounds();
        frame.size.width = 0.0;
        let view = NSVisualEffectView::initWithFrame(NSVisualEffectView::alloc(main), frame);
        view.setMaterial(NSVisualEffectMaterial::Sidebar);
        view.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
        view.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewHeightSizable
                | NSAutoresizingMaskOptions::ViewMaxXMargin,
        );
        view.setHidden(true);
        parent.addSubview_positioned_relativeTo(
            &view,
            NSWindowOrderingMode::Below,
            Some(gpui_view),
        );
        Some(Self {
            view,
            width: 0.0,
            target: 0.0,
            dark: None,
        })
    }

    /// Shows the material `width` points wide from the window's left edge, or hides it (`None`).
    /// Wider shows at once, under a frame GPUI is about to draw. Narrower waits for
    /// [`Frost::settle`] after that frame is drawn: AppKit can put a changed view on screen before
    /// GPUI's frame, and the column must never be clear without the material under it. True when
    /// it waits.
    pub fn fit(&mut self, width: Option<f32>) -> bool {
        self.target = width.unwrap_or(0.0);
        if self.target >= self.width {
            self.set_width(self.target);
            false
        } else {
            true
        }
    }

    /// The narrower width [`Frost::fit`] put off, now that the window has drawn it.
    pub fn settle(&mut self) {
        if self.target < self.width {
            self.set_width(self.target);
        }
    }

    fn set_width(&mut self, width: f32) {
        if width == self.width {
            return;
        }
        let mut frame = self.view.frame();
        frame.size.width = f64::from(width);
        self.view.setFrame(frame);
        self.view.setHidden(width <= 0.0);
        self.width = width;
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
}

impl Drop for Frost {
    fn drop(&mut self) {
        // The window's view lets go of it too; a window already closed has nothing to remove from.
        self.view.removeFromSuperview();
    }
}
