//! The right sidebar: pushed out from the window's right edge, narrowing the terminal rather than
//! covering it, with four tabs, Changes, Browser, Kanban and Cairn, a button to widen it and one to
//! close it. Changes shows the focused pane's worktree (`changes.rs`); Browser a web page
//! (`browser_view.rs`); Kanban the task files of the focused pane's repository
//! (`kanban_view.rs`); Cairn what cairn has recorded for the focused pane's directory
//! (`cairn_view.rs`). The window draws the card it sits in and lays it out; the rules for its
//! width live here, so they can be tested without a window.
use crate::{
    browser,
    diff::Scope,
    fonts::UiFont,
    footer_icon::{self, Icon},
    kanban::{self, Column},
    theme::Theme,
    view::hsla,
};
use gpui::{
    AnyElement, App, ClickEvent, Div, FontWeight, Hsla, Pixels, Render, Window, div, prelude::*, px,
};
use serde::{Deserialize, Serialize};
use std::rc::Rc;

/// The width it opens at the first time, in points.
pub const DEFAULT_WIDTH: f32 = 420.0;
/// The narrowest it goes at the base interface size; larger sizes widen it alike, so the tab row,
/// its four tabs and two buttons, keeps to one line.
pub const MIN_WIDTH: f32 = 426.0;
/// The widest it goes, as a share of the window.
pub const MAX_SHARE: f32 = 0.7;
/// What it always leaves the terminal, when the window has room for both: about 44 columns at
/// the default font.
pub const TERMINAL_MIN: f32 = 360.0;

type Pick = fn(&crate::preset::Theme) -> crate::preset::Color;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tab {
    #[default]
    Changes,
    Browser,
    Kanban,
    Cairn,
}

impl Tab {
    pub const ALL: [Tab; 4] = [Tab::Changes, Tab::Browser, Tab::Kanban, Tab::Cairn];

    pub fn label(self) -> &'static str {
        match self {
            Tab::Changes => "Changes",
            Tab::Browser => "Browser",
            Tab::Kanban => "Kanban",
            Tab::Cairn => "Cairn",
        }
    }

    /// The placeholder's line until the tab has something in it.
    pub fn empty(self) -> &'static str {
        match self {
            Tab::Changes => "Changes will show here",
            Tab::Browser => "Open a page",
            Tab::Kanban => "Tasks will show here",
            Tab::Cairn => "Handover records will show here",
        }
    }
}

/// What the layout file keeps of it; files from before it was saved open it closed, at the
/// default width, on Changes, files from before Changes was made show uncommitted changes,
/// unified, files from before the Browser kept its address open it on nothing, and files from
/// before the Kanban fold only its DONE group.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Saved {
    pub open: bool,
    pub width: f32,
    pub tab: Tab,
    /// Which changes the Changes tab shows.
    pub scope: Scope,
    /// Widened, the Changes tab shows old and new side by side.
    pub split: bool,
    /// The address the Browser tab last showed, which it opens again.
    pub url: Option<String>,
    /// The Kanban tab's folded groups.
    pub kanban_folded: Vec<Column>,
}

impl Default for Saved {
    fn default() -> Self {
        Saved {
            open: false,
            width: DEFAULT_WIDTH,
            tab: Tab::Changes,
            scope: Scope::Uncommitted,
            split: false,
            url: None,
            kanban_folded: kanban::folded_default(),
        }
    }
}

/// The room it has, in points: the window's width, what else takes width across it (the left
/// sidebar and the seams around the cards), and its own least width at this interface size.
#[derive(Clone, Copy, Debug)]
pub struct Room {
    pub window: f32,
    pub others: f32,
    pub min: f32,
}

impl Room {
    /// `width` kept within reach, to whole points: at most 70% of the window and what leaves the
    /// terminal its least; never under its own least, which wins when the window is too narrow
    /// for both.
    pub fn clamp(self, width: f32) -> f32 {
        let max = (self.window * MAX_SHARE).min(self.window - self.others - TERMINAL_MIN);
        width.min(max).max(self.min).round()
    }
}

pub struct RightPanel {
    pub open: bool,
    /// Its width as set, which the layout keeps.
    pub width: f32,
    /// Widened to about half the window, until widened again or dragged.
    pub wide: bool,
    pub tab: Tab,
    /// The Changes tab's scope and layout, as it last said.
    pub scope: Scope,
    pub split: bool,
    /// The Browser tab's address, as it last said.
    pub url: Option<String>,
    /// The Kanban tab's folded groups, as it last said.
    pub kanban_folded: Vec<Column>,
    /// The divider is being dragged: the width shown and the mouse's x when it was pressed.
    resizing: Option<(f32, f32)>,
}

