//! paddock out of the front (DESIGN §13 P5-162): while another application is in front, each
//! colour paddock's windows draw is faded at its source: most of its saturation gone, its lightness
//! drawn well towards a dark middle, so they read as out of the front while every colour is
//! still known and the text still crisp. One of paddock's own windows over another is not out of
//! the front. The theme hands its colours out so (`Theme::faded`), and each window draws in that
//! copy while paddock is not in front ([`Front`]); the terminal fades its cells the same way.
use crate::palette::Rgb;
use block2::RcBlock;
use gpui::{App, Global};
use objc2::{MainThreadMarker, class, msg_send, rc::Retained, runtime::AnyObject};
use objc2_app_kit::NSApplication;
use objc2_foundation::{NSString, ns_string};
use std::ptr::{self, NonNull};

/// Whether paddock is the application in front, as the system says; its windows watch it
/// (`cx.observe_global_in::<Front>`). GPUI says only whether a window is, so this asks AppKit.
pub struct Front {
    active: bool,
    /// The system's notification center keeps telling while these are kept.
    _observers: Vec<Retained<AnyObject>>,
}

impl Global for Front {}

/// Whether paddock is the application in front; in front too before [`watch`] has asked.
pub fn in_front(cx: &App) -> bool {
    cx.try_global::<Front>().is_none_or(|front| front.active)
}

/// Starts following whether paddock is in front: as it is now, then each time the system says it
/// became the application in front or stopped being it. AppKit's notice only puts the news on a
/// channel; GPUI takes it from there, outside AppKit's call. Nothing off the main thread.
pub fn watch(cx: &mut App) {
    let Some(main) = MainThreadMarker::new() else {
        return;
    };
    let active = NSApplication::sharedApplication(main).isActive();
    let (tell, told) = async_channel::unbounded::<bool>();
    let notices: [(&NSString, bool); 2] = [
        (ns_string!("NSApplicationDidBecomeActiveNotification"), true),
        (
            ns_string!("NSApplicationDidResignActiveNotification"),
            false,
        ),
    ];
    let observers = notices
        .into_iter()
        .map(|(name, active)| {
            let tell = tell.clone();
            let heard = RcBlock::new(move |_notice: NonNull<AnyObject>| {
                let _ = tell.try_send(active);
            });
            // SAFETY: on the main thread; NSNotificationCenter's block observer, called on the
            // main queue, which copies the block and returns the observer to keep.
            unsafe {
                let center: *mut AnyObject = msg_send![class!(NSNotificationCenter), defaultCenter];
                let queue: *mut AnyObject = msg_send![class!(NSOperationQueue), mainQueue];
                let observer: Retained<AnyObject> = msg_send![
                    center,
                    addObserverForName: name,
                    object: ptr::null_mut::<AnyObject>(),
                    queue: queue,
                    usingBlock: &*heard,
                ];
                observer
            }
        })
        .collect();
    cx.set_global(Front {
        active,
        _observers: observers,
    });
    cx.spawn(async move |cx| {
        while let Ok(active) = told.recv().await {
            cx.update(|cx| {
                if in_front(cx) != active {
                    cx.global_mut::<Front>().active = active;
                }
            });
        }
    })
    .detach();
}

/// How much of a colour's saturation stays.
const SATURATION: f32 = 0.4;

/// How much of a colour's distance in lightness from [`MIDDLE`] stays.
const CONTRAST: f32 = 0.62;

/// The lightness the contrast closes towards: low, as a dark theme's grounds are dark, so a ground
/// lifts only a little and text dims only a little.
const MIDDLE: f32 = 50.0;

/// On a light theme's grounds (P5-175) the lightness closes towards white instead, so a ground
/// pales only a little and dark text lightens; more of the distance stays, as a light theme's text
/// stands less far from its ground and would not read otherwise.
const MIDDLE_LIGHT: f32 = 255.0;
const CONTRAST_LIGHT: f32 = 0.88;

