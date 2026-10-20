//! Window decoration for terminal-owned signals. Existing tab marks and focus rims stay intact.
use super::*;
use crate::{terminal::Progress, view::signals::Frame};
use gpui::PathBuilder;

const RING: f32 = 12.0;
const DOT: f32 = 4.0;
const GAP: f32 = 4.0;

impl PaddockWindow {
    pub(super) fn signal_ink(&self, signal: Frame) -> Hsla {
        self.fg(if matches!(signal.progress, Progress::Error(_)) {
            |t| t.agents_red
        } else {
            |t| t.agents_accent
        })
    }

    pub(super) fn tab_signal(
        &self,
        index: usize,
        now: Instant,
        still: bool,
        cx: &Context<Self>,
    ) -> Frame {
        let mut selected = Frame::default();
        let mut bell: f32 = 0.0;
        let rank = |frame: Frame| {
            if frame.opacity <= 0.0 {
                return 0;
            }
            if frame.completing {
                return 1;
            }
            match frame.progress {
                Progress::Error(_) => 5,
                Progress::Paused(_) => 4,
                Progress::Indeterminate => 3,
                Progress::Percent(_) => 2,
                Progress::None => 0,
            }
        };
        for pane in self.workspace.tabs[index].panes() {
            let frame = self.panes[&pane].read(cx).signal_frame(now, still);
            if rank(frame) > rank(selected) {
                selected = frame;
            }
            bell = bell.max(frame.bell);
        }
        selected.bell = bell;
        selected.moving |= bell > 0.0;
        selected
    }
}

/// Extra room in the capsule, including the gap to the neighbouring item.
pub(super) fn tab_width(frame: Frame, scale: f32) -> f32 {
    let progress = frame.opacity > 0.0;
    let bell = frame.bell > 0.0;
    if !progress && !bell {
        return 0.0;
    }
    (TAB_GAP
        + if progress { RING } else { 0.0 }
        + if bell { DOT } else { 0.0 }
        + if progress && bell { GAP } else { 0.0 })
        * scale
}

pub(super) fn tab_mark(frame: Frame, ink: Hsla, amber: Hsla, scale: f32) -> Option<Div> {
    (tab_width(frame, scale) > 0.0).then(|| {
        div()
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap(px(GAP * scale))
            .when(frame.opacity > 0.0, |mark| {
                mark.child(ring(frame, ink, scale))
            })
            .when(frame.bell > 0.0, |mark| {
                mark.child(
                    div()
                        .flex_shrink_0()
                        .size(px(DOT * scale))
                        .rounded_full()
                        .bg(amber.opacity(frame.bell)),
                )
            })
    })
}

fn ring(frame: Frame, ink: Hsla, scale: f32) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let arc = |offset: f32, fraction: f32| {
                let mut path = PathBuilder::stroke(px(1.5 * scale));
                let center = bounds.origin + point(px(RING * scale / 2.0), px(RING * scale / 2.0));
                let radius = (RING - 2.0) * scale / 2.0;
                for step in 0..=64 {
                    let angle = std::f32::consts::TAU * (offset + fraction * step as f32 / 64.0)
                        - std::f32::consts::FRAC_PI_2;
                    let at = center + point(px(radius * angle.cos()), px(radius * angle.sin()));
                    if step == 0 {
                        path.move_to(at);
                    } else {
                        path.line_to(at);
                    }
                }
                path.build()
            };
            if let Ok(path) = arc(0.0, 1.0) {
                window.paint_path(path, ink.opacity(frame.opacity * 0.18));
            }
            if frame.fraction > 0.0
                && let Ok(path) = arc(frame.offset, frame.fraction)
            {
                window.paint_path(path, ink.opacity(frame.opacity));
            }
        },
    )
    .flex_shrink_0()
    .size(px(RING * scale))
}

pub(super) fn bar(frame: Frame, ink: Hsla) -> Option<Div> {
    (frame.opacity > 0.0).then(|| {
        div()
            .absolute()
            .bottom_0()
            .left_0()
            .right_0()
            .h(px(2.0))
            .overflow_hidden()
            .bg(ink.opacity(frame.opacity * 0.12))
            .child(
                div()
                    .absolute()
                    .h_full()
                    .left(relative(frame.offset * (1.0 - frame.fraction)))
                    .w(relative(frame.fraction))
                    .bg(ink.opacity(frame.opacity)),
            )
    })
}

pub(super) fn rim(glow: f32, amber: Hsla) -> Div {
    div()
        .absolute()
        .inset_0()
        .rounded(px(PANEL_RADIUS))
        .border(px(5.0))
        .border_color(amber.opacity(glow * 0.07))
        .child(
            div()
                .absolute()
                .inset_0()
                .rounded(px(PANEL_RADIUS))
                .border(px(1.5))
                .border_color(amber.opacity(glow * 0.30)),
        )
}
