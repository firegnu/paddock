//! A sandboxed agent on its card (DESIGN §13 P5-120): the open card's Sandbox cell, and the
//! Blocked popover it opens: what fence's wall stopped in the agent's run, newest first, as
//! `fence denials` gives it. It opens on the network and file-write blocks the card counts; All
//! adds reads and the rest. It follows each read while open.
use super::*;
use crate::{
    fence::{Denial, Denials, Refusal},
    popover,
};
use gpui::uniform_list;

/// A press on the cell's button, which opens the popover.
type OnPress = Box<dyn Fn(&MouseDownEvent, &mut Window, &mut App)>;

/// The open card's Sandbox cell: the profile, what was stopped, the press that opens the popover,
/// and the popover while it is open.
pub(super) struct SandboxCell {
    pub profile: String,
    pub told: String,
    /// Something was stopped: the words are yellow.
    pub counted: bool,
    pub open: bool,
    pub on_press: OnPress,
    pub menu: Option<AnyElement>,
}

/// The popover's width, and one row's height, at the base interface size.
const WIDTH: f32 = 500.0;
const ROW: f32 = 46.0;
/// The most rows it shows before it scrolls.
const ROWS: usize = 8;

const FILES_NOTE: &str = "Network blocks are all here. File blocks may be missing; the agent's terminal shows every one.";

/// `t`, seconds since the epoch, as this Mac's clock reads: `14:06`.
pub(super) fn clock_of(t: f64) -> String {
    let at = t.floor() as i64;
    let local = at + crate::activity::local_offset(at);
    let minutes = local.rem_euclid(86_400) / 60;
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

/// The blocks to list, newest first: the counted ones, or `all`.
pub(super) fn listed(denials: &Denials, all: bool) -> Vec<&Denial> {
    denials
        .denials
        .iter()
        .rev()
        .filter(|denial| all || denial.counted())
        .collect()
}

/// What an empty list says: nothing blocked yet; but when fence gave as many records as it was
/// asked for, only that none of those is listed, as older ones were not read.
pub(super) fn nothing_listed(denials: Option<&Denials>) -> String {
    match denials {
        Some(denials) if denials.capped() => format!(
            "None among the latest {} records; older ones aren't read.",
            crate::fence::LIMIT
        ),
        _ => "Nothing blocked yet.".into(),
    }
}

/// One row as drawn: the time, the type's tag and its kind, what it was after (`~` for home; the
/// detail when fence did not know), and the detail under it; hovering shows fence's words whole,
/// as both lines may be cut to the row.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Row {
    pub time: String,
    pub tag: String,
    pub kind: String,
    pub target: String,
    pub whole: String,
    pub detail: Option<String>,
}

pub(super) fn row(denial: &Denial, home: Option<&str>) -> Row {
    let detail = denial.detail.clone().filter(|detail| !detail.is_empty());
    let (target, whole, detail) = match &denial.target {
        Some(target) => {
            let whole = match &detail {
                Some(detail) => format!("{target}\n{detail}"),
                None => target.clone(),
            };
            (card::tilde(target, home), whole, detail)
        }
        // Not known: the detail says what it was.
        None => {
            let said = detail.unwrap_or_else(|| "—".into());
            (said.clone(), said, None)
        }
    };
    Row {
        time: clock_of(denial.t),
        tag: denial.tag(),
        kind: denial.kind.clone(),
        target,
        whole,
        detail,
    }
}

/// The sandboxed agents' runs to ask fence about, by name and instance.
pub(super) fn runs(agents: &[Agent]) -> Vec<(String, String)> {
    agents
        .iter()
        .filter(|a| a.sandbox.is_some())
        .filter_map(|a| Some((a.name.clone(), a.instance.clone()?)))
        .collect()
}

