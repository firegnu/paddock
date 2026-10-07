//! The left sidebar's activity grid, what it knows (DESIGN §13 P5-32): each day's commits in the
//! repositories the sidebar's agents work in, read from Git and never stored. The one thing kept
//! is the list of those repositories, in paddock's state directory beside the layout, so one stays
//! counted after its agents are gone; only their paths are written (`load_repos`, `save_repos`).
//! Git is read in the background with read-only commands, through [`git::git`]'s options
//! (`Poller`). `activity_view.rs` draws it.
use crate::{git, kanban, layout_state};
use std::{
    collections::{BTreeMap, HashMap},
    fs, io,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, RecvTimeoutError},
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

/// A calendar day: days since 1970-01-01, in the time zone it was counted in.
pub type Day = i64;

const DAY: i64 = 86_400;
/// The most weeks shown, however wide the sidebar.
pub const MAX_WEEKS: usize = 53;
/// The fewest commits in a day that reach the darkest level, when the busiest day shown has fewer.
pub const FULL: u32 = 8;
/// How long after the last read the repositories are read again.
pub const EVERY: Duration = Duration::from_secs(60);

pub const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

/// The day `time` (Unix seconds) falls on, `offset` seconds east of UTC.
pub fn day_of(time: i64, offset: i64) -> Day {
    (time + offset).div_euclid(DAY)
}

/// How far east of UTC this machine's time zone is at `time`, summer time included.
pub fn local_offset(time: i64) -> i64 {
    let at = time as libc::time_t;
    // SAFETY: `localtime_r` writes only the `tm` it is given, and both pointers outlive the call.
    unsafe {
        let mut tm: libc::tm = std::mem::zeroed();
        if libc::localtime_r(&at, &mut tm).is_null() {
            0
        } else {
            tm.tm_gmtoff as i64
        }
    }
}

/// Today, on this machine's calendar.
pub fn today() -> Day {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    day_of(now, local_offset(now))
}

/// The day of the week, Sunday 0 to Saturday 6.
pub fn weekday(day: Day) -> usize {
    (day + 4).rem_euclid(7) as usize
}

/// The year, month (1 to 12) and day of the month.
pub fn date(day: Day) -> (i64, u32, u32) {
    // Howard Hinnant's `civil_from_days`.
    let z = day + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// The day as the hover card names it: `Wed, Oct 7`.
pub fn label(day: Day) -> String {
    let (_, month, of) = date(day);
    format!(
        "{}, {} {of}",
        WEEKDAYS[weekday(day)],
        MONTHS[month as usize - 1]
    )
}

/// The first day of `weeks` weeks ending with the week of `today`: a Sunday.
pub fn first_day(today: Day, weeks: usize) -> Day {
    today - weekday(today) as i64 - 7 * (weeks.max(1) as i64 - 1)
}

/// How many weeks fit in `width` points, a column of cells `cell` wide each, `gap` apart; at least
/// one, at most [`MAX_WEEKS`].
pub fn weeks_for(width: f32, cell: f32, gap: f32) -> usize {
    (((width + gap) / (cell + gap)).floor().max(1.0) as usize).min(MAX_WEEKS)
}

/// The months named over `weeks` columns from `first`: a name over each column whose Sunday starts
/// a new month, at least three columns after the one before and two before the end, where it has
/// room; the first column named only when the next name is three columns away.
pub fn months(first: Day, weeks: usize) -> Vec<(usize, &'static str)> {
    let month = |column: usize| date(first + 7 * column as i64).1;
    let mut starts: Vec<usize> = (1..weeks).filter(|&c| month(c) != month(c - 1)).collect();
    if starts.first().is_none_or(|&c| c >= 3) && weeks >= 3 {
        starts.insert(0, 0);
    }
    let mut out: Vec<(usize, &'static str)> = Vec::new();
    for column in starts {
        if out.last().is_none_or(|&(last, _)| column >= last + 3) && column + 2 <= weeks {
            out.push((column, MONTHS[month(column) as usize - 1]));
        }
    }
    out
}

/// How many of `times` (Unix seconds) fall on each day from `first` to `last`, each counted on
/// the calendar `offset(time)` seconds east of UTC; the rest are left out.
pub fn bucket(
    times: impl IntoIterator<Item = i64>,
    offset: impl Fn(i64) -> i64,
    first: Day,
    last: Day,
) -> BTreeMap<Day, u32> {
    let mut days = BTreeMap::new();
    for time in times {
        let day = day_of(time, offset(time));
        if (first..=last).contains(&day) {
            *days.entry(day).or_insert(0) += 1;
        }
    }
    days
}

/// How dark a day with `commits` is drawn, 0 (none) to 4, against the busiest day shown: four
/// equal steps up to it, or up to [`FULL`] while the busiest has fewer.
pub fn level(commits: u32, busiest: u32) -> usize {
    if commits == 0 {
        return 0;
    }
    let top = busiest.max(FULL);
    (commits * 4).div_ceil(top).clamp(1, 4) as usize
}

/// One day's tally.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tally {
    /// Commits in each repository, in the order of [`Counts::repos`].
    pub by_repo: Vec<u32>,
    /// Wrap-up commits on main (`收尾: <id>`), in all the repositories.
    pub wrapped: u32,
}

impl Tally {
    pub fn commits(&self) -> u32 {
        self.by_repo.iter().sum()
    }
}

/// What one round of reading found.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub today: Day,
    /// The first day read.
    pub first: Day,
    /// The repositories read, by their directories' names.
    pub repos: Vec<String>,
    /// The days with any commit or wrap-up.
    pub days: BTreeMap<Day, Tally>,
}

