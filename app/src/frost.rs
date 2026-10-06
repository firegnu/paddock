//! The system's sidebar material under the main window's left column (DESIGN §13, P5-19b): one
//! AppKit `NSVisualEffectView`, the material Finder's sidebar uses, blurring what is behind the
//! window with the system's own frosting, brightening and saturation. It sits under GPUI's view,
//! which draws everything else opaque over it, so it only shows where GPUI leaves the column clear;
//! GPUI's view covers it whole, so it never takes the mouse or the keyboard. Beside the collapsed
//! strip it also runs along the top edge (P5-19d), one view masked to that upside-down L, so the
//! two parts blur as one piece.
use objc2::{AnyThread, MainThreadMarker, MainThreadOnly, rc::Retained};
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
    NSAutoresizingMaskOptions, NSBitmapImageRep, NSDeviceRGBColorSpace, NSImage,
    NSImageResizingMode, NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial,
    NSVisualEffectView, NSWindowOrderingMode,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// Where the material shows, in points from the window's top-left corner: a column `column` wide
/// from the top edge to the bottom and, when `bar` is wider, along the top edge to `bar`,
/// `bar_height` tall. All zero while hidden.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Shape {
    pub column: f32,
    pub bar: f32,
    pub bar_height: f32,
}

impl Shape {
    /// The least shape covering both.
    fn cover(self, other: Self) -> Self {
        Self {
            column: self.column.max(other.column),
            bar: self.bar.max(other.bar),
            bar_height: self.bar_height.max(other.bar_height),
        }
    }
}

/// The mask's pixels to a point, sharp on a Retina screen.
const MASK_SCALE: usize = 2;

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
        let mut frame = self.view.frame();
        frame.size.width = f64::from(shape.column.max(shape.bar));
        self.view.setFrame(frame);
        self.view.setMaskImage(upside_down_l(shape).as_deref());
        self.view.setHidden(shape.column <= 0.0);
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

/// The mask that cuts the view, as wide as the bar, to `shape`'s upside-down L; none for a plain
/// column. A small image AppKit stretches to the view: the bar's rows and the column's points
/// are its caps, drawn as they are, and one point of each in between stretches, across under
/// the bar and down the column. Whole points, rounded out: past the shape it lies under what
/// GPUI draws opaque.
fn upside_down_l(shape: Shape) -> Option<Retained<NSImage>> {
    let (left, top) = (shape.column.ceil(), shape.bar_height.ceil());
    // Under the bar the mask needs a point to stretch and one for the cap at its end.
    if top <= 0.0 || shape.bar < left + 2.0 {
        return None;
    }
    let (left, top) = (left as usize, top as usize);
    let (width, height) = ((left + 2) * MASK_SCALE, (top + 2) * MASK_SCALE);
    // SAFETY: null planes, so AppKit allocates the pixels; 8-bit RGBA, rows packed.
    let pixels = unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            std::ptr::null_mut(),
            width as isize,
            height as isize,
            8,
            4,
            true,
            false,
            NSDeviceRGBColorSpace,
            (width * 4) as isize,
            32,
        )
    }?;
    let data = pixels.bitmapData();
    if data.is_null() {
        return None;
    }
    // SAFETY: the rep's own buffer, `height` rows of `width` RGBA pixels, the top row first.
    let data = unsafe { std::slice::from_raw_parts_mut(data, width * height * 4) };
    for (index, pixel) in data.chunks_exact_mut(4).enumerate() {
        let (x, y) = (index % width, index / width);
        let shown = y < top * MASK_SCALE || x < left * MASK_SCALE;
        // Black, premultiplied: only the alpha counts.
        pixel.copy_from_slice(&[0, 0, 0, if shown { 255 } else { 0 }]);
    }
    // In points: the rep's size, changed.
    let mut size = pixels.size();
    size.width = (left + 2) as f64;
    size.height = (top + 2) as f64;
    pixels.setSize(size);
    let image = NSImage::initWithSize(NSImage::alloc(), size);
    image.addRepresentation(&pixels);
    let mut caps = image.capInsets();
    caps.top = top as f64;
    caps.left = left as f64;
    caps.bottom = 1.0;
    caps.right = 1.0;
    image.setCapInsets(caps);
    image.setResizingMode(NSImageResizingMode::Stretch);
    Some(image)
}

impl Drop for Frost {
    fn drop(&mut self) {
        // The window's view lets go of it too; a window already closed has nothing to remove from.
        self.view.removeFromSuperview();
    }
}