/// The open card's last row, across both columns on a faint accent ground: `SANDBOX` over the
/// lock, the profile and what was stopped, which opens the Blocked popover.
pub(super) fn cell(theme: &Theme, ui: &UiFont, sandbox: SandboxCell) -> Div {
    let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
    let (text, dim, dimmer, accent) = (
        fg(|t| t.agents_text),
        fg(|t| t.agents_dim),
        fg(|t| t.agents_dimmer),
        fg(|t| t.agents_accent),
    );
    let told = if sandbox.counted {
        fg(|t| t.agents_yellow)
    } else {
        dim
    };
    let (ground, lit) = (accent.opacity(0.07), accent.opacity(0.13));
    div().relative().child(
        div()
            .id("card-sandbox")
            .mx(-ui.px(CHIP_X))
            .px(ui.px(CHIP_X))
            .py(ui.px(7.0))
            .flex()
            .flex_col()
            .gap(px(2.0))
            .rounded(ui.px(8.0))
            .bg(if sandbox.open { lit } else { ground })
            .cursor_pointer()
            .hover(move |style| style.bg(lit))
            .on_mouse_down(MouseButton::Left, sandbox.on_press)
            // The press opened the popover: its click is not one on the card.
            .on_click(|_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .whitespace_nowrap()
                    .text_size(ui.px(CELL_LABEL_SIZE))
                    .text_color(dimmer.opacity(0.8))
                    .child("SANDBOX"),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(ui.px(MARK_GAP))
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .text_size(ui.px(SECOND_SIZE))
                    .child(footer_icon::lock(false, dim, None, ui.scale(LOCK)))
                    .child(div().text_color(text).child(sandbox.profile))
                    .child(div().text_color(dimmer).child("·"))
                    .child(div().text_color(told).child(sandbox.told)),
            )
            .children(sandbox.menu),
    )
}