impl RightPanel {
    pub fn new(saved: &Saved) -> Self {
        RightPanel {
            open: saved.open,
            width: saved.width,
            wide: false,
            tab: saved.tab,
            scope: saved.scope,
            split: saved.split,
            url: saved.url.clone(),
            kanban_folded: saved.kanban_folded.clone(),
            resizing: None,
        }
    }

    pub fn saved(&self) -> Saved {
        Saved {
            open: self.open,
            width: self.width,
            tab: self.tab,
            scope: self.scope,
            split: self.split,
            url: self.url.clone(),
            kanban_folded: self.kanban_folded.clone(),
        }
    }

    /// How wide it is drawn in `room`: its width, or about half the window while widened (never
    /// narrower than its width), kept within reach.
    pub fn shown(&self, room: Room) -> f32 {
        let width = if self.wide {
            (room.window / 2.0).max(self.width)
        } else {
            self.width
        };
        room.clamp(width)
    }

    /// The divider was pressed at `x`.
    pub fn press(&mut self, room: Room, x: f32) {
        self.resizing = Some((self.shown(room), x));
    }

    /// The divider moved to `x`: it follows, its width becoming what is shown, no longer
    /// widened. Whether that changed anything.
    pub fn drag(&mut self, room: Room, x: f32) -> bool {
        let Some((width, from)) = self.resizing else {
            return false;
        };
        let width = room.clamp(width + from - x);
        if width == self.shown(room) {
            return false;
        }
        self.width = width;
        self.wide = false;
        true
    }

    /// The divider was let go; whether it was being dragged.
    pub fn release(&mut self) -> bool {
        self.resizing.take().is_some()
    }

    pub fn resizing(&self) -> bool {
        self.resizing.is_some()
    }

