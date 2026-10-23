//! The system's material under the main window (DESIGN §13, P5-19b, P5-95, P5-174): one AppKit
//! `NSVisualEffectView` blurring what is behind the window with the system's own frosting,
//! brightening and saturation. It sits under GPUI's view, the window's size, so it shows wherever
//! GPUI leaves the window clear: the sidebar's column and the frame around the panes' panels,
//! which GPUI draws over it; GPUI's view covers it whole, so it never takes the mouse or the
//! keyboard. Its blur and saturation always stay whole; only the tint over them can be thinned,
//! down to nothing, leaving the desktop behind a soft blur of its own colours that shows no shapes
//! (P5-174; P5-169 thinned the whole view, which let the shapes through).
//!
//! The tint is reached inside the view, which AppKit does not publish. On macOS 27 the view's
//! layer holds one layer of the material's whose sublayers are a `CABackdropLayer` (the blur and
//! saturation filters), then the tint (a fill colour, and a tone blended over it), then a
//! `CAChameleonLayer` (the desktop's faint colour, left as the system sets it). The tint is taken
//! to be the layers drawn over a `CABackdropLayer` beside it, found by class, since the layers'
//! names change with the appearance. Where no such layer is found, nothing is changed and the
//! material stays whole.
use objc2::{
    MainThreadMarker, MainThreadOnly, msg_send,
    rc::Retained,
    runtime::{AnyClass, AnyObject},
};
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
    NSAutoresizingMaskOptions, NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial,
    NSVisualEffectState, NSVisualEffectView, NSWindowOrderingMode,
};
use objc2_foundation::NSArray;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// Which of the system's materials lies under the window (P5-174): the one Finder's sidebar uses.
/// `NSVisualEffectMaterial::Menu` is the one menus use.
const MATERIAL: NSVisualEffectMaterial = NSVisualEffectMaterial::Sidebar;

/// How deep in the view's layers the material's own layer is looked for.
const DEPTH: usize = 4;

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
    /// How strongly its tint lies over the blur, from 0 to 1 (see [`Frost::set_tint`]).
    tint: f32,
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
        view.setMaterial(MATERIAL);
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
        // wherever nothing is drawn, such as the seams and the title bar's empty stretches. Set to
        // false, AppKit gives the window every click in its frame instead (P5-169).
        if let Some(window) = gpui_view.window() {
            window.setIgnoresMouseEvents(false);
        }
        Some(Self {
            view,
            shown: false,
            target: false,
            dark: None,
            // The system's own.
            tint: 1.0,
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
            if shown {
                self.refresh();
            }
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
        self.refresh();
    }

    /// How strongly the material's tint lies over its blur, from 0 to 1 (P5-174): 1 is the
    /// system's own; lower, more of the blurred desktop's colours come through; 0 leaves the blur
    /// and saturation alone. AppKit makes the tint's layers afresh, at its own strength, when it
    /// redraws the material (a new appearance, for one), so this is called for every frame the
    /// window draws and puts back what AppKit replaced.
    pub fn set_tint(&mut self, tint: f32) {
        self.tint = tint.clamp(0.0, 1.0);
        self.apply();
    }

    /// Has AppKit make the material's layers now, where it would at its next drawing, and tints
    /// them: a new appearance takes the tint's layers away at once and makes new ones later.
    fn refresh(&self) {
        self.view.displayIfNeeded();
        self.apply();
    }

    fn apply(&self) {
        let tint = self.tint;
        let changed: Vec<_> = self
            .tints()
            .into_iter()
            .filter(|layer| {
                // SAFETY: CALayer's `opacity`, a plain getter.
                let opacity: f32 = unsafe { msg_send![&**layer, opacity] };
                opacity != tint
            })
            .collect();
        if changed.is_empty() {
            return;
        }
        let Some(transaction) = AnyClass::get(c"CATransaction") else {
            return;
        };
        // SAFETY: CATransaction's class methods and CALayer's `opacity` setter, on the main
        // thread; without animation, so the slider's every step shows at once.
        unsafe {
            let _: () = msg_send![transaction, begin];
            let _: () = msg_send![transaction, setDisableActions: true];
            for layer in changed {
                let _: () = msg_send![&*layer, setOpacity: tint];
            }
            let _: () = msg_send![transaction, commit];
        }
    }

    /// The layers of the tint, or none where the view's layers are not as described at the top.
    fn tints(&self) -> Vec<Retained<AnyObject>> {
        let Some(backdrop) = AnyClass::get(c"CABackdropLayer") else {
            return Vec::new();
        };
        let chameleon = AnyClass::get(c"CAChameleonLayer");
        // SAFETY: the view's layer, a plain getter on the main thread.
        let root: Option<Retained<AnyObject>> = unsafe { msg_send![&*self.view, layer] };
        let mut level: Vec<Retained<AnyObject>> = root.into_iter().collect();
        for _ in 0..DEPTH {
            let mut next = Vec::new();
            for layer in &level {
                let sublayers = sublayers(layer);
                let kinds: Vec<Kind> = sublayers
                    .iter()
                    .map(|layer| {
                        if is_kind(layer, backdrop) {
                            Kind::Backdrop
                        } else if chameleon.is_some_and(|class| is_kind(layer, class)) {
                            Kind::Chameleon
                        } else {
                            Kind::Other
                        }
                    })
                    .collect();
                if let Some(tints) = tint_indices(&kinds) {
                    return tints.into_iter().map(|i| sublayers[i].clone()).collect();
                }
                next.extend(sublayers);
            }
            level = next;
        }
        Vec::new()
    }
}