impl Sidebar {
    /// The open card's Sandbox cell, for a sandboxed agent.
    pub(super) fn sandbox_cell(&self, card: &Card, cx: &mut Context<Self>) -> Option<SandboxCell> {
        let sandbox = card.sandbox.as_ref()?;
        let ui = UiFont::get(cx);
        let open = matches!(&self.floating, Some(Floating::Denials(name)) if *name == card.name);
        let name = card.name.clone();
        let on_press: OnPress = Box::new(cx.listener(
            move |this, event: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                this.denials_all = false;
                this.press_menu(Floating::Denials(name.clone()), event, window, cx);
            },
        ));
        Some(SandboxCell {
            profile: sandbox.profile.clone(),
            told: sandbox.told(),
            counted: sandbox.count().is_some(),
            open,
            on_press,
            menu: open
                .then(|| Self::hung(self.denials_popover(card, &ui, cx), true).into_any_element()),
        })
    }

    /// The Blocked popover of `card`'s agent.
    fn denials_popover(
        &self,
        card: &Card,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<Div> {
        let theme = &*self.theme;
        let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
        let (text, dim, dimmer, accent, yellow, red, rule) = (
            fg(|t| t.agents_text),
            fg(|t| t.agents_dim),
            fg(|t| t.agents_dimmer),
            fg(|t| t.agents_accent),
            fg(|t| t.agents_yellow),
            fg(|t| t.agents_red),
            popover::edge_rule(theme),
        );
        let profile = card
            .sandbox
            .as_ref()
            .map(|s| s.profile.clone())
            .unwrap_or_default();
        let agent = self
            .listing
            .panel
            .agents
            .iter()
            .find(|a| a.name == card.name);
        let read: Option<&Result<Option<Denials>, Refusal>> = agent
            .and_then(|a| a.instance.as_deref())
            .and_then(|instance| self.listing.panel.fence.of(&card.name, instance));
        let denials = match read {
            Some(Ok(Some(denials))) => Some(denials),
            _ => None,
        };
        let all = self.denials_all;
        let more = |n: usize| match denials {
            Some(d) if d.capped() => format!("{n}+"),
            _ => n.to_string(),
        };
        let counted = denials.map_or(0, Denials::blocked);
        let total = denials.map_or(0, |d| d.denials.len());
        let rows: Vec<Row> = denials
            .map(|d| {
                listed(d, all)
                    .into_iter()
                    .map(|denial| row(denial, self.home.as_deref()))
                    .collect()
            })
            .unwrap_or_default();

        let close = div()
            .id("denials-close")
            .flex_shrink_0()
            .size(ui.px(24.0))
            .ml(ui.px(4.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded(ui.px(6.0))
            .cursor_pointer()
            .hover(move |style| style.bg(text.opacity(0.08)))
            .child(footer_icon::icon(
                Icon::Close,
                dim,
                ui.scale(12.0) / footer_icon::SIZE,
            ))
            .on_click(
                cx.listener(|this, _: &ClickEvent, window, cx| this.close_floating(window, cx)),
            );
        let head = div()
            .flex()
            .items_center()
            .gap(ui.px(8.0))
            .px(ui.px(16.0))
            .pt(ui.px(14.0))
            .pb(ui.px(12.0))
            .border_b_1()
            .border_color(rule)
            .child(footer_icon::lock(
                false,
                accent,
                Some(accent.opacity(0.2)),
                ui.scale(15.0),
            ))
            .child(
                div()
                    .text_size(ui.px(14.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Blocked"),
            )
            .child(
                div()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .text_ellipsis()
                    .text_size(ui.px(12.5))
                    .text_color(dimmer)
                    .child(format!("{} · {profile}", card.name)),
            )
            .child(div().flex_1())
            .child(
                div()
                    .flex_shrink_0()
                    .text_size(ui.px(12.0))
                    .text_color(dimmer)
                    .child(more(rows.len())),
            )
            .child(close);

        let toggle =
            |id: &'static str, label: &'static str, count: String, on: bool, tint: Hsla| {
                div()
                    .id(id)
                    .h(ui.px(24.0))
                    .px(ui.px(9.0))
                    .flex()
                    .items_center()
                    .gap(ui.px(5.0))
                    .rounded(ui.px(6.0))
                    .text_size(ui.px(12.0))
                    .cursor_pointer()
                    .map(|button| {
                        if on {
                            button.bg(text.opacity(0.1)).text_color(text)
                        } else {
                            button
                                .text_color(dim)
                                .hover(move |style| style.bg(text.opacity(0.05)))
                        }
                    })
                    .child(label)
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(tint)
                            .child(count),
                    )
            };
        let choose = div()
            .flex()
            .items_center()
            .gap(ui.px(8.0))
            .px(ui.px(16.0))
            .pt(ui.px(10.0))
            .pb(ui.px(2.0))
            .child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .gap(ui.px(2.0))
                    .p(ui.px(2.0))
                    .rounded(ui.px(8.0))
                    .bg(gpui::black().opacity(0.2))
                    .child(
                        toggle(
                            "denials-counted",
                            "Network & writes",
                            more(counted),
                            !all,
                            yellow,
                        )
                        .on_click(cx.listener(
                            |this, _: &ClickEvent, _, cx| {
                                this.denials_all = false;
                                cx.notify();
                            },
                        )),
                    )
                    .child(
                        toggle("denials-all", "All", more(total), all, dimmer).on_click(
                            cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.denials_all = true;
                                cx.notify();
                            }),
                        ),
                    ),
            )
            .child(
                div()
                    .min_w(px(0.0))
                    .text_size(ui.px(11.5))
                    .text_color(dimmer)
                    .child("All adds reads and system checks, mostly caches"),
            );

        let quiet = |words: String, color: Hsla| {
            div()
                .px(ui.px(16.0))
                .py(ui.px(14.0))
                .text_size(ui.px(12.5))
                .text_color(color)
                .child(words)
                .into_any_element()
        };
        let body = match read {
            Some(Err(refusal)) => quiet(format!("Couldn't read what was blocked: {refusal}"), red),
            _ if rows.is_empty() => quiet(nothing_listed(denials), dim),
            _ => {
                let row_height = ui.scale(ROW);
                let count = rows.len();
                let rows = Rc::new(rows);
                let colors = TagColors::new(theme);
                let (mono, size) = (self.mono.clone(), ui.px(12.5));
                let note = ui.px(11.5);
                let shown = count.min(ROWS) as f32 * row_height;
                let tip_colors = (
                    text,
                    hsla(theme.bg(|t| t.agents_bg), 1.0),
                    fg(|t| t.agents_rule),
                );
                uniform_list(
                    "denials",
                    count,
                    cx.processor(move |_, range: std::ops::Range<usize>, _, _| {
                        range
                            .filter_map(|index| Some((index, rows.get(index)?.clone())))
                            .map(|(index, row)| {
                                let (ink, ground) = colors.tag(&row.kind);
                                let tip = Tip {
                                    text: row.whole.clone().into(),
                                    size: note,
                                    color: tip_colors.0,
                                    background: tip_colors.1,
                                    border: tip_colors.2,
                                };
                                div()
                                    .id(("denial", index))
                                    .w_full()
                                    .h(px(row_height))
                                    .px(px(8.0))
                                    .flex()
                                    .items_center()
                                    .gap(px(10.0))
                                    .rounded(px(8.0))
                                    .hover(move |style| style.bg(text.opacity(0.04)))
                                    .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
                                    .child(
                                        div()
                                            .flex_shrink_0()
                                            .w(px(40.0))
                                            .text_size(note)
                                            .text_color(dimmer)
                                            .child(row.time),
                                    )
                                    .child(
                                        div().flex_shrink_0().w(px(64.0)).child(
                                            div()
                                                .flex()
                                                .flex_shrink_0()
                                                .h(px(20.0))
                                                .px(px(7.0))
                                                .items_center()
                                                .rounded(px(5.0))
                                                .bg(ground)
                                                .text_size(note)
                                                .text_color(ink)
                                                .whitespace_nowrap()
                                                .overflow_hidden()
                                                .child(row.tag),
                                        ),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w(px(0.0))
                                            .flex()
                                            .flex_col()
                                            .gap(px(2.0))
                                            .child(
                                                div()
                                                    .overflow_hidden()
                                                    .whitespace_nowrap()
                                                    .text_ellipsis()
                                                    .font(mono.clone())
                                                    .text_size(size)
                                                    .text_color(text)
                                                    .child(row.target),
                                            )
                                            .children(row.detail.map(|detail| {
                                                div()
                                                    .overflow_hidden()
                                                    .whitespace_nowrap()
                                                    .text_ellipsis()
                                                    .text_size(note)
                                                    .text_color(dimmer)
                                                    .child(detail)
                                            })),
                                    )
                            })
                            .collect()
                    }),
                )
                .h(px(shown))
                .mx(ui.px(8.0))
                .my(ui.px(6.0))
                .into_any_element()
            }
        };
        let stopped = denials.filter(|d| d.truncated).map(|_| {
            div()
                .px(ui.px(16.0))
                .pb(ui.px(8.0))
                .text_size(ui.px(12.0))
                .text_color(yellow)
                .child(
                    "fence stopped recording: this run's log is full, so later blocks are missing.",
                )
        });
        let foot = div()
            .mx(ui.px(16.0))
            .pt(ui.px(10.0))
            .pb(ui.px(13.0))
            .border_t_1()
            .border_color(rule)
            .flex()
            .items_start()
            .gap(ui.px(8.0))
            .text_size(ui.px(12.0))
            .line_height(relative(1.5))
            .text_color(dim)
            .child(div().flex_1().min_w(px(0.0)).child(FILES_NOTE));
        self.floated(popover::panel(theme, ui), "denials", cx)
            .w(ui.px(WIDTH))
            .mt(ui.px(4.0))
            .p(px(0.0))
            .whitespace_normal()
            .child(head)
            .child(choose)
            .child(body)
            .children(stopped)
            .child(foot)
    }
}

