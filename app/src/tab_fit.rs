//! How the title bar's tabs fit (P5-165, rework 1): every tab as wide as the others, the base
//! width while there is room, all narrowing together when there is not, each title kept whole
//! where it fits, else dropped to the part that tells it apart, else cut in the middle; at the
//! least width, what still has no room waits behind a `+N` menu at the bar's end, the active tab
//! always staying in the bar.
use crate::window::{Subject, agent_name, last_part};

/// Where a title's start may be cut off: just after one of these.
const BREAKS: [char; 4] = ['-', '_', '.', '/'];

/// The part of each tab's title that tells it from the other tabs' (`subjects`, in the bar's
/// order; `open`, the agents open in the window, as [`agent_name`] takes them): for an agent whose
/// short name another open agent shares, its group (`paddock` for `paddock/main`); for another
/// agent, its short name less the start it shares with another such tab's up to a word break
/// (`newagent` for `dev-newagent` beside `dev-fixes`); for a shell, its directory; anything else
/// as its title.
pub fn distinct(subjects: &[Subject], open: &[String]) -> Vec<String> {
    // The agents' short names that are theirs alone, the ones whose starts are compared.
    let own: Vec<Option<String>> = subjects
        .iter()
        .map(|subject| match subject {
            Subject::Agent(name) => match agent_name(name, open) {
                (None, short) => Some(short),
                (Some(_), _) => None,
            },
            _ => None,
        })
        .collect();
    subjects
        .iter()
        .zip(&own)
        .enumerate()
        .map(|(index, (subject, short))| match (subject, short) {
            (Subject::Agent(_), Some(short)) => {
                let cut = own
                    .iter()
                    .enumerate()
                    .filter(|(other, _)| *other != index)
                    .filter_map(|(_, other)| other.as_deref())
                    .map(|other| shared_start(short, other))
                    .max()
                    .unwrap_or(0);
                short[cut..].to_owned()
            }
            (Subject::Agent(name), None) => agent_name(name, open)
                .0
                .unwrap_or_default()
                .trim_end_matches('/')
                .to_owned(),
            (Subject::Shell { cwd, .. }, _) => last_part(cwd).to_owned(),
            (Subject::Command(command), _) => command.clone(),
            (Subject::Empty, _) => "empty".into(),
        })
        .collect()
}

/// How much of `name`'s start it shares with `other`'s, up to and including the last word break
/// in it; none when that would leave nothing of `name`.
fn shared_start(name: &str, other: &str) -> usize {
    let common = name
        .char_indices()
        .zip(other.chars())
        .take_while(|((_, a), b)| a == b)
        .last()
        .map_or(0, |((at, c), _)| at + c.len_utf8());
    match name[..common].rfind(BREAKS) {
        Some(at) if at + 1 < name.len() => at + 1,
        _ => 0,
    }
}

/// `text` cut in the middle to fit `room` as `width` measures it: as much of its start and its
/// end as fit around `…`, the start a letter longer; whole when it fits.
pub fn middle(text: &str, room: f32, width: impl Fn(&str) -> f32) -> String {
    if width(text) <= room {
        return text.to_owned();
    }
    let letters: Vec<char> = text.chars().collect();
    (1..letters.len())
        .rev()
        .map(|keep| {
            let (head, tail) = (keep.div_ceil(2), keep / 2);
            letters[..head]
                .iter()
                .chain(['…'].iter())
                .chain(&letters[letters.len() - tail..])
                .collect::<String>()
        })
        .find(|cut| width(cut) <= room)
        .unwrap_or_else(|| "…".into())
}

/// A tab's widths, in points: its own around the title (the badge, the gaps, an inline ×), its
/// whole title, and the part of it that tells it apart ([`distinct`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TabSize {
    pub chrome: f32,
    pub full: f32,
    pub short: f32,
}

/// The bar the tabs share, in points: its room, the gap between tabs, every tab's width while
/// there is room, the least they narrow to, and the `+N` button's width.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bar {
    pub room: f32,
    pub gap: f32,
    pub widest: f32,
    pub least: f32,
    pub more: f32,
}

