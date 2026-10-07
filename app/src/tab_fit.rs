//! How the title bar's tabs fit when they are short of room: the active tab keeps its whole
//! title; the others first drop to the part of theirs that tells them apart, then shorten in the
//! middle down to a least width; what still has no room waits behind a `+N` menu at the bar's
//! end, the active tab always staying in the bar.
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

/// The bar the tabs share, in points: its room, the gap between tabs, the widest a tab grows,
/// the least a shortened one keeps, and the `+N` button's width.
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

/// How the tabs fit: the ones in the bar, in order, and the ones behind `+N`. `roomy` when they
/// all fit whole, to be drawn as they are without any width set.
#[derive(Clone, Debug, PartialEq)]
pub struct Fit {
    pub shown: Vec<Slot>,
    pub hidden: Vec<usize>,
    pub roomy: bool,
}

impl TabSize {
    fn whole(&self, widest: f32) -> f32 {
        (self.chrome + self.full).min(widest)
    }

    /// Its width and title, shortened, when it may be at most `cap` wide: the part that tells it
    /// apart, cut to `cap` when that is wider.
    fn at(&self, cap: f32) -> (f32, Title) {
        let part = self.chrome + self.short;
        if part <= cap {
            (part, Title::Short)
        } else {
            (cap, Title::Squeezed((cap - self.chrome).max(0.0)))
        }
    }
}