impl Counts {
    pub fn commits(&self, day: Day) -> u32 {
        self.days.get(&day).map_or(0, Tally::commits)
    }

    pub fn wrapped(&self, day: Day) -> u32 {
        self.days.get(&day).map_or(0, |t| t.wrapped)
    }

    /// Commits from `from` to today.
    pub fn total(&self, from: Day) -> u32 {
        self.days.range(from..).map(|(_, t)| t.commits()).sum()
    }

    /// The most commits on one day from `from` to today.
    pub fn busiest(&self, from: Day) -> u32 {
        self.days
            .range(from..)
            .map(|(_, t)| t.commits())
            .max()
            .unwrap_or(0)
    }

    /// Days in a row with commits, up to today; while today has none yet, up to yesterday.
    pub fn streak(&self) -> u32 {
        let mut day = self.today;
        if self.commits(day) == 0 {
            day -= 1;
        }
        let mut streak = 0;
        while day >= self.first && self.commits(day) > 0 {
            streak += 1;
            day -= 1;
        }
        streak
    }

    /// The repositories with commits on `day`, the most first, and their commits.
    pub fn repos_on(&self, day: Day) -> Vec<(&str, u32)> {
        let Some(tally) = self.days.get(&day) else {
            return Vec::new();
        };
        let mut repos: Vec<(&str, u32)> = self
            .repos
            .iter()
            .map(String::as_str)
            .zip(tally.by_repo.iter().copied())
            .filter(|(_, n)| *n > 0)
            .collect();
        repos.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
        repos
    }
}

/// A day's repositories as its card lists them: all of them when there are at most `named` and
/// one more, else the first `named` and the rest put together (`None` for their name).
pub fn grouped<'a>(repos: &[(&'a str, u32)], named: usize) -> Vec<(Option<&'a str>, u32)> {
    if repos.len() <= named + 1 {
        return repos.iter().map(|&(name, n)| (Some(name), n)).collect();
    }
    let mut out: Vec<(Option<&str>, u32)> = repos[..named]
        .iter()
        .map(|&(name, n)| (Some(name), n))
        .collect();
    out.push((None, repos[named..].iter().map(|(_, n)| n).sum()));
    out
}

/// Reads each repository's commits from `first` to `today`: on all its local branches, each
/// commit once, on the day of its author time, `offset(time)` seconds east of UTC; and the
/// wrap-ups on its main. A repository Git cannot read counts nothing; `None` once cancelled.
pub fn count(
    program: &str,
    repos: &[PathBuf],
    first: Day,
    today: Day,
    offset: &dyn Fn(i64) -> i64,
    cancel: &AtomicBool,
) -> Option<Counts> {
    // Git picks by committer time, which is never much before the author time: two days early
    // takes in every time zone, and the days are then told by the author time.
    let since = format!("--max-age={}", (first - 2) * DAY);
    let mut counts = Counts {
        today,
        first,
        repos: Vec::new(),
        days: BTreeMap::new(),
    };
    let log = |repo: &Path, args: &[&str]| -> String {
        git::git(program, repo, args, cancel)
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .unwrap_or_default()
    };
    for (index, repo) in repos.iter().enumerate() {
        counts.repos.push(repo.file_name().map_or_else(
            || repo.display().to_string(),
            |n| n.to_string_lossy().into(),
        ));
        let commits = log(
            repo,
            &[
                "log",
                "--branches",
                "--no-show-signature",
                &since,
                "--format=%at",
            ],
        );
        let times = commits.lines().filter_map(|t| t.trim().parse().ok());
        for (day, n) in bucket(times, offset, first, today) {
            let tally = counts.days.entry(day).or_default();
            tally.by_repo.resize(repos.len(), 0);
            tally.by_repo[index] = n;
        }
        let main = format!("refs/heads/{}", kanban::MAIN);
        let line = log(
            repo,
            &[
                "log",
                "--first-parent",
                "--no-show-signature",
                &since,
                "--format=%at%x1f%s",
                &main,
                "--",
            ],
        );
        let wraps = line.lines().filter_map(|row| {
            let (time, subject) = row.split_once('\x1f')?;
            kanban::wrapped_id(subject)?;
            time.trim().parse().ok()
        });
        for (day, n) in bucket(wraps, offset, first, today) {
            let tally = counts.days.entry(day).or_default();
            tally.by_repo.resize(repos.len(), 0);
            tally.wrapped += n;
        }
        if cancel.load(Ordering::Relaxed) {
            return None;
        }
    }
    for tally in counts.days.values_mut() {
        tally.by_repo.resize(repos.len(), 0);
    }
    Some(counts)
}