/// What a tab's title shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Title {
    Full,
    /// Only the part that tells it apart.
    Short,
    /// That part, cut in the middle to this much room.
    Squeezed(f32),
}

/// A tab in the bar: which, how wide, and what its title shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slot {
    pub tab: usize,
    pub width: f32,
    pub title: Title,
}

/// How the tabs fit: the ones in the bar, in order, and the ones behind `+N`.
#[derive(Clone, Debug, PartialEq)]
pub struct Fit {
    pub shown: Vec<Slot>,
    pub hidden: Vec<usize>,
}

impl TabSize {
    /// What its title shows when it is `width` wide: whole if it fits, else the part that tells
    /// it apart if that does, else that part cut to the room left.
    fn title(&self, width: f32) -> Title {
        if self.chrome + self.full <= width {
            Title::Full
        } else if self.chrome + self.short <= width {
            Title::Short
        } else {
            Title::Squeezed((width - self.chrome).max(0.0))
        }
    }
}

/// How the tabs `sizes` fit `bar` with the one at `active` active: all of them as wide as each
/// other, `bar.widest` while they fit at that, else narrowed together to fit; when that would
/// take them under `bar.least`, as many as fit at it beside the `+N` button, the active one and
/// the others from the first, share the room left, the rest behind `+N`. The active tab keeps its
/// place among them, or comes last when it would have been behind `+N`.
pub fn fit(sizes: &[TabSize], active: usize, bar: &Bar) -> Fit {
    // Each of `count` tabs' width to share out `room`, the gaps between them taken first.
    let width = |count: usize, room: f32| {
        let gaps = bar.gap * count.saturating_sub(1) as f32;
        ((room - gaps) / count.max(1) as f32).min(bar.widest)
    };
    let slots = |tabs: &[usize], width: f32| {
        tabs.iter()
            .map(|&tab| Slot {
                tab,
                width,
                title: sizes[tab].title(width),
            })
            .collect()
    };
    let all: Vec<usize> = (0..sizes.len()).collect();
    let even = width(all.len(), bar.room);
    if even >= bar.least {
        return Fit {
            shown: slots(&all, even),
            hidden: Vec::new(),
        };
    }
    let room = bar.room - bar.gap - bar.more;
    // `count` tabs at the least width take `count` widths and a gap fewer.
    let count = ((room + bar.gap) / (bar.least + bar.gap)).floor().max(1.0) as usize;
    let mut kept: Vec<usize> = all
        .iter()
        .copied()
        .filter(|&tab| tab != active)
        .take(count - 1)
        .collect();
    kept.push(active);
    kept.sort_unstable();
    Fit {
        shown: slots(&kept, width(kept.len(), room).max(bar.least)),
        hidden: all.into_iter().filter(|tab| !kept.contains(tab)).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(name: &str) -> Subject {
        Subject::Agent(name.into())
    }

    fn shell(cwd: &str) -> Subject {
        Subject::Shell {
            program: "/bin/zsh".into(),
            cwd: cwd.into(),
        }
    }

    /// The tabs the design draws, and the agents open with them.
    fn bar_tabs() -> (Vec<Subject>, Vec<String>) {
        let subjects = vec![
            agent("paddock/main"),
            agent("paddock/dev-newagent"),
            agent("paddock/dev-fixes"),
            shell("/Users/me/paddock"),
            agent("ranch/main"),
            agent("cairn/main"),
            shell("/Users/me/work/global-mesh/"),
        ];
        let open = subjects
            .iter()
            .filter_map(|subject| match subject {
                Subject::Agent(name) => Some(name.clone()),
                _ => None,
            })
            .collect();
        (subjects, open)
    }

    #[test]
    fn the_distinct_part_is_the_group_on_a_clash_the_rest_after_a_shared_start_or_the_dir() {
        let (subjects, open) = bar_tabs();
        assert_eq!(
            distinct(&subjects, &open),
            [
                "paddock",
                "newagent",
                "fixes",
                "paddock",
                "ranch",
                "cairn",
                "global-mesh"
            ]
        );
    }

    #[test]
    fn names_that_clash_with_nothing_and_share_no_start_stay_as_they_are() {
        let subjects = vec![
            agent("paddock/main"),
            agent("ranch/fixes"),
            agent("solo"),
            Subject::Command("htop -d 5".into()),
            Subject::Empty,
        ];
        let open = vec![
            "paddock/main".to_owned(),
            "ranch/fixes".to_owned(),
            "solo".to_owned(),
        ];
        assert_eq!(
            distinct(&subjects, &open),
            ["main", "fixes", "solo", "htop -d 5", "empty"]
        );
    }

    #[test]
    fn a_shared_start_is_cut_only_at_a_word_break_and_never_to_nothing() {
        // `dev-new` is shared, but only `dev-` ends at a break.
        assert_eq!(shared_start("dev-newagent", "dev-newer"), 4);
        assert_eq!(shared_start("dev-a-x", "dev-a-y"), 6);
        // No break in the shared start, or nothing after it.
        assert_eq!(shared_start("newagent", "newer"), 0);
        assert_eq!(shared_start("dev-", "dev-fixes"), 0);
        assert_eq!(shared_start("fixes", "main"), 0);
        let subjects = vec![agent("dev-fixes"), agent("ops-fixes"), agent("dev-")];
        let open: Vec<String> = ["dev-fixes", "ops-fixes", "dev-"].map(String::from).into();
        assert_eq!(distinct(&subjects, &open), ["fixes", "ops-fixes", "dev-"]);
    }

    /// Eight points a letter.
    fn letters(text: &str) -> f32 {
        text.chars().count() as f32 * 8.0
    }

    #[test]
    fn a_title_is_cut_in_the_middle_keeping_a_letter_more_of_its_start() {
        assert_eq!(middle("newagent", 7.0 * 8.0, letters), "new…ent");
        assert_eq!(middle("global-mesh", 7.0 * 8.0, letters), "glo…esh");
        assert_eq!(middle("global-mesh", 6.0 * 8.0, letters), "glo…sh");
        assert_eq!(middle("fixes", 5.0 * 8.0, letters), "fixes");
        assert_eq!(middle("fixes", 1.0, letters), "…");
    }

    const BAR: Bar = Bar {
        room: 0.0,
        gap: 4.0,
        widest: 180.0,
        least: 110.0,
        more: 40.0,
    };

    /// Tabs 40 points around titles of these many letters, whole and short.
    fn sizes(titles: &[(usize, usize)]) -> Vec<TabSize> {
        titles
            .iter()
            .map(|&(full, short)| TabSize {
                chrome: 40.0,
                full: full as f32 * 8.0,
                short: short as f32 * 8.0,
            })
            .collect()
    }

    /// Seven tabs, the active one among them included.
    fn seven() -> Vec<TabSize> {
        sizes(&[
            (12, 7),
            (12, 8),
            (9, 5),
            (13, 7),
            (10, 5),
            (10, 5),
            (17, 11),
        ])
    }

    fn order(fit: &Fit) -> Vec<usize> {
        fit.shown.iter().map(|slot| slot.tab).collect()
    }

    fn widths(fit: &Fit) -> Vec<f32> {
        fit.shown.iter().map(|slot| slot.width).collect()
    }

    #[test]
    fn with_room_every_tab_is_the_base_width_whatever_its_title() {
        // Titles of 3, 12 and 17 letters: 64, 136 and 176 points whole.
        let tabs = sizes(&[(3, 3), (12, 7), (17, 11)]);
        let fit = fit(
            &tabs,
            1,
            &Bar {
                room: 1000.0,
                ..BAR
            },
        );
        assert!(fit.hidden.is_empty());
        assert_eq!(widths(&fit), [180.0, 180.0, 180.0]);
        assert!(fit.shown.iter().all(|slot| slot.title == Title::Full));
        // A title wider than the base width drops to its distinct part, the active one too.
        let tabs = sizes(&[(20, 7), (20, 5)]);
        let fit = super::fit(
            &tabs,
            1,
            &Bar {
                room: 1000.0,
                ..BAR
            },
        );
        assert_eq!(widths(&fit), [180.0, 180.0]);
        assert_eq!(fit.shown[0].title, Title::Short);
        assert_eq!(fit.shown[1].title, Title::Short);
    }

    #[test]
    fn short_of_room_every_tab_narrows_together() {
        let tabs = sizes(&[(12, 7), (12, 8), (9, 5)]);
        // Three at 120 and the gaps.
        let fit = fit(
            &tabs,
            2,
            &Bar {
                room: 3.0 * 120.0 + 2.0 * 4.0,
                ..BAR
            },
        );
        assert!(fit.hidden.is_empty());
        assert_eq!(widths(&fit), [120.0, 120.0, 120.0]);
        // 136 whole does not fit in 120, 96 short does; the active one's 112 whole does.
        let titles: Vec<Title> = fit.shown.iter().map(|slot| slot.title).collect();
        assert_eq!(titles, [Title::Short, Title::Short, Title::Full]);
    }

    #[test]
    fn narrowed_titles_drop_to_their_distinct_part_then_are_cut() {
        let tab = sizes(&[(12, 9)])[0];
        assert_eq!(tab.title(136.0), Title::Full);
        assert_eq!(tab.title(135.0), Title::Short);
        assert_eq!(tab.title(112.0), Title::Short);
        assert_eq!(tab.title(111.0), Title::Squeezed(71.0));
    }

    #[test]
    fn they_narrow_to_the_least_width_and_no_further() {
        let tabs = seven();
        // Seven at the least, and the gaps: all of them still.
        let room = 7.0 * 110.0 + 6.0 * 4.0;
        let fit = fit(&tabs, 2, &Bar { room, ..BAR });
        assert!(fit.hidden.is_empty());
        assert!(
            widths(&fit)
                .iter()
                .all(|&width| (width - 110.0).abs() < 0.01)
        );
        // A little less: some wait behind `+N` rather than any narrowing under it.
        let fit = super::fit(
            &tabs,
            2,
            &Bar {
                room: room - 6.0,
                ..BAR
            },
        );
        assert!(!fit.hidden.is_empty());
        assert!(widths(&fit).iter().all(|&width| width >= 110.0));
    }

    #[test]
    fn with_no_room_at_the_least_width_the_rest_wait_behind_more() {
        let tabs = seven();
        // Three at the least, the `+N` button and the gaps.
        let room = 3.0 * 110.0 + 40.0 + 3.0 * 4.0;
        let fit = fit(&tabs, 2, &Bar { room, ..BAR });
        assert_eq!(order(&fit), [0, 1, 2]);
        assert_eq!(fit.hidden, [3, 4, 5, 6]);
        assert_eq!(widths(&fit), [110.0, 110.0, 110.0]);
    }

    #[test]
    fn tabs_kept_beside_more_share_the_room_it_leaves_evenly() {
        let tabs = seven();
        // Room for three at the least and a little over, not enough for a fourth.
        let room = 3.0 * 110.0 + 40.0 + 3.0 * 4.0 + 60.0;
        let fit = fit(&tabs, 2, &Bar { room, ..BAR });
        assert_eq!(order(&fit), [0, 1, 2]);
        assert_eq!(widths(&fit), [130.0, 130.0, 130.0]);
    }

    #[test]
    fn the_active_tab_is_never_behind_more_it_takes_the_last_place_shown() {
        let tabs = seven();
        let room = 3.0 * 110.0 + 40.0 + 3.0 * 4.0;
        let fit = fit(&tabs, 6, &Bar { room, ..BAR });
        assert_eq!(order(&fit), [0, 1, 6]);
        assert_eq!(fit.hidden, [2, 3, 4, 5]);
        assert_eq!(widths(&fit), [110.0, 110.0, 110.0]);
        // No room for anything beside it: it alone, at the least width.
        let fit = super::fit(&tabs, 4, &Bar { room: 60.0, ..BAR });
        assert_eq!(order(&fit), [4]);
        assert_eq!(fit.hidden, [0, 1, 2, 3, 5, 6]);
        assert_eq!(widths(&fit), [110.0]);
    }
}
