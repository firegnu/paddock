//! A paddock window out of the front (DESIGN §13 P5-162): a neutral gray laid halfway over
//! everything it draws, so every colour in it — the ground, the terminal's text and colours, the
//! sidebar's cards and marks — goes duller and grayer at once, hue still there, and so does
//! anything added later. Back in front, it goes.
use crate::{palette::Rgb, view::hsla};
use gpui::{IntoElement, Styled, Window, deferred, div};

/// The gray laid over the window.
const GRAY: Rgb = (0x2b, 0x2b, 0x2b);

/// How much of [`GRAY`] covers what is under it; the rest of each colour shows through.
const SHARE: f32 = 0.51;

/// The veil over its parent, drawn after everything else in the window, while `window` is not in
/// front; nothing while it is. It takes no mouse: a click goes to what is under it.
pub fn veil(window: &Window) -> Option<impl IntoElement> {
    (!window.is_window_active()).then(|| {
        deferred(div().absolute().inset_0().bg(hsla(GRAY, SHARE))).with_priority(usize::MAX)
    })
}

/// How opaque an AppKit view over the window is, that GPUI cannot veil: as much as shows
/// through the veil, so it fades into the veiled window under it the same way.
pub fn native_alpha(window: &Window) -> f64 {
    if window.is_window_active() {
        1.0
    } else {
        f64::from(1.0 - SHARE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `color` as the window shows it under the veil: GPUI blends in the stored sRGB values.
    fn veiled(color: Rgb) -> Rgb {
        let blend = |under: u8, gray: u8| {
            (f32::from(under) * (1.0 - SHARE) + f32::from(gray) * SHARE).round() as u8
        };
        (
            blend(color.0, GRAY.0),
            blend(color.1, GRAY.1),
            blend(color.2, GRAY.2),
        )
    }

    #[test]
    fn the_veil_turns_the_front_windows_colours_into_the_ones_the_user_asked_for() {
        // Measured from the user's two screenshots (P5-162): in front, then out of it.
        for (front, asked) in [
            ((0xc2, 0xd8, 0xd8), (0x75, 0x80, 0x80)), // text
            ((0xe0, 0x73, 0x6e), (0x84, 0x4f, 0x4b)), // red file names
            ((0x9d, 0x8c, 0xcb), (0x62, 0x5b, 0x7a)), // purple file names
            ((0x6b, 0xc4, 0x8e), (0x4b, 0x77, 0x5b)), // green
            ((0xd1, 0xb3, 0x66), (0x7c, 0x6f, 0x48)), // the prompt's yellow block
            ((0x5b, 0x9f, 0xc4), (0x43, 0x64, 0x76)), // the prompt's blue block
        ] {
            let got = veiled(front);
            let near = |a: u8, b: u8| a.abs_diff(b) <= 3;
            assert!(
                near(got.0, asked.0) && near(got.1, asked.1) && near(got.2, asked.2),
                "{front:02x?} veiled is {got:02x?}, asked {asked:02x?}"
            );
        }
        // The terminal's teal-dark ground goes to a near-neutral gray, lighter, not darker.
        let (r, g, b) = veiled((0x1b, 0x22, 0x23));
        assert!(r.max(g).max(b) - r.min(g).min(b) <= 4, "{:?}", (r, g, b));
        assert!(r > 0x1b && b > 0x23);
    }
}