/// The main worktree of the repository `cwd` is in, canonical: a worktree's or a subdirectory's
/// is its repository's own.
pub fn main_repository(program: &str, cwd: &str, cancel: &AtomicBool) -> Option<PathBuf> {
    if !Path::new(cwd).is_absolute() {
        return None;
    }
    let output = git::git(
        program,
        Path::new(cwd),
        &["worktree", "list", "--porcelain"],
        cancel,
    )
    .filter(|o| o.status.success())?;
    let list = String::from_utf8(output.stdout).ok()?;
    // The first worktree listed is the main one.
    let main = PathBuf::from(list.lines().next()?.strip_prefix("worktree ")?);
    Some(main.canonicalize().unwrap_or(main))
}

/// The list of repositories, beside the layout: `$XDG_STATE_HOME/paddock/activity-repos.json`.
pub fn repos_path() -> Option<PathBuf> {
    layout_state::default_path().map(|path| path.with_file_name("activity-repos.json"))
}

/// The repositories listed at `path` that are still there; none when there is no list or it
/// cannot be read.
pub fn load_repos(path: &Path) -> Vec<PathBuf> {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Vec<PathBuf>>(&bytes).ok())
        .unwrap_or_default()
        .into_iter()
        .filter(|repo| repo.is_dir())
        .collect()
}

/// Writes `repos` to `path`: their paths, nothing else.
pub fn save_repos(path: &Path, repos: &[PathBuf]) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let bytes = serde_json::to_vec_pretty(repos).map_err(io::Error::other)?;
    let temporary = path.with_extension("json.saving");
    fs::write(&temporary, bytes)?;
    fs::rename(&temporary, path)
}

/// Adds the repositories in `found` that `known` does not list, keeping it sorted; whether any
/// were added.
pub fn remember(known: &mut Vec<PathBuf>, found: impl IntoIterator<Item = PathBuf>) -> bool {
    let before = known.len();
    for repo in found {
        if !known.contains(&repo) {
            known.push(repo);
        }
    }
    known.sort();
    known.len() != before
}

/// What the grid asks for: the agents' directories, and how many weeks it shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ask {
    pub cwds: Vec<String>,
    pub weeks: usize,
}

/// Reads the repositories in the background: at once when asked something new, and every
/// [`EVERY`]; sends a round only when it differs from the last.
pub struct Poller {
    pub updates: mpsc::Receiver<Counts>,
    asked: Option<Ask>,
    wake: mpsc::Sender<Ask>,
    cancel: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}