/// What a layer beside the material's blur is.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    /// The blur and saturation (`CABackdropLayer`).
    Backdrop,
    /// The desktop's faint colour (`CAChameleonLayer`).
    Chameleon,
    Other,
}

/// Of a layer's sublayers, bottom first, the tint's: those drawn over the first blur, but the
/// desktop's colour; none without a blur among them or anything over it.
fn tint_indices(kinds: &[Kind]) -> Option<Vec<usize>> {
    let blur = kinds.iter().position(|kind| *kind == Kind::Backdrop)?;
    let tints: Vec<usize> = (blur + 1..kinds.len())
        .filter(|&i| kinds[i] == Kind::Other)
        .collect();
    (!tints.is_empty()).then_some(tints)
}

fn sublayers(layer: &AnyObject) -> Vec<Retained<AnyObject>> {
    // SAFETY: CALayer's `sublayers`, a plain getter.
    let sublayers: Option<Retained<NSArray<AnyObject>>> = unsafe { msg_send![layer, sublayers] };
    sublayers.map_or_else(Vec::new, |sublayers| sublayers.to_vec())
}

fn is_kind(layer: &AnyObject, class: &AnyClass) -> bool {
    // SAFETY: NSObject's `isKindOfClass:`.
    unsafe { msg_send![layer, isKindOfClass: class] }
}

impl Drop for Frost {
    fn drop(&mut self) {
        // The window's view lets go of it too; a window already closed has nothing to remove from.
        self.view.removeFromSuperview();
    }
}

#[cfg(test)]
mod tests {
    use super::{Kind::*, tint_indices};

    #[test]
    fn the_tint_is_what_lies_over_the_blur_but_the_desktops_colour() {
        // As macOS 27 lays out both materials, dark and light.
        assert_eq!(
            tint_indices(&[Backdrop, Other, Other, Chameleon]),
            Some(vec![1, 2])
        );
        // Under the blur is not the tint.
        assert_eq!(
            tint_indices(&[Other, Backdrop, Other, Chameleon, Other]),
            Some(vec![2, 4])
        );
        // No blur, or nothing over it: not the material's layer, and the search goes on; found
        // nowhere, the material is left whole.
        assert_eq!(tint_indices(&[Backdrop]), None);
        assert_eq!(tint_indices(&[Backdrop, Chameleon]), None);
        assert_eq!(tint_indices(&[Other, Other, Chameleon]), None);
        assert_eq!(tint_indices(&[]), None);
    }
}