/// `color` out of the front: the same hue, less than half as saturated, its lightness well nearer
/// [`MIDDLE`], or nearer [`MIDDLE_LIGHT`] for a `light` theme. Grays stay gray.
pub fn fade((r, g, b): Rgb, light: bool) -> Rgb {
    let (middle, contrast) = if light {
        (MIDDLE_LIGHT, CONTRAST_LIGHT)
    } else {
        (MIDDLE, CONTRAST)
    };
    let [r, g, b] = [r, g, b].map(f32::from);
    let lightness = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    let lightness_now = middle + (lightness - middle) * contrast;
    let channel = |c: f32| {
        (lightness_now + (c - lightness) * SATURATION)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    (channel(r), channel(g), channel(b))
}

/// A GPUI colour faded as [`fade`] does, its opacity kept.
pub fn fade_hsla(color: gpui::Hsla, light: bool) -> gpui::Hsla {
    let rgba = gpui::Rgba::from(color);
    let channel = |c: f32| (c * 255.0).round().clamp(0.0, 255.0) as u8;
    let faded = fade((channel(rgba.r), channel(rgba.g), channel(rgba.b)), light);
    crate::view::hsla(faded, rgba.a)
}

/// A whole terminal palette faded, as its own background is dark or light.
pub fn fade_palette(theme: &crate::palette::Theme) -> crate::palette::Theme {
    let fade = |color| fade(color, theme.is_light());
    crate::palette::Theme {
        ansi: theme.ansi.map(fade),
        foreground: fade(theme.foreground),
        background: fade(theme.background),
        cursor: fade(theme.cursor),
        selection: fade(theme.selection),
    }
}

/// A row of terminal cells faded, their text and ground: whatever colour the program gave them,
/// a palette's or its own; `light`, the theme's.
pub fn fade_cells(cells: &mut [crate::rows::Cell], light: bool) {
    for cell in cells {
        cell.bg = fade(cell.bg, light);
        cell.style.fg = fade(cell.style.fg, light);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spread((r, g, b): Rgb) -> u8 {
        r.max(g).max(b) - r.min(g).min(b)
    }

    fn lightness((r, g, b): Rgb) -> f32 {
        0.2126 * f32::from(r) + 0.7152 * f32::from(g) + 0.0722 * f32::from(b)
    }

    // The colours of the user's screenshot in front (P5-162): text, red, purple and green file
    // names, the prompt's yellow and blue blocks, and the terminal's teal-dark ground.
    const SEEN: [Rgb; 7] = [
        (0xc2, 0xd8, 0xd8),
        (0xe0, 0x73, 0x6e),
        (0x9d, 0x8c, 0xcb),
        (0x6b, 0xc4, 0x8e),
        (0xd1, 0xb3, 0x66),
        (0x5b, 0x9f, 0xc4),
        (0x1b, 0x22, 0x23),
    ];

    #[test]
    fn a_faded_colour_keeps_its_share_of_saturation_and_its_hue() {
        for color in SEEN {
            let faded = fade(color, false);
            let (before, after) = (f32::from(spread(color)), f32::from(spread(faded)));
            assert!(
                (after - before * SATURATION).abs() <= 1.5,
                "{color:02x?} → {faded:02x?}: spread {before} → {after}"
            );
            // The strongest channel stays the strongest, the weakest the weakest.
            let order = |(r, g, b): Rgb| {
                let mut channels = [(r, 0), (g, 1), (b, 2)];
                channels.sort();
                (channels[0].1, channels[2].1)
            };
            if spread(color) > 8 {
                assert_eq!(order(color), order(faded), "{color:02x?} → {faded:02x?}");
            }
        }
    }

    #[test]
    fn a_gpui_colour_fades_alike_and_keeps_its_opacity() {
        for color in SEEN {
            let faded = fade_hsla(crate::view::hsla(color, 0.3), false);
            let rgba = gpui::Rgba::from(faded);
            let back = |c: f32| (c * 255.0).round() as u8;
            let want = fade(color, false);
            assert!(
                back(rgba.r).abs_diff(want.0) <= 1
                    && back(rgba.g).abs_diff(want.1) <= 1
                    && back(rgba.b).abs_diff(want.2) <= 1,
                "{color:02x?}: {rgba:?} for {want:02x?}"
            );
            assert!((rgba.a - 0.3).abs() < 1e-4);
        }
    }

    #[test]
    fn grays_stay_gray_and_lightness_closes_in_a_little_keeping_its_order() {
        for v in [0u8, 0x1b, 0x60, 0xa0, 0xeb, 0xff] {
            for light in [false, true] {
                let (r, g, b) = fade((v, v, v), light);
                assert!(r == g && g == b, "{v:02x} → {:02x?}", (r, g, b));
            }
        }
        // The ground lifts a little, the text dims clearly; neither goes to a flat gray.
        let (ground, text) = (SEEN[6], SEEN[0]);
        let lift = lightness(fade(ground, false)) - lightness(ground);
        assert!((3.0..=15.0).contains(&lift), "ground lifts {lift}");
        let dim = lightness(text) - lightness(fade(text, false));
        assert!((40.0..=80.0).contains(&dim), "text dims {dim}");
        // Lighter stays lighter.
        let mut by_lightness = SEEN;
        by_lightness.sort_by(|a, b| lightness(*a).total_cmp(&lightness(*b)));
        let faded: Vec<f32> = by_lightness
            .iter()
            .map(|c| lightness(fade(*c, false)))
            .collect();
        assert!(faded.windows(2).all(|w| w[0] <= w[1]), "{faded:?}");
    }

    // Catppuccin Latte's text, red, green and mauve, then its base and mantle (P5-175).
    const LATTE: [Rgb; 6] = [
        (0x4c, 0x4f, 0x69),
        (0xd2, 0x0f, 0x39),
        (0x40, 0xa0, 0x2b),
        (0x88, 0x39, 0xef),
        (0xef, 0xf1, 0xf5),
        (0xe6, 0xe9, 0xef),
    ];

    #[test]
    fn on_a_light_theme_the_ground_pales_a_little_and_the_text_lightens() {
        for color in LATTE {
            let faded = fade(color, true);
            let (before, after) = (f32::from(spread(color)), f32::from(spread(faded)));
            assert!(
                (after - before * SATURATION).abs() <= 1.5,
                "{color:02x?} → {faded:02x?}: spread {before} → {after}"
            );
        }
        let (ground, text) = (LATTE[4], LATTE[0]);
        let pale = lightness(fade(ground, true)) - lightness(ground);
        assert!((0.5..=5.0).contains(&pale), "ground pales {pale}");
        let lift = lightness(fade(text, true)) - lightness(text);
        assert!((10.0..=40.0).contains(&lift), "text lightens {lift}");
        // Lighter stays lighter.
        let mut by_lightness = LATTE;
        by_lightness.sort_by(|a, b| lightness(*a).total_cmp(&lightness(*b)));
        let faded: Vec<f32> = by_lightness
            .iter()
            .map(|c| lightness(fade(*c, true)))
            .collect();
        assert!(faded.windows(2).all(|w| w[0] <= w[1]), "{faded:?}");
    }
}