/// How the tabs `sizes` fit `bar` with the one at `active` active: whole if they can; else every
/// other tab shortened to the part that tells it apart, cut evenly where that is still too wide
/// but none narrower than `bar.least` (unless that part is), the active one whole; else as many
/// of the others as fit, from the first, that narrow beside it and the `+N` button, the rest
/// behind it. The active tab keeps its place among them, or comes last when it would have been
/// behind `+N`.
pub fn fit(sizes: &[TabSize], active: usize, bar: &Bar) -> Fit {
    let at = |tab: usize, cap: f32| {
        if tab == active {
            (sizes[tab].whole(bar.widest), Title::Full)
        } else {
            sizes[tab].at(cap)
        }
    };
    let total = |tabs: &[usize], cap: f32| {
        let gaps = bar.gap * tabs.len().saturating_sub(1) as f32;
        tabs.iter().map(|&tab| at(tab, cap).0).sum::<f32>() + gaps
    };
    // The widest every shortened tab may be, for `tabs` to fit `room`.
    let squeeze = |tabs: &[usize], room: f32| {
        let (mut low, mut high) = (bar.least, bar.widest);
        for _ in 0..24 {
            let cap = (low + high) / 2.0;
            if total(tabs, cap) <= room {
                low = cap;
            } else {
                high = cap;
            }
        }
        tabs.iter()
            .map(|&tab| {
                let (width, title) = at(tab, low);
                Slot { tab, width, title }
            })
            .collect()
    };
    let all: Vec<usize> = (0..sizes.len()).collect();
    let whole: Vec<Slot> = all
        .iter()
        .map(|&tab| Slot {
            tab,
            width: sizes[tab].whole(bar.widest),
            title: Title::Full,
        })
        .collect();
    let gaps = bar.gap * all.len().saturating_sub(1) as f32;
    if whole.iter().map(|slot| slot.width).sum::<f32>() + gaps <= bar.room {
        return Fit {
            shown: whole,
            hidden: Vec::new(),
            roomy: true,
        };
    }
    if total(&all, bar.least) <= bar.room {
        return Fit {
            shown: squeeze(&all, bar.room),
            hidden: Vec::new(),
            roomy: false,
        };
    }
    let room = bar.room - bar.gap - bar.more;
    let mut used = at(active, bar.least).0;
    let mut kept = vec![active];
    for tab in all.iter().copied().filter(|&tab| tab != active) {
        let width = at(tab, bar.least).0;
        if used + bar.gap + width > room {
            break;
        }
        used += bar.gap + width;
        kept.push(tab);
    }
    kept.sort_unstable();
    Fit {
        shown: squeeze(&kept, room),
        hidden: all.into_iter().filter(|tab| !kept.contains(tab)).collect(),
        roomy: false,
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
        widest: 220.0,
        least: 78.0,
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

    /// Seven tabs: the least width each, 78, but the third whole at 112 and the last at 176.
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

    #[test]
    fn tabs_that_fit_stay_whole() {
        // 136, 136, 112: 392 with the gaps.
        let tabs = sizes(&[(12, 7), (12, 8), (9, 5)]);
        let fit = fit(&tabs, 2, &Bar { room: 392.0, ..BAR });
        assert!(fit.roomy);
        assert!(fit.hidden.is_empty());
        assert!(fit.shown.iter().all(|slot| slot.title == Title::Full));
        assert_eq!(fit.shown[0].width, 136.0);
    }

    #[test]
    fn short_of_room_the_others_drop_to_their_distinct_part_and_the_active_stays_whole() {
        let tabs = sizes(&[(12, 7), (12, 8), (9, 5)]);
        // Short: 96 and 104, the active 112, and the gaps: 320.
        let fit = fit(&tabs, 2, &Bar { room: 320.0, ..BAR });
        assert!(!fit.roomy);
        let titles: Vec<Title> = fit.shown.iter().map(|slot| slot.title).collect();
        assert_eq!(titles, [Title::Short, Title::Short, Title::Full]);
        let widths: Vec<f32> = fit.shown.iter().map(|slot| slot.width).collect();
        assert_eq!(widths, [96.0, 104.0, 112.0]);
    }

    #[test]
    fn short_of_room_no_other_tab_keeps_its_whole_title_even_where_it_would_fit() {
        // Whole: 136, 88 and the active 112, 344 with the gaps; short: 96 and 72.
        let tabs = sizes(&[(12, 7), (6, 4), (9, 5)]);
        let fit = fit(&tabs, 2, &Bar { room: 320.0, ..BAR });
        let titles: Vec<Title> = fit.shown.iter().map(|slot| slot.title).collect();
        assert_eq!(titles, [Title::Short, Title::Short, Title::Full]);
        let widths: Vec<f32> = fit.shown.iter().map(|slot| slot.width).collect();
        assert_eq!(widths, [96.0, 72.0, 112.0]);
    }

    #[test]
    fn shorter_still_they_are_cut_to_an_even_width_but_never_under_the_least() {
        let tabs = sizes(&[(12, 7), (12, 8), (9, 5), (3, 3)]);
        // The active 112, the three-letter one at 64, two cut to 80 each, and the gaps.
        let room = 112.0 + 64.0 + 2.0 * 80.0 + 3.0 * 4.0;
        let fit = fit(&tabs, 2, &Bar { room, ..BAR });
        assert!(fit.hidden.is_empty());
        let slot = fit.shown[0];
        assert!((slot.width - 80.0).abs() < 0.01, "{slot:?}");
        assert!(matches!(slot.title, Title::Squeezed(text) if (text - 40.0).abs() < 0.01));
        assert_eq!(fit.shown[2].title, Title::Full);
        assert_eq!(fit.shown[3].title, Title::Short);
        assert!(fit.shown.iter().all(|slot| slot.width >= 64.0));
        // A little less room: one waits behind `+N` rather than any narrowing under the least.
        let fit = super::fit(
            &tabs,
            2,
            &Bar {
                room: room - 6.0,
                ..BAR
            },
        );
        assert_eq!(fit.hidden, [3]);
        assert!(fit.shown.iter().all(|slot| slot.width >= 78.0));
    }

    #[test]
    fn with_no_room_at_the_least_width_the_rest_wait_behind_more() {
        let tabs = seven();
        // The active, two others, the `+N` button and the gaps.
        let room = 112.0 + 2.0 * 78.0 + 40.0 + 3.0 * 4.0;
        let fit = fit(&tabs, 2, &Bar { room, ..BAR });
        assert_eq!(order(&fit), [0, 1, 2]);
        assert_eq!(fit.hidden, [3, 4, 5, 6]);
        assert_eq!(fit.shown[2].title, Title::Full);
    }

    #[test]
    fn the_active_tab_is_never_behind_more_it_takes_the_last_place_shown() {
        let tabs = seven();
        let room = 112.0 + 2.0 * 78.0 + 40.0 + 3.0 * 4.0;
        // The last tab active, wider than the others: the first two and it.
        let fit = fit(
            &tabs,
            6,
            &Bar {
                room: room + 64.0,
                ..BAR
            },
        );
        assert_eq!(order(&fit), [0, 1, 6]);
        assert_eq!(fit.hidden, [2, 3, 4, 5]);
        assert_eq!(fit.shown[2].title, Title::Full);
        assert_eq!(fit.shown[2].width, 176.0);
        // No room for anything beside it: it alone.
        let fit = super::fit(&tabs, 4, &Bar { room: 60.0, ..BAR });
        assert_eq!(order(&fit), [4]);
        assert_eq!(fit.hidden, [0, 1, 2, 3, 5, 6]);
    }

    #[test]
    fn tabs_kept_beside_more_share_the_room_it_leaves() {
        let tabs = seven();
        // Room for the active and two others at the least, and a little over.
        let room = 112.0 + 2.0 * 78.0 + 40.0 + 3.0 * 4.0 + 40.0;
        let fit = fit(&tabs, 2, &Bar { room, ..BAR });
        assert_eq!(order(&fit), [0, 1, 2]);
        let used: f32 = fit.shown.iter().map(|slot| slot.width).sum::<f32>() + 2.0 * 4.0;
        assert!(used <= room - 40.0 - 4.0 + 0.01);
        // The first drops to its distinct part, 96; the second is cut to the room left.
        assert_eq!(fit.shown[0].title, Title::Short);
        assert_eq!(
            fit.shown[1].title,
            Title::Squeezed(fit.shown[1].width - 40.0)
        );
    }
}
