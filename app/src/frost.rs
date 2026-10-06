//! The system's sidebar material under the main window's left column (DESIGN §13, P5-19b): one
//! AppKit `NSVisualEffectView`, the material Finder's sidebar uses, blurring what is behind the
//! window with the system's own frosting, brightening and saturation. It sits under GPUI's view,
//! which draws everything else opaque over it, so it only shows where GPUI leaves the column clear;
//! GPUI's view covers it whole, so it never takes the mouse or the keyboard. Beside the collapsed
//! strip it starts under the title bar (P5-19e).
use objc2::{MainThreadMarker, MainThreadOnly, rc::Retained};
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
    NSAutoresizingMaskOptions, NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial,
    NSVisualEffectView, NSWindowOrderingMode,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// Where the material shows, in points from the window's top-left corner: a column `width` wide
/// from `top` down to the bottom edge. All zero while hidden.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Shape {
    pub top: f32,
    pub width: f32,
}

impl Shape {
    /// The least shape covering both; a hidden one covers nothing.
    fn cover(self, other: Self) -> Self {
        if self.width <= 0.0 {
            other
        } else if other.width <= 0.0 {
            self
        } else {
            Self {
                top: self.top.min(other.top),
                width: self.width.max(other.width),
            }
        }
    }
}

/// The material, owned here besides its place in the window's view: removed when this is dropped.
/// Only made and used on the main thread (`NSVisualEffectView` is main-thread only).
pub struct Frost {
    view: Retained<NSVisualEffectView>,
    /// Its shape now; all zero while hidden.
    shape: Shape,
    /// Its shape once the window has drawn what it gives up (see [`Frost::fit`]).
    target: Shape,
    /// The appearance it was last given: dark or light.
    dark: Option<bool>,
}

impl Frost {
    /// Puts the material behind `window`'s GPUI view, hidden until [`Frost::fit`] gives it a shape;
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
            shape: Shape::default(),
            target: Shape::default(),
            dark: None,
        })
    }

    /// Shows the material in `shape`, or hides it (`None`). What it adds shows at once, under a
    /// frame GPUI is about to draw. What it gives up waits for [`Frost::settle`] after that frame
    /// is drawn: AppKit can put a changed view on screen before GPUI's frame, and what GPUI leaves
    /// clear must never be without the material under it. True when it waits.
    pub fn fit(&mut self, shape: Option<Shape>) -> bool {
        self.target = shape.unwrap_or_default();
        self.set_shape(self.shape.cover(self.target));
        self.shape != self.target
    }

    /// The shape [`Frost::fit`] put off, now that the window has drawn it.
    pub fn settle(&mut self) {
        self.set_shape(self.target);
    }

    fn set_shape(&mut self, shape: Shape) {
        if shape == self.shape {
            return;
        }
        // SAFETY: a plain getter; it was put in the window's content view, gone with the window.
        let Some(parent) = (unsafe { self.view.superview() }) else {
            return;
        };
        // From the left edge, `top` down from the top edge to the bottom; the margins stay as the
        // window is resized, so it keeps starting under the title bar.
        let top = f64::from(shape.top);
        let mut frame = parent.bounds();
        frame.size.width = f64::from(shape.width);
        frame.size.height = (frame.size.height - top).max(0.0);
        if parent.isFlipped() {
            frame.origin.y += top;
        }
        self.view.setFrame(frame);
        self.view.setHidden(shape.width <= 0.0);
        self.shape = shape;
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