    /// Its top row and the active tab's content, on the ground of the card the window puts it in:
    /// `content` is the Changes or the Browser view. `pick` chooses a tab; `widen` and `close` are
    /// the buttons at the right end.
    pub fn render(
        &self,
        theme: &Theme,
        ui: &UiFont,
        content: AnyElement,
        pick: impl Fn(&Tab, &mut Window, &mut App) + 'static,
        widen: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Div {
        let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
        let highlight = hsla(theme.bg(|t| t.agent_selected), 1.0);
        let pick = Rc::new(pick);
        let bright = fg(|t| t.agents_text);
        let mut tabs = div().flex().items_center().gap(ui.px(2.0)).min_w(px(0.0));
        for tab in Tab::ALL {
            let on = tab == self.tab;
            let glyph = match tab {
                Tab::Changes => div()
                    .flex_shrink_0()
                    .w(ui.px(12.0))
                    .flex()
                    .justify_center()
                    .child("±")
                    .into_any_element(),
                Tab::Browser | Tab::Kanban | Tab::Cairn => footer_icon::icon(
                    match tab {
                        Tab::Browser => Icon::Browser,
                        Tab::Kanban => Icon::Kanban,
                        _ => Icon::Cairn,
                    },
                    if on {
                        fg(|t| t.agents_accent)
                    } else {
                        fg(|t| t.agents_dimmer)
                    },
                    ui.scale(12.0 / footer_icon::SIZE),
                )
                .into_any_element(),
            };
            let pick = pick.clone();
            let text = if on {
                fg(|t| t.agents_text)
            } else {
                fg(|t| t.muted)
            };
            tabs = tabs.child(
                div()
                    .id(tab.label())
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap(ui.px(7.0))
                    .h(ui.px(28.0))
                    .px(ui.px(11.0))
                    .rounded(px(7.0))
                    .whitespace_nowrap()
                    .text_size(ui.px(12.5))
                    .cursor_pointer()
                    .text_color(text)
                    .when(on, |item| {
                        item.bg(highlight).font_weight(FontWeight::SEMIBOLD)
                    })
                    .when(!on, |item| {
                        item.hover(move |style| style.text_color(bright))
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .text_color(if on {
                                fg(|t| t.agents_accent)
                            } else {
                                fg(|t| t.agents_dimmer)
                            })
                            .child(glyph),
                    )
                    .child(tab.label())
                    .on_click(move |_, window, cx| pick(&tab, window, cx)),
            );
        }
        let tip = |text: &'static str| Tip::new(text, theme, ui);
        let button = |id: &'static str, icon: Icon, tip: Tip| {
            div()
                .id(id)
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .size(ui.px(26.0))
                .rounded(px(6.0))
                .cursor_pointer()
                .hover(move |style| style.bg(highlight))
                .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
                .child(footer_icon::icon(
                    icon,
                    fg(|t| t.muted),
                    ui.scale(12.0 / footer_icon::SIZE),
                ))
        };
        let (icon, words) = if self.wide {
            (Icon::Restore, "Narrower")
        } else {
            (Icon::Zoom, "Wider")
        };
        let top = div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(ui.px(2.0))
            .h(ui.px(40.0))
            .pl(ui.px(10.0))
            .pr(ui.px(8.0))
            .child(tabs)
            .child(div().flex_1().min_w(px(0.0)))
            .child(button("right-wider", icon, tip(words)).on_click(widen))
            .child(button("right-close", Icon::Close, tip("Close sidebar")).on_click(close));
        div().size_full().flex().flex_col().child(top).child(
            div()
                .flex_1()
                .min_h(px(0.0))
                .flex()
                .flex_col()
                .child(content),
        )
    }
}

/// A tab with nothing in it yet: a faint icon and one line, in the middle.
pub fn placeholder(theme: &Theme, ui: &UiFont, tab: Tab) -> Div {
    let icon = match tab {
        Tab::Changes => Icon::Changes,
        Tab::Browser => Icon::Browser,
        Tab::Kanban => Icon::Kanban,
        Tab::Cairn => Icon::Cairn,
    };
    div()
        .flex_1()
        .min_h(px(0.0))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(ui.px(12.0))
        .pb(ui.px(40.0))
        .child(footer_icon::icon(
            icon,
            hsla(theme.fg(|t| t.agents_dimmer), 0.7),
            ui.scale(26.0 / footer_icon::SIZE),
        ))
        .child(
            div()
                .whitespace_nowrap()
                .text_size(ui.px(13.0))
                .text_color(hsla(theme.fg(|t| t.agents_dim), 1.0))
                .child(tab.empty()),
        )
}

/// A button's hover text, here and on the Browser's toolbar.
#[derive(Clone)]
pub struct Tip {
    text: &'static str,
    size: Pixels,
    color: Hsla,
    background: Hsla,
    border: Hsla,
}

impl Tip {
    pub fn new(text: &'static str, theme: &Theme, ui: &UiFont) -> Self {
        Tip {
            text,
            size: ui.px(11.5),
            color: hsla(theme.fg(|t| t.agents_text), 1.0),
            background: hsla(theme.bg(|t| t.agents_bg), 1.0),
            border: hsla(theme.fg(|t| t.agents_rule), 1.0),
        }
    }
}

impl Render for Tip {
    fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        div()
            .relative()
            .px(px(7.0))
            .py(px(3.0))
            .rounded(px(6.0))
            .border_1()
            .border_color(self.border)
            .bg(self.background)
            .shadow_md()
            .text_size(self.size)
            .text_color(self.color)
            .child(self.text)
            .child(browser::cover())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 1440-point window, the left sidebar at 380 with its divider and this one's.
    fn room() -> Room {
        Room {
            window: 1440.0,
            others: 382.0,
            min: MIN_WIDTH,
        }
    }

    #[test]
    fn its_width_keeps_within_reach_and_leaves_the_terminal_room() {
        let room = room();
        assert_eq!(room.clamp(500.0), 500.0);
        assert_eq!(room.clamp(100.0), MIN_WIDTH);
        // The width it first opens at is under what four tabs take: it opens at the least.
        assert_eq!(room.clamp(DEFAULT_WIDTH), MIN_WIDTH);
        // The terminal keeps its least: 1440 - 382 - 360.
        assert_eq!(room.clamp(1200.0), 698.0);
        assert_eq!(room.clamp(1440.0 - 382.0 - 300.0), 698.0);
        // With the left sidebar collapsed, 70% of the window is the limit.
        let wide = Room {
            others: 54.0,
            ..room
        };
        assert_eq!(wide.clamp(1200.0), 1008.0);
        // Too narrow a window for both: the panel keeps its least.
        let narrow = Room {
            window: 900.0,
            ..room
        };
        assert_eq!(narrow.clamp(DEFAULT_WIDTH), MIN_WIDTH);
        // A larger interface size raises its least.
        let large = Room { min: 443.0, ..room };
        assert_eq!(large.clamp(DEFAULT_WIDTH), 443.0);
        // Whole points.
        assert_eq!(room.clamp(500.4), 500.0);
    }