impl Poller {
    /// `path` is where the list of repositories is kept; `None` keeps it only in memory.
    pub fn start(program: String, path: Option<PathBuf>, every: Duration) -> Self {
        let (tx, updates) = mpsc::channel();
        let (wake, wait) = mpsc::channel::<Ask>();
        let cancel = Arc::new(AtomicBool::new(false));
        let quitting = cancel.clone();
        let worker = thread::Builder::new()
            .name("paddock-activity".into())
            .spawn(move || {
                let mut repos = path.as_deref().map(load_repos).unwrap_or_default();
                let mut mains: HashMap<String, PathBuf> = HashMap::new();
                let mut ask: Option<Ask> = None;
                let mut counted: Option<(Day, Day, Vec<PathBuf>)> = None;
                let mut last: Option<Counts> = None;
                loop {
                    let mut due = match wait.recv_timeout(every) {
                        Ok(next) => {
                            ask = Some(next);
                            false
                        }
                        Err(RecvTimeoutError::Timeout) => true,
                        Err(RecvTimeoutError::Disconnected) => break,
                    };
                    if let Some(latest) = wait.try_iter().last() {
                        ask = Some(latest);
                    }
                    if quitting.load(Ordering::Relaxed) {
                        break;
                    }
                    let Some(current) = &ask else { continue };
                    let mut found = Vec::new();
                    for cwd in &current.cwds {
                        if mains.contains_key(cwd) {
                            continue;
                        }
                        if let Some(main) = main_repository(&program, cwd, &quitting) {
                            mains.insert(cwd.clone(), main.clone());
                            found.push(main);
                        }
                    }
                    // Another window may have added some since: theirs are kept too.
                    if let Some(path) = &path {
                        remember(&mut repos, load_repos(path));
                    }
                    if remember(&mut repos, found)
                        && let Some(path) = &path
                    {
                        let _ = save_repos(path, &repos);
                    }
                    let today = today();
                    let first = first_day(today, current.weeks);
                    let present: Vec<PathBuf> =
                        repos.iter().filter(|r| r.is_dir()).cloned().collect();
                    let key = (today, first, present);
                    due |= counted.as_ref() != Some(&key);
                    if !due {
                        continue;
                    }
                    let Some(counts) =
                        count(&program, &key.2, first, today, &local_offset, &quitting)
                    else {
                        break;
                    };
                    counted = Some(key);
                    if last.as_ref() != Some(&counts) {
                        last = Some(counts.clone());
                        if tx.send(counts).is_err() {
                            break;
                        }
                    }
                }
            })
            .ok();
        Self {
            updates,
            asked: None,
            wake,
            cancel,
            worker,
        }
    }

    /// Reads for `ask` at once when it is new.
    pub fn ask(&mut self, ask: Ask) {
        if self.asked.as_ref() != Some(&ask) {
            self.asked = Some(ask.clone());
            let _ = self.wake.send(ask);
        }
    }
}

impl Drop for Poller {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        let (wake, _) = mpsc::channel();
        // Dropping the only sender wakes the worker at once.
        drop(std::mem::replace(&mut self.wake, wake));
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-10-07, a Wednesday.
    const OCT_7: Day = 20_733;

    #[test]
    fn days_are_told_on_the_local_calendar() {
        assert_eq!(date(OCT_7), (2026, 10, 7));
        assert_eq!(weekday(OCT_7), 3);
        assert_eq!(label(OCT_7), "Wed, Oct 7");
        assert_eq!(date(0), (1970, 1, 1));
        assert_eq!(date(OCT_7 - 280), (2025, 12, 31));
        // 2026-10-06 23:30 UTC is already the 7th eight hours east, still the 6th in UTC, and
        // the 6th five hours west even at 03:00 UTC on the 7th.
        let late = OCT_7 * DAY - 30 * 60;
        assert_eq!(day_of(late, 8 * 3600), OCT_7);
        assert_eq!(day_of(late, 0), OCT_7 - 1);
        assert_eq!(day_of(OCT_7 * DAY + 3 * 3600, -5 * 3600), OCT_7 - 1);
    }

    #[test]
    fn commits_fall_on_their_days_across_midnight() {
        let offset = |_| 8 * 3600;
        // Local midnight between the 6th and the 7th, eight hours east.
        let midnight = OCT_7 * DAY - 8 * 3600;
        let times = [midnight - 1, midnight, midnight + 1, midnight - DAY * 40];
        let days = bucket(times, offset, OCT_7 - 7, OCT_7);
        assert_eq!(days, BTreeMap::from([(OCT_7 - 1, 1), (OCT_7, 2)]));
        // Each time on its own offset, as summer time changes it.
        let days = bucket(
            [midnight - 1],
            |t| if t < midnight { 9 * 3600 } else { 8 * 3600 },
            OCT_7 - 7,
            OCT_7,
        );
        assert_eq!(days, BTreeMap::from([(OCT_7, 1)]));
    }

    #[test]
    fn five_levels_split_at_quarters_of_the_busiest_day() {
        assert_eq!(level(0, 40), 0);
        // Quarters of 40: 1–10, 11–20, 21–30, 31–40.
        let levels: Vec<usize> = [1, 10, 11, 20, 21, 30, 31, 40]
            .into_iter()
            .map(|n| level(n, 40))
            .collect();
        assert_eq!(levels, [1, 1, 2, 2, 3, 3, 4, 4]);
        // A quiet stretch: one commit is not drawn as the darkest.
        assert_eq!(level(1, 1), 1);
        assert_eq!(level(2, 3), 1);
        assert_eq!(level(3, 3), 2);
        assert_eq!(level(FULL, 3), 4);
        assert_eq!(level(50, 40), 4);
    }