/// Each type's tag colours: network blue, writes red, reads yellow, the rest quiet.
#[derive(Clone, Copy)]
struct TagColors {
    network: Hsla,
    write: Hsla,
    read: Hsla,
    other: Hsla,
}

impl TagColors {
    fn new(theme: &Theme) -> Self {
        let fg = |pick: Pick| hsla(theme.fg(pick), 1.0);
        Self {
            network: fg(|t| t.agents_blue),
            write: fg(|t| t.agents_red),
            read: fg(|t| t.agents_yellow),
            other: fg(|t| t.agents_dim),
        }
    }

    /// The ink and the ground of `kind`'s tag.
    fn of_kind(self, kind: &str) -> Hsla {
        match kind {
            "network" => self.network,
            "file-write" => self.write,
            "file-read" => self.read,
            _ => self.other,
        }
    }

    fn tag(self, kind: &str) -> (Hsla, Hsla) {
        let ink = self.of_kind(kind);
        (ink, ink.opacity(0.14))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn denial(t: f64, kind: &str, target: Option<&str>, detail: Option<&str>) -> Denial {
        Denial {
            t,
            kind: kind.into(),
            target: target.map(str::to_owned),
            detail: detail.map(str::to_owned),
        }
    }

    #[test]
    fn only_sandboxed_runs_are_asked_about() {
        let agent = |name: &str, sandbox: bool, instance: Option<&str>| Agent {
            name: name.into(),
            instance: instance.map(str::to_owned),
            sandbox: sandbox.then(|| crate::corral::Sandbox {
                profile: "dev".into(),
            }),
            ..Agent::default()
        };
        let agents = [
            agent("a", true, Some("1")),
            agent("b", false, Some("2")),
            agent("c", true, None),
        ];
        assert_eq!(runs(&agents), [("a".to_owned(), "1".to_owned())]);
    }

    #[test]
    fn newest_first_the_counted_ones_or_all() {
        let denials = Denials {
            denials: vec![
                denial(1.0, "network", Some("a:443"), None),
                denial(2.0, "file-read", Some("/r"), None),
                denial(3.0, "file-write", Some("/w"), None),
                denial(4.0, "other", None, Some("mach-lookup")),
            ],
            ..Denials::default()
        };
        let times = |list: Vec<&Denial>| list.iter().map(|d| d.t).collect::<Vec<_>>();
        assert_eq!(times(listed(&denials, false)), [3.0, 1.0]);
        assert_eq!(times(listed(&denials, true)), [4.0, 3.0, 2.0, 1.0]);
    }

    #[test]
    fn a_row_shortens_home_and_falls_back_to_the_detail() {
        let row = row(
            &denial(
                0.0,
                "file-write",
                Some("/Users/u/w/card.rs"),
                Some("deny file-write-data"),
            ),
            Some("/Users/u"),
        );
        assert_eq!(row.target, "~/w/card.rs");
        assert_eq!(row.whole, "/Users/u/w/card.rs\ndeny file-write-data");
        assert_eq!(row.detail.as_deref(), Some("deny file-write-data"));
        assert_eq!(row.tag, "Write");
        let unknown = super::row(&denial(0.0, "mystery", None, Some("what it was")), None);
        assert_eq!(unknown.target, "what it was");
        assert_eq!(
            (unknown.whole.as_str(), unknown.detail),
            ("what it was", None)
        );
        assert_eq!(unknown.tag, "mystery");
        let network = super::row(
            &denial(0.0, "network", Some("github.com:443"), None),
            Some("/Users/u"),
        );
        assert_eq!(
            (network.target.as_str(), network.whole.as_str()),
            ("github.com:443", "github.com:443")
        );
        // Outside home, or a long detail in place of the target: still all there on hover.
        let far = "/opt/somewhere/very/deep/down/in/a/tree/that/the/row/cuts/short/file.lock";
        let outside = super::row(
            &denial(0.0, "file-write", Some(far), None),
            Some("/Users/u"),
        );
        assert_eq!(
            (outside.target.as_str(), outside.whole.as_str()),
            (far, far)
        );
        let long = "mach-lookup com.apple.something.long.enough.to.be.cut.in.the.row (global-name)";
        let unnamed = super::row(&denial(0.0, "other", None, Some(long)), None);
        assert_eq!(
            (unnamed.target.as_str(), unnamed.whole.as_str()),
            (long, long)
        );
    }

    #[test]
    fn an_empty_list_of_a_full_read_doesnt_say_nothing_was_blocked() {
        assert_eq!(nothing_listed(None), "Nothing blocked yet.");
        let few = Denials {
            denials: vec![denial(1.0, "file-read", Some("/r"), None)],
            ..Denials::default()
        };
        assert_eq!(nothing_listed(Some(&few)), "Nothing blocked yet.");
        let full = Denials {
            denials: vec![denial(1.0, "file-read", Some("/r"), None); crate::fence::LIMIT],
            ..Denials::default()
        };
        assert!(listed(&full, false).is_empty());
        assert_eq!(
            nothing_listed(Some(&full)),
            "None among the latest 5000 records; older ones aren't read."
        );
    }

    #[test]
    fn times_are_this_macs_clock() {
        // Whatever the zone, minutes past the hour stay and the shape is HH:MM.
        let t = 1_760_760_123.4;
        let shown = clock_of(t);
        assert_eq!(shown.len(), 5);
        assert_eq!(&shown[2..3], ":");
        let offset = crate::activity::local_offset(t as i64);
        let minutes = ((t as i64 + offset).rem_euclid(86_400)) / 60;
        assert_eq!(shown, format!("{:02}:{:02}", minutes / 60, minutes % 60));
    }
}