    #[test]
    fn its_least_width_keeps_the_four_tabs_and_the_buttons_on_one_line() {
        // Each label at 12.5 points in the system font, as Core Text measures it (P5-55), and
        // what the selected one gains at most by being heavier.
        let labels = [
            ("Changes", 51.54),
            ("Browser", 47.62),
            ("Kanban", 43.95),
            ("Cairn", 30.93),
        ];
        let heavier = 4.0;
        assert_eq!(
            Tab::ALL.map(Tab::label).as_slice(),
            labels.map(|(label, _)| label).as_slice()
        );
        // A tab: its padding, its icon, the room after it, its label. Two points between tabs.
        let tabs = labels
            .iter()
            .map(|(_, label)| 11.0 + 12.0 + 7.0 + label + 11.0)
            .sum::<f32>()
            + 2.0 * (labels.len() - 1) as f32
            + heavier;
        // The row: its left padding, the tabs, the room that stretches, the two buttons, its
        // right padding, two points between each.
        let row = 10.0 + tabs + 2.0 + 2.0 + 26.0 + 2.0 + 26.0 + 8.0;
        assert!(MIN_WIDTH >= row, "{MIN_WIDTH} < {row}");
        // And no wider than that asks, to a few points.
        assert!(MIN_WIDTH - row < 4.0, "{MIN_WIDTH} against {row}");
    }

    #[test]
    fn a_layout_file_from_before_the_cairn_tab_reads_as_it_did_and_cairn_is_kept() {
        for (word, tab) in [
            ("changes", Tab::Changes),
            ("browser", Tab::Browser),
            ("kanban", Tab::Kanban),
            ("cairn", Tab::Cairn),
        ] {
            let file = serde_json::json!({ "open": true, "width": 500.0, "tab": word });
            let saved: Saved = serde_json::from_value(file).unwrap();
            assert_eq!(
                saved,
                Saved {
                    open: true,
                    width: 500.0,
                    tab,
                    ..Saved::default()
                }
            );
            assert_eq!(serde_json::to_value(&saved).unwrap()["tab"], word);
            assert_eq!(RightPanel::new(&saved).saved(), saved);
        }
        // A width narrower than four tabs take stays as the file has it, and is drawn at the least.
        let narrow: Saved = serde_json::from_value(serde_json::json!({ "width": 320.0 })).unwrap();
        let panel = RightPanel::new(&narrow);
        assert_eq!(panel.saved().width, 320.0);
        assert_eq!(panel.shown(room()), MIN_WIDTH);
    }

    #[test]
    fn dragging_the_divider_follows_the_mouse_within_reach() {
        let room = room();
        let mut panel = RightPanel::new(&Saved {
            width: 500.0,
            ..Saved::default()
        });
        panel.press(room, 1000.0);
        assert!(panel.resizing());
        // Left widens it, right narrows it.
        assert!(panel.drag(room, 950.0));
        assert_eq!(panel.width, 550.0);
        assert!(panel.drag(room, 1300.0));
        assert_eq!(panel.width, MIN_WIDTH);
        assert!(panel.drag(room, 0.0));
        assert_eq!(panel.width, 698.0);
        assert!(!panel.drag(room, -50.0));
        assert!(panel.release());
        assert!(!panel.resizing());
        assert!(!panel.release());
        // Not pressed: moving does nothing.
        assert!(!panel.drag(room, 500.0));
        assert_eq!(panel.width, 698.0);
    }

    #[test]
    fn widening_goes_to_about_half_the_window_and_back() {
        let room = Room {
            others: 54.0,
            ..room()
        };
        let mut panel = RightPanel::new(&Saved {
            width: 500.0,
            ..Saved::default()
        });
        assert_eq!(panel.shown(room), 500.0);
        panel.wide = true;
        assert_eq!(panel.shown(room), 720.0);
        // Its own width is kept, and comes back.
        assert_eq!(panel.saved().width, 500.0);
        panel.wide = false;
        assert_eq!(panel.shown(room), 500.0);
        // Half the window is held to the limits too: here the terminal's least.
        panel.wide = true;
        assert_eq!(
            panel.shown(Room {
                others: 382.0,
                ..room
            }),
            698.0
        );
        // Never narrower than it was set.
        panel.wide = false;
        panel.width = 800.0;
        panel.wide = true;
        assert_eq!(panel.shown(room), 800.0);
        // Dragging while widened sets the width from what is shown and ends the widening.
        panel.width = DEFAULT_WIDTH;
        panel.press(room, 700.0);
        assert!(panel.drag(room, 690.0));
        assert_eq!(panel.width, 730.0);
        assert!(!panel.wide);
        panel.release();
    }
}