    fn counts(today: Day, commits: &[(Day, u32)]) -> Counts {
        Counts {
            today,
            first: first_day(today, 4),
            repos: vec!["a".into()],
            days: commits
                .iter()
                .map(|&(day, n)| {
                    (
                        day,
                        Tally {
                            by_repo: vec![n],
                            wrapped: 0,
                        },
                    )
                })
                .collect(),
        }
    }

    #[test]
    fn the_streak_runs_back_from_today_or_yesterday() {
        let run = [(OCT_7 - 3, 1), (OCT_7 - 2, 4), (OCT_7 - 1, 2)];
        assert_eq!(counts(OCT_7, &run).streak(), 3);
        let mut with_today = run.to_vec();
        with_today.push((OCT_7, 9));
        assert_eq!(counts(OCT_7, &with_today).streak(), 4);
        // A gap ends it.
        assert_eq!(counts(OCT_7, &[(OCT_7 - 2, 1), (OCT_7, 1)]).streak(), 1);
        assert_eq!(counts(OCT_7, &[(OCT_7 - 2, 1)]).streak(), 0);
        assert_eq!(counts(OCT_7, &[]).streak(), 0);
        // It never runs before the first day read.
        let all: Vec<(Day, u32)> = (OCT_7 - 60..=OCT_7).map(|d| (d, 1)).collect();
        let c = counts(OCT_7, &all);
        assert_eq!(c.streak() as i64, OCT_7 - c.first + 1);
    }

    #[test]
    fn columns_fill_the_width() {
        // 11-point cells 3 apart: 22 in 305 points, not in 304.
        assert_eq!(weeks_for(305.0, 11.0, 3.0), 22);
        assert_eq!(weeks_for(304.0, 11.0, 3.0), 21);
        assert_eq!(weeks_for(318.0, 11.0, 3.0), 22);
        assert_eq!(weeks_for(5.0, 11.0, 3.0), 1);
        assert_eq!(weeks_for(5000.0, 11.0, 3.0), MAX_WEEKS);
        // The weeks end with this one, from a Sunday.
        let first = first_day(OCT_7, 22);
        assert_eq!(weekday(first), 0);
        assert_eq!(OCT_7 - first, 21 * 7 + 3);
        assert_eq!(first_day(OCT_7, 1), OCT_7 - 3);
    }

    #[test]
    fn months_are_named_where_they_start_and_never_crowd() {
        let first = first_day(OCT_7, 22);
        // From Sunday 10 May 2026: June starts in column 4, July 8, August 12, September 17;
        // October in the last, too narrow for its name.
        let names = months(first, 22);
        assert_eq!(
            names,
            [(0, "May"), (4, "Jun"), (8, "Jul"), (12, "Aug"), (17, "Sep")]
        );
        // A month starting a column after the first leaves the first unnamed.
        let first = first_day(date_day(2026, 9, 27), 10);
        assert_eq!(date(first), (2026, 7, 26));
        assert_eq!(months(first, 10)[0], (1, "Aug"));
    }

    fn date_day(year: i64, month: u32, day: u32) -> Day {
        (0..30_000)
            .find(|&d| date(d) == (year, month, day))
            .unwrap()
    }

    #[test]
    fn a_day_names_its_busiest_repositories_first() {
        let c = Counts {
            today: OCT_7,
            first: OCT_7 - 10,
            repos: vec!["ranch".into(), "paddock".into(), "quiet".into()],
            days: BTreeMap::from([(
                OCT_7,
                Tally {
                    by_repo: vec![5, 24, 0],
                    wrapped: 6,
                },
            )]),
        };
        assert_eq!(c.repos_on(OCT_7), [("paddock", 24), ("ranch", 5)]);
        assert_eq!(c.commits(OCT_7), 29);
        assert_eq!(c.wrapped(OCT_7), 6);
        assert_eq!(c.total(OCT_7 - 10), 29);
        assert_eq!(c.busiest(OCT_7 - 10), 29);
        assert!(c.repos_on(OCT_7 - 1).is_empty());
        // Four are all named; five or more name three and put the rest together.
        let four = [("a", 9), ("b", 5), ("c", 2), ("d", 1)];
        assert_eq!(
            grouped(&four, 3),
            [
                (Some("a"), 9),
                (Some("b"), 5),
                (Some("c"), 2),
                (Some("d"), 1)
            ]
        );
        let six = [("a", 9), ("b", 5), ("c", 2), ("d", 1), ("e", 1), ("f", 1)];
        assert_eq!(
            grouped(&six, 3),
            [(Some("a"), 9), (Some("b"), 5), (Some("c"), 2), (None, 3)]
        );
    }
}
