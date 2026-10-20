//! The Kanban tab's Loop (DESIGN §13 P5-106): tasks the user puts in it from Queued, in the order
//! they set, handed one at a time to the repository's controller with `corral send`, the next once
//! the last is wrapped up. It is an outer loop only: the controller writes, dispatches, reviews,
//! waits on the user, merges and wraps up as its own rules say; the loop only says `开始做 <id>`.
//! Done is told by Git alone, a wrap-up commit on main (`kanban::wrapped_id`), never by what the
//! controller replies. What is in it, its order, whether it is on and what was handed over are
//! kept per repository on this machine, beside `kanban-repos.json`, never in Git or the task
//! files. Nothing here draws, reads Git or runs on its own: `kanban_view.rs` reads the boards and
//! calls [`settle`] and [`look`], and hands over what [`look`] says to with [`deliver`].
use crate::{
    agents::Status,
    corral::{Client, Refused},
    inbox,
    kanban::{self, Board, Card, Column, Seen},
    layout_state,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

/// How long, in seconds, the controller may idle after a task is handed to it, the task neither
/// wrapped up nor waiting on the user, before the loop calls it stuck.
pub const STUCK: f64 = 15.0 * 60.0;
/// How long, in seconds, the loop waits before handing a task over again after the controller was
/// busy or someone had just typed in its pane.
pub const RETRY: f64 = 30.0;

/// The loop's times, which tests make short.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Timing {
    pub stuck: f64,
    pub retry: f64,
}

impl Timing {
    pub const DEFAULT: Timing = Timing {
        stuck: STUCK,
        retry: RETRY,
    };
}

/// What the loop says to the controller to start a task: which, its title, its file, and that the
/// loop said it.
pub fn start_words(id: &str, title: &str, file: &str) -> String {
    format!(
        "开始做 {id}（{title}）：照 {}/{file} 和你的派活规矩做。{SIGNED}",
        kanban::TASKS
    )
}

/// What Nudge says to the controller about the task it was handed and has not wrapped up.
pub fn nudge_words(id: &str) -> String {
    format!("{id} 还没收尾：接着做，或者说明卡在哪。{SIGNED}")
}

/// How what the loop says ends, so that it is seen to be the loop's.
const SIGNED: &str = "— paddock loop";

/// One repository's loop, as kept.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Loop {
    /// The switch: on, it hands the next task over whenever it can.
    pub on: bool,
    /// The tasks in it not handed over yet, by id, in the order the user set.
    pub tasks: Vec<String>,
    /// The task handed over last, until it is wrapped up.
    pub sent: Option<Sent>,
    /// How many it handed over were wrapped up since it was turned on.
    pub done: u32,
    /// It turned itself off having nothing left: how many were done.
    pub finished: Option<u32>,
    /// A handing over corral did not take, waiting for Retry.
    pub failed: Option<Failed>,
}

/// A task handed to the controller.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sent {
    pub id: String,
    /// When, in seconds since the epoch.
    pub at: f64,
    /// When it was last nudged.
    #[serde(default)]
    pub nudged: Option<f64>,
}

/// What corral did not take, and why.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Failed {
    pub id: String,
    /// A nudge, not the task's start.
    #[serde(default)]
    pub nudge: bool,
    pub why: String,
}

impl Loop {
    /// Whether `id` is in it, handed over or not.
    pub fn has(&self, id: &str) -> bool {
        self.tasks.iter().any(|task| task == id) || self.sent.as_ref().is_some_and(|s| s.id == id)
    }

    /// `id` at its end; not a task in it already.
    pub fn add(&mut self, id: &str) -> bool {
        if id.is_empty() || self.has(id) {
            return false;
        }
        self.tasks.push(id.to_owned());
        self.finished = None;
        true
    }

    /// `id` out of it; the one handed over too, which it no longer waits for.
    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.clone();
        self.tasks.retain(|task| task != id);
        if self.sent.as_ref().is_some_and(|s| s.id == id) {
            self.sent = None;
        }
        if self.failed.as_ref().is_some_and(|f| f.id == id) {
            self.failed = None;
        }
        *self != before
    }

    /// `id` one place earlier (`up`) or later; not past either end.
    pub fn shift(&mut self, id: &str, up: bool) -> bool {
        let Some(at) = self.tasks.iter().position(|task| task == id) else {
            return false;
        };
        let to = if up {
            at.checked_sub(1)
        } else {
            Some(at + 1).filter(|to| *to < self.tasks.len())
        };
        let Some(to) = to else {
            return false;
        };
        self.tasks.swap(at, to);
        true
    }

    /// Whether it can be turned on: it has something to hand over or wait for.
    pub fn can_turn_on(&self) -> bool {
        !self.tasks.is_empty() || self.sent.is_some()
    }

    /// The switch; on only when [`Loop::can_turn_on`]. Turned on, the last run's finish is
    /// forgotten. Whether it changed.
    pub fn turn(&mut self, on: bool) -> bool {
        if on == self.on || (on && !self.can_turn_on()) {
            return false;
        }
        self.on = on;
        if on {
            self.finished = None;
        }
        true
    }

    /// Skip on a stuck task: no longer waited for, left where it is; why a nudge failed goes with
    /// it.
    pub fn skip(&mut self) -> bool {
        let Some(sent) = self.sent.take() else {
            return false;
        };
        if self.failed.as_ref().is_some_and(|f| f.id == sent.id) {
            self.failed = None;
        }
        true
    }
}

/// What settling a loop against the board found.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Settled {
    /// The loop changed, to be kept.
    pub changed: bool,
    /// It turned itself off with nothing left, so many done.
    pub finished: Option<u32>,
}

/// The non-draft card of `id`, in any column.
fn card<'a>(board: &'a Board, id: &str) -> Option<&'a Card> {
    board
        .columns
        .iter()
        .flatten()
        .find(|card| card.id == id && !card.draft)
}

/// The loop as Git now has it: the task handed over is done once wrapped up (dropped too) and
/// forgotten once its file is gone or a draft again; tasks wrapped up, dropped, made drafts or
/// gone leave it, those started otherwise stay; and on, with nothing left to hand over or wait for,
/// it turns itself off.
pub fn settle(lp: &mut Loop, board: &Board) -> Settled {
    let before = lp.clone();
    if let Some(sent) = &lp.sent {
        match card(board, &sent.id).map(|card| card.column) {
            Some(Column::Done) => {
                lp.done += 1;
                lp.sent = None;
            }
            None => lp.sent = None,
            Some(_) => {}
        }
    }
    lp.tasks
        .retain(|id| card(board, id).is_some_and(|card| card.column != Column::Done));
    // What failed for a task no longer in it, or a nudge for one no longer waited for.
    if let Some(failed) = &lp.failed {
        let still = if failed.nudge {
            lp.sent.as_ref().is_some_and(|s| s.id == failed.id)
        } else {
            lp.tasks.contains(&failed.id)
        };
        if !still {
            lp.failed = None;
        }
    }
    let mut finished = None;
    if lp.on && lp.sent.is_none() && lp.tasks.is_empty() {
        lp.on = false;
        lp.finished = Some(lp.done);
        finished = lp.finished;
        lp.done = 0;
        lp.failed = None;
    }
    Settled {
        changed: *lp != before,
        finished,
    }
}

/// Where the loop stands, as its status line says and what it is to do.
#[derive(Clone, Debug, PartialEq)]
pub enum Stand {
    /// Off: how many tasks are in it, and how its last run finished.
    Off { tasks: usize, finished: Option<u32> },
    /// No controller is running in the repository.
    NoController,
    /// More than one is.
    Controllers(usize),
    /// corral did not take a handing over: Retry.
    Failed {
        id: String,
        why: String,
        nudge: bool,
    },
    /// The task handed over waits on the user: its `待用户：` line.
    Asks { id: String, asks: String },
    /// The task handed over is not wrapped up, and the controller has idled this long.
    Stuck { id: String, idle: f64 },
    /// The task handed over is under way, or not begun yet.
    Working { id: String, begun: bool },
    /// Another task is under way in the repository.
    Busy { id: String },
    /// The controller turned the last try away; tried again at `at`.
    Retrying { id: String, at: f64 },
    /// Being handed over now.
    Sending { id: String },
    /// To hand over now: the task, its file, and to which controller.
    Send {
        id: String,
        title: String,
        file: String,
        to: String,
    },
    /// Nothing in it can start yet.
    Blocked,
}

impl Stand {
    /// The loop waits on the user: its group says Needs you, and the Inbox has a row.
    pub fn needs_you(&self) -> bool {
        matches!(
            self,
            Stand::NoController
                | Stand::Controllers(_)
                | Stand::Failed { .. }
                | Stand::Asks { .. }
                | Stand::Stuck { .. }
        )
    }

    /// The status line, `next` being how many wait after the task under way.
    pub fn words(&self, next: usize, timing: &Timing) -> String {
        let next = if next > 0 {
            format!(" · {next} next")
        } else {
            String::new()
        };
        match self {
            Stand::Off {
                finished: Some(done),
                ..
            } => format!("Loop finished · {done} done"),
            Stand::Off { tasks: 0, .. } => "Off · add queued tasks to the loop".into(),
            Stand::Off { tasks, .. } => format!("Off · {tasks} in the loop"),
            Stand::NoController => "Waiting: no controller".into(),
            Stand::Controllers(_) => "Waiting: more than one controller".into(),
            Stand::Failed {
                why, nudge: false, ..
            } => format!("Send failed: {why}"),
            Stand::Failed {
                why, nudge: true, ..
            } => format!("Nudge failed: {why}"),
            Stand::Asks { id, asks } => format!("Needs you: {id} \u{2014} {asks}"),
            Stand::Stuck { id, .. } => format!(
                "Stuck: {id} not wrapped up, controller idle {}m",
                (timing.stuck / 60.0).round()
            ),
            Stand::Working { id, begun: true } => format!("On · {id} in progress{next}"),
            Stand::Working { id, begun: false } => {
                format!("On · {id} sent to the controller{next}")
            }
            Stand::Busy { id } => format!("Waiting: {id} in progress"),
            Stand::Retrying { id, .. } => {
                format!("Waiting: controller busy, sending {id} again shortly")
            }
            Stand::Sending { id } | Stand::Send { id, .. } => format!("Sending {id}\u{2026}"),
            Stand::Blocked => "Waiting: nothing in the loop can start yet".into(),
        }
    }
}

/// Whether `card` can be started: queued and on main, nothing it waits for, not waiting on the
/// user.
pub fn startable(card: &Card) -> bool {
    card.column == Column::Queued && !card.draft && card.waits.is_none() && card.asks.is_none()
}

/// Where the loop stands, settled against `board`, at `now`, with the repository's agents; `hold`
/// when a handing over was turned away and is tried again then, `sending` while one is on its way.
pub fn look(
    lp: &Loop,
    board: &Board,
    agents: &[Seen],
    now: f64,
    timing: &Timing,
    hold: Option<f64>,
    sending: bool,
) -> Stand {
    if !lp.on {
        return Stand::Off {
            tasks: lp.tasks.len() + usize::from(lp.sent.is_some()),
            finished: lp.finished,
        };
    }
    let controllers = kanban::controllers_for(&board.repo, agents);
    let failed = lp.failed.as_ref().map(|failed| Stand::Failed {
        id: failed.id.clone(),
        why: failed.why.clone(),
        nudge: failed.nudge,
    });
    let alone = match controllers.as_slice() {
        [] => Err(Stand::NoController),
        [one] => Ok(*one),
        many => Err(Stand::Controllers(many.len())),
    };
    if let Some(sent) = &lp.sent {
        let card = card(board, &sent.id);
        if let Some(asks) = card.and_then(|card| card.asks.clone()) {
            return Stand::Asks {
                id: sent.id.clone(),
                asks,
            };
        }
        let controller = match alone {
            Ok(controller) => controller,
            Err(stand) => return stand,
        };
        if let Some(failed) = failed {
            return failed;
        }
        // Only the time it idles counts: from the latest of the handing over, the nudge and its
        // idling began.
        if controller.status == Status::Idle {
            let from = [Some(sent.at), sent.nudged, controller.since]
                .into_iter()
                .flatten()
                .fold(sent.at, f64::max);
            if now - from >= timing.stuck {
                return Stand::Stuck {
                    id: sent.id.clone(),
                    idle: now - from,
                };
            }
        }
        return Stand::Working {
            id: sent.id.clone(),
            begun: card.is_some_and(|card| card.column != Column::Queued),
        };
    }
    if let Some(failed) = failed {
        return failed;
    }
    let controller = match alone {
        Ok(controller) => controller,
        Err(stand) => return stand,
    };
    // Anything under way, the loop's or not: the controller starts no new task before the last is
    // wrapped up.
    let under_way = [Column::InProgress, Column::ToReview, Column::Merged]
        .into_iter()
        .find_map(|column| board.cards(column).first());
    if let Some(card) = under_way {
        return Stand::Busy {
            id: card.id.clone(),
        };
    }
    let Some(next) = lp
        .tasks
        .iter()
        .filter_map(|id| card(board, id))
        .find(|card| startable(card))
    else {
        return Stand::Blocked;
    };
    let id = next.id.clone();
    if sending {
        return Stand::Sending { id };
    }
    if let Some(at) = hold.filter(|at| now < *at) {
        return Stand::Retrying { id, at };
    }
    Stand::Send {
        id,
        title: next.title.clone(),
        file: next.file.clone(),
        to: controller.name.clone(),
    }
}

/// How a handing over went.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Delivery {
    /// corral took it; text already in the controller's box went in with it.
    Taken { merged: bool },
    /// The controller was busy, or someone had just typed in its pane: try again later.
    Later(String),
    /// Anything else: Retry.
    Failed(String),
}

/// Hands `text` to the controller `to` with `corral send`, never forced: corral's own guard turns
/// it away while the controller works or someone has just typed in its pane, which is only later.
pub fn deliver(client: &Client, to: &str, text: &str, cancel: &AtomicBool) -> Delivery {
    match client.deliver(to, text, cancel) {
        Ok(taken) => Delivery::Taken {
            merged: taken.merged_with_draft,
        },
        Err(refused) if later(&refused) => Delivery::Later(inbox::refusal(&refused)),
        Err(refused) => Delivery::Failed(inbox::refusal(&refused)),
    }
}

/// corral turned it away for now: the controller works, or someone has just typed in its pane.
fn later(refused: &Refused) -> bool {
    matches!(refused.code.as_deref(), Some("not_idle" | "human_active"))
}

/// Where the loops are kept, beside the layout: `$XDG_STATE_HOME/paddock/kanban-loops.json`.
pub fn kept_path() -> Option<PathBuf> {
    layout_state::default_path().map(|path| path.with_file_name("kanban-loops.json"))
}

/// Every repository's loop, by its main worktree, as kept at `path`.
pub struct Loops {
    path: Option<PathBuf>,
    repos: BTreeMap<PathBuf, Loop>,
    /// The last save that did not work.
    problem: Option<String>,
}

impl Loops {
    /// As kept at `path`; `None` keeps them only in memory. A file that cannot be read gives none.
    pub fn load(path: Option<PathBuf>) -> Self {
        let repos = path
            .as_ref()
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Loops {
            path,
            repos,
            problem: None,
        }
    }

    /// `repo`'s loop: an empty one, off, when it has none.
    pub fn get(&self, repo: &Path) -> Loop {
        self.repos.get(repo).cloned().unwrap_or_default()
    }

    /// `repo`'s loop made `lp`, and kept when it changed; an empty one, off, is left out. Whether
    /// it changed.
    pub fn put(&mut self, repo: &Path, lp: Loop) -> bool {
        if self.get(repo) == lp {
            return false;
        }
        if lp == Loop::default() {
            self.repos.remove(repo);
        } else {
            self.repos.insert(repo.to_path_buf(), lp);
        }
        self.save();
        true
    }

    /// The repositories whose loop is on.
    pub fn on(&self) -> Vec<PathBuf> {
        self.repos
            .iter()
            .filter(|(_, lp)| lp.on)
            .map(|(repo, _)| repo.clone())
            .collect()
    }

    /// Why the loops were not saved, when the last save failed.
    pub fn problem(&self) -> Option<&str> {
        self.problem.as_deref()
    }

    /// Written whole to a file beside it, then put in its place.
    fn save(&mut self) {
        let Some(path) = &self.path else {
            return;
        };
        let write = || -> std::io::Result<()> {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            let bytes = serde_json::to_vec_pretty(&self.repos).map_err(std::io::Error::other)?;
            let temporary = path.with_extension("json.saving");
            std::fs::write(&temporary, bytes)?;
            std::fs::rename(&temporary, path)
        };
        self.problem = write()
            .err()
            .map(|error| format!("Couldn't save the loop: {error}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kanban::{Doer, Facts, Priority, Task, board};

    const REPO: &str = "/w/paddock";
    const T: Timing = Timing {
        stuck: 900.0,
        retry: 30.0,
    };

    fn task(id: &str) -> Task {
        Task {
            id: id.into(),
            title: format!("活 {id}"),
            file: format!("{id}-x.md"),
            worktree: Some(format!("/w/wt/{id}")),
            branch: Some(id.to_lowercase()),
            depends: None,
            asks: None,
            doer: Doer::Decides,
            priority: Priority::Medium,
            problems: Vec::new(),
        }
    }

    fn facts(tasks: Vec<Task>) -> Facts {
        Facts {
            repo: PathBuf::from(REPO),
            name: "paddock".into(),
            tasks,
            ..Default::default()
        }
    }

    fn seen(name: &str, cwd: &str, status: Status, controller: bool) -> Seen {
        Seen {
            name: name.into(),
            kind: Some("claude".into()),
            cwd: Some(cwd.into()),
            task: None,
            status,
            said_done: false,
            since: Some(0.0),
            controller,
        }
    }

    fn main(status: Status, since: f64) -> Seen {
        Seen {
            since: Some(since),
            ..seen("paddock/main", REPO, status, true)
        }
    }

    fn on(tasks: &[&str]) -> Loop {
        Loop {
            on: true,
            tasks: tasks.iter().map(|t| t.to_string()).collect(),
            ..Default::default()
        }
    }

    fn sent(id: &str, at: f64) -> Option<Sent> {
        Some(Sent {
            id: id.into(),
            at,
            nudged: None,
        })
    }

    fn at(lp: &Loop, facts: &Facts, agents: &[Seen], now: f64) -> Stand {
        look(lp, &board(facts, agents, now), agents, now, &T, None, false)
    }

    fn send(id: &str) -> Stand {
        Stand::Send {
            id: id.into(),
            title: format!("活 {id}"),
            file: format!("{id}-x.md"),
            to: "paddock/main".into(),
        }
    }

    #[test]
    fn the_words_say_which_task_which_file_and_that_the_loop_sent_them() {
        assert_eq!(
            start_words("P5-12", "看板 Loop", "P5-12-看板Loop.md"),
            "开始做 P5-12（看板 Loop）：照 docs/任务/P5-12-看板Loop.md 和你的派活规矩做。— paddock loop"
        );
        let nudge = nudge_words("P5-12");
        assert!(nudge.starts_with("P5-12 "), "{nudge}");
        assert!(nudge.ends_with("— paddock loop"), "{nudge}");
    }

    #[test]
    fn a_task_goes_only_when_every_condition_holds() {
        let facts = facts(vec![task("P1-1"), task("P1-2")]);
        let idle = main(Status::Idle, 0.0);
        let lp = on(&["P1-1"]);
        // All there: on, one controller, nothing under way, one that can start.
        assert_eq!(
            at(&lp, &facts, std::slice::from_ref(&idle), 100.0),
            send("P1-1")
        );
        // Off.
        let off = Loop {
            on: false,
            ..lp.clone()
        };
        assert_eq!(
            at(&off, &facts, std::slice::from_ref(&idle), 100.0),
            Stand::Off {
                tasks: 1,
                finished: None
            }
        );
        // No controller: none, one exited, one elsewhere, one not labelled.
        for agents in [
            vec![],
            vec![main(Status::Exited, 0.0)],
            vec![seen("ranch/main", "/w/ranch", Status::Idle, true)],
            vec![seen("paddock/main", REPO, Status::Idle, false)],
        ] {
            assert_eq!(at(&lp, &facts, &agents, 100.0), Stand::NoController);
        }
        // Two.
        let other = seen("paddock/main-1", "/w/paddock/app", Status::Idle, true);
        assert_eq!(
            at(&lp, &facts, &[idle.clone(), other], 100.0),
            Stand::Controllers(2)
        );
        // Something else under way, in each column it can be in, the loop's or not.
        let mut busy = facts.clone();
        busy.worktrees
            .push(("/w/wt/P1-2".into(), Some("p1-2".into())));
        assert_eq!(
            at(&lp, &busy, std::slice::from_ref(&idle), 100.0),
            Stand::Busy { id: "P1-2".into() }
        );
        let mut review = facts.clone();
        review.records.insert("P1-2".into());
        assert_eq!(
            at(&lp, &review, std::slice::from_ref(&idle), 100.0),
            Stand::Busy { id: "P1-2".into() }
        );
        let mut merged = facts.clone();
        merged.merges.insert("P1-2".into(), 50);
        assert_eq!(
            at(&lp, &merged, std::slice::from_ref(&idle), 100.0),
            Stand::Busy { id: "P1-2".into() }
        );
        // One handed over already, not wrapped up.
        let handed = Loop {
            sent: sent("P1-2", 90.0),
            ..lp.clone()
        };
        assert_eq!(
            at(&handed, &facts, std::slice::from_ref(&idle), 100.0),
            Stand::Working {
                id: "P1-2".into(),
                begun: false
            }
        );
        // Nothing in it that can start.
        assert_eq!(
            at(&on(&[]), &facts, std::slice::from_ref(&idle), 100.0),
            Stand::Blocked
        );
        // On its way, or held back after the controller turned it away.
        let board = board(&facts, std::slice::from_ref(&idle), 100.0);
        let agents = [idle];
        assert_eq!(
            look(&lp, &board, &agents, 100.0, &T, None, true),
            Stand::Sending { id: "P1-1".into() }
        );
        assert_eq!(
            look(&lp, &board, &agents, 100.0, &T, Some(120.0), false),
            Stand::Retrying {
                id: "P1-1".into(),
                at: 120.0
            }
        );
        assert_eq!(
            look(&lp, &board, &agents, 120.0, &T, Some(120.0), false),
            send("P1-1")
        );
    }

    #[test]
    fn tasks_that_cannot_start_are_passed_over_and_stay_in_the_loop() {
        let waits = Task {
            depends: Some("P1-9".into()),
            ..task("P1-1")
        };
        let asks = Task {
            asks: Some("看样稿".into()),
            ..task("P1-2")
        };
        let mut facts = facts(vec![waits, asks, task("P1-3"), task("P1-9")]);
        // P1-9 is wrapped up elsewhere for one case below.
        let idle = main(Status::Idle, 0.0);
        let mut lp = on(&["P1-1", "P1-2", "P1-3"]);
        let b = board(&facts, std::slice::from_ref(&idle), 100.0);
        assert!(!settle(&mut lp, &b).changed);
        assert_eq!(lp.tasks, ["P1-1", "P1-2", "P1-3"]);
        assert_eq!(
            at(&lp, &facts, std::slice::from_ref(&idle), 100.0),
            send("P1-3")
        );
        // Once what it waits for is wrapped up, the first goes first.
        facts.wrapped.insert("P1-9".into(), 99);
        assert_eq!(
            at(&lp, &facts, std::slice::from_ref(&idle), 100.0),
            send("P1-1")
        );
        // None can start.
        let lp = on(&["P1-2"]);
        assert_eq!(at(&lp, &facts, &[idle], 100.0), Stand::Blocked);
        // A draft never can.
        let mut drafted = facts.clone();
        drafted
            .drafts
            .push(kanban::draft("P1-7-x.md", "# 任务：x\n"));
        let card = board(&drafted, &[], 0.0)
            .cards(Column::Queued)
            .iter()
            .find(|c| c.id == "P1-7")
            .cloned()
            .unwrap();
        assert!(!startable(&card));
    }

    #[test]
    fn the_next_goes_once_the_last_is_wrapped_up() {
        let mut facts = facts(vec![task("P1-1"), task("P1-2"), task("P1-3")]);
        let idle = main(Status::Idle, 0.0);
        let mut lp = Loop {
            sent: sent("P1-1", 10.0),
            ..on(&["P1-2", "P1-3"])
        };
        // Under way: nothing else goes.
        facts
            .worktrees
            .push(("/w/wt/P1-1".into(), Some("p1-1".into())));
        let b = board(&facts, std::slice::from_ref(&idle), 100.0);
        assert!(!settle(&mut lp, &b).changed);
        assert_eq!(
            at(&lp, &facts, std::slice::from_ref(&idle), 100.0),
            Stand::Working {
                id: "P1-1".into(),
                begun: true
            }
        );
        // Merged, not wrapped up: still waits.
        facts.merges.insert("P1-1".into(), 50);
        let b = board(&facts, std::slice::from_ref(&idle), 100.0);
        settle(&mut lp, &b);
        assert_eq!(lp.sent.as_ref().map(|s| s.id.as_str()), Some("P1-1"));
        // Wrapped up: done, and the next goes.
        facts.worktrees.clear();
        facts.wrapped.insert("P1-1".into(), 60);
        let b = board(&facts, std::slice::from_ref(&idle), 100.0);
        let settled = settle(&mut lp, &b);
        assert!(settled.changed);
        assert_eq!(settled.finished, None);
        assert_eq!((lp.sent.clone(), lp.done), (None, 1));
        assert_eq!(
            at(&lp, &facts, std::slice::from_ref(&idle), 100.0),
            send("P1-2")
        );
        // Dropped counts as wrapped up.
        lp.tasks.retain(|t| t != "P1-2");
        lp.sent = sent("P1-2", 70.0);
        facts.wrapped.insert("P1-2".into(), 80);
        facts.dropped.insert("P1-2".into(), "重复".into());
        settle(&mut lp, &board(&facts, &[idle], 100.0));
        assert_eq!((lp.sent.clone(), lp.done), (None, 2));
        // The controller saying DONE is not wrapped up.
        let mut said = facts.clone();
        said.wrapped.remove("P1-2");
        said.dropped.clear();
        let mut lp = Loop {
            sent: sent("P1-2", 70.0),
            ..on(&["P1-3"])
        };
        let done = Seen {
            said_done: true,
            ..main(Status::Idle, 0.0)
        };
        settle(&mut lp, &board(&said, &[done], 100.0));
        assert_eq!(lp.sent.as_ref().map(|s| s.id.as_str()), Some("P1-2"));
    }

    #[test]
    fn the_task_handed_over_waiting_on_the_user_holds_the_loop() {
        let asks = Task {
            asks: Some("实测".into()),
            ..task("P1-1")
        };
        let mut facts = facts(vec![asks, task("P1-2")]);
        facts
            .worktrees
            .push(("/w/wt/P1-1".into(), Some("p1-1".into())));
        let lp = Loop {
            sent: sent("P1-1", 10.0),
            ..on(&["P1-2"])
        };
        // Long idle, but waiting on the user is not stuck.
        let stand = at(&lp, &facts, &[main(Status::Idle, 0.0)], 10_000.0);
        assert_eq!(
            stand,
            Stand::Asks {
                id: "P1-1".into(),
                asks: "实测".into()
            }
        );
        assert!(stand.needs_you());
    }

    #[test]
    fn stuck_counts_only_the_time_the_controller_idles() {
        let facts = facts(vec![task("P1-1"), task("P1-2")]);
        let lp = Loop {
            sent: sent("P1-1", 1_000.0),
            ..on(&["P1-2"])
        };
        // Idle since before it was handed over: counted from the handing over.
        let idle = main(Status::Idle, 900.0);
        assert_eq!(
            at(&lp, &facts, std::slice::from_ref(&idle), 1_899.0),
            Stand::Working {
                id: "P1-1".into(),
                begun: false
            }
        );
        let stuck = at(&lp, &facts, std::slice::from_ref(&idle), 1_900.0);
        assert_eq!(
            stuck,
            Stand::Stuck {
                id: "P1-1".into(),
                idle: 900.0
            }
        );
        assert!(stuck.needs_you());
        // Working is never stuck, however long.
        let working = main(Status::Working, 1_000.0);
        assert!(matches!(
            at(&lp, &facts, &[working], 99_000.0),
            Stand::Working { .. }
        ));
        // Idle again after working: counted from then.
        let again = main(Status::Idle, 1_500.0);
        assert!(matches!(
            at(&lp, &facts, std::slice::from_ref(&again), 2_399.0),
            Stand::Working { .. }
        ));
        assert!(matches!(
            at(&lp, &facts, std::slice::from_ref(&again), 2_400.0),
            Stand::Stuck { .. }
        ));
        // Nudged: counted from the nudge.
        let nudged = Loop {
            sent: Some(Sent {
                id: "P1-1".into(),
                at: 1_000.0,
                nudged: Some(3_000.0),
            }),
            ..lp.clone()
        };
        assert!(matches!(
            at(&nudged, &facts, &[again], 3_899.0),
            Stand::Working { .. }
        ));
        assert_eq!(
            Stand::Stuck {
                id: "P1-1".into(),
                idle: 1234.0
            }
            .words(1, &T),
            "Stuck: P1-1 not wrapped up, controller idle 15m"
        );
    }

    #[test]
    fn controllers_and_failures_hold_it_and_need_the_user() {
        let facts = facts(vec![task("P1-1")]);
        let lp = on(&["P1-1"]);
        assert!(Stand::NoController.needs_you());
        assert!(Stand::Controllers(2).needs_you());
        assert_eq!(Stand::NoController.words(0, &T), "Waiting: no controller");
        assert_eq!(
            Stand::Controllers(3).words(0, &T),
            "Waiting: more than one controller"
        );
        // Handed over and the controller went: waits for it, nothing else goes.
        let handed = Loop {
            sent: sent("P1-1", 10.0),
            ..on(&[])
        };
        assert_eq!(at(&handed, &facts, &[], 100.0), Stand::NoController);
        // corral did not take it: Retry, never again on its own.
        let failed = Loop {
            failed: Some(Failed {
                id: "P1-1".into(),
                nudge: false,
                why: "It's paused".into(),
            }),
            ..lp.clone()
        };
        let stand = at(&failed, &facts, &[main(Status::Idle, 0.0)], 100.0);
        assert_eq!(
            stand,
            Stand::Failed {
                id: "P1-1".into(),
                why: "It's paused".into(),
                nudge: false
            }
        );
        assert!(stand.needs_you());
        assert_eq!(stand.words(0, &T), "Send failed: It's paused");
        // Waiting on a busy controller, or for something under way, does not.
        for stand in [
            Stand::Retrying {
                id: "P1-1".into(),
                at: 1.0,
            },
            Stand::Busy { id: "P1-2".into() },
            Stand::Working {
                id: "P1-1".into(),
                begun: true,
            },
            Stand::Blocked,
            Stand::Off {
                tasks: 0,
                finished: Some(2),
            },
        ] {
            assert!(!stand.needs_you(), "{stand:?}");
        }
    }

    #[test]
    fn an_empty_loop_turns_itself_off() {
        let mut facts = facts(vec![task("P1-1"), task("P1-2")]);
        facts.wrapped.insert("P1-1".into(), 60);
        let b = board(&facts, &[], 100.0);
        // The last one handed over is wrapped up.
        let mut lp = Loop {
            sent: sent("P1-1", 10.0),
            done: 2,
            ..on(&[])
        };
        let settled = settle(&mut lp, &b);
        assert_eq!(settled.finished, Some(3));
        assert!(!lp.on);
        assert_eq!((lp.done, lp.finished, lp.sent.clone()), (0, Some(3), None));
        assert_eq!(
            Stand::Off {
                tasks: 0,
                finished: Some(3)
            }
            .words(0, &T),
            "Loop finished · 3 done"
        );
        // Its last task done elsewhere.
        let mut lp = on(&["P1-1"]);
        assert_eq!(settle(&mut lp, &b).finished, Some(0));
        // Off and empty: nothing to finish.
        let mut lp = Loop::default();
        assert_eq!(settle(&mut lp, &b), Settled::default());
        // Off, the one handed over wrapped up: counted, still off, no finish.
        let mut lp = Loop {
            sent: sent("P1-1", 10.0),
            ..Default::default()
        };
        let settled = settle(&mut lp, &b);
        assert_eq!((settled.finished, lp.done, lp.on), (None, 1, false));
    }

    #[test]
    fn what_leaves_queued_otherwise_than_by_starting_leaves_the_loop() {
        // P1-1 done, P1-2 dropped, P1-3 a draft again, P1-4 gone, P1-5 started otherwise, P1-6
        // queued.
        let mut facts = facts(vec![task("P1-1"), task("P1-2"), task("P1-5"), task("P1-6")]);
        facts.wrapped.insert("P1-1".into(), 1);
        facts.wrapped.insert("P1-2".into(), 2);
        facts.dropped.insert("P1-2".into(), "不做".into());
        facts.drafts.push(kanban::draft("P1-3-x.md", "# 任务：x\n"));
        facts
            .worktrees
            .push(("/w/wt/P1-5".into(), Some("p1-5".into())));
        let mut lp = Loop {
            on: false,
            ..on(&["P1-1", "P1-2", "P1-3", "P1-4", "P1-5", "P1-6"])
        };
        let settled = settle(&mut lp, &board(&facts, &[], 0.0));
        assert!(settled.changed);
        assert_eq!(lp.tasks, ["P1-5", "P1-6"]);
        // The one handed over, its file gone or a draft again: no longer waited for, not done.
        for id in ["P1-3", "P1-4"] {
            let mut lp = Loop {
                sent: sent(id, 1.0),
                ..on(&["P1-6"])
            };
            settle(&mut lp, &board(&facts, &[], 0.0));
            assert_eq!((lp.sent.clone(), lp.done, lp.on), (None, 0, true), "{id}");
        }
    }

    #[test]
    fn tasks_are_added_taken_out_and_moved() {
        let mut lp = Loop::default();
        assert!(!lp.can_turn_on());
        assert!(!lp.turn(true));
        assert!(!lp.on);
        assert!(lp.add("P1-1") && lp.add("P1-2") && lp.add("P1-3"));
        assert!(!lp.add("P1-2"));
        assert_eq!(lp.tasks, ["P1-1", "P1-2", "P1-3"]);
        assert!(lp.shift("P1-3", true));
        assert_eq!(lp.tasks, ["P1-1", "P1-3", "P1-2"]);
        assert!(lp.shift("P1-1", false));
        assert_eq!(lp.tasks, ["P1-3", "P1-1", "P1-2"]);
        // Not past either end.
        assert!(!lp.shift("P1-3", true));
        assert!(!lp.shift("P1-2", false));
        assert!(!lp.shift("P9-9", true));
        assert!(lp.remove("P1-1"));
        assert!(!lp.remove("P1-1"));
        assert_eq!(lp.tasks, ["P1-3", "P1-2"]);
        // Turned on, the last run's finish forgotten.
        lp.finished = Some(4);
        assert!(lp.turn(true));
        assert!(lp.on && lp.finished.is_none());
        // The one handed over is in it, and can be taken out.
        lp.sent = sent("P1-9", 1.0);
        assert!(lp.has("P1-9") && lp.has("P1-2") && !lp.has("P1-1"));
        assert!(!lp.add("P1-9"));
        assert!(lp.remove("P1-9"));
        assert_eq!(lp.sent, None);
        // Skip: the one handed over is let go, and why it failed with it.
        lp.sent = sent("P1-9", 1.0);
        lp.failed = Some(Failed {
            id: "P1-9".into(),
            nudge: true,
            why: "x".into(),
        });
        assert!(lp.skip());
        assert_eq!((lp.sent.clone(), lp.failed.clone()), (None, None));
        assert!(lp.turn(false));
        assert!(!lp.on);
        // Off with only the one handed over: it can be turned on, to wait for it.
        let lp = Loop {
            sent: sent("P1-9", 1.0),
            ..Default::default()
        };
        assert!(lp.can_turn_on());
    }

    #[test]
    fn loops_are_kept_per_repository_and_read_back() {
        let dir = std::env::temp_dir().join(format!("paddock-loops-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("state/kanban-loops.json");
        let (a, b) = (Path::new("/w/a"), Path::new("/w/b"));
        let mut loops = Loops::load(Some(path.clone()));
        let first = Loop {
            sent: sent("P1-1", 5.0),
            ..on(&["P1-2", "P1-3"])
        };
        let second = Loop {
            finished: Some(2),
            failed: Some(Failed {
                id: "P2-1".into(),
                nudge: false,
                why: "x".into(),
            }),
            ..Loop::default()
        };
        assert!(loops.put(a, first.clone()));
        assert!(loops.put(b, second.clone()));
        assert!(!loops.put(b, second.clone()));
        assert_eq!(loops.on(), [a.to_path_buf()]);
        // Read back, as paddock opened again: as they were.
        let again = Loops::load(Some(path.clone()));
        assert_eq!((again.get(a), again.get(b)), (first, second));
        assert_eq!(again.get(Path::new("/w/c")), Loop::default());
        // An empty one, off, is left out.
        assert!(loops.put(b, Loop::default()));
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("/w/a") && !text.contains("/w/b"), "{text}");
        // A file that cannot be read gives none.
        std::fs::write(&path, "not json").unwrap();
        assert_eq!(Loops::load(Some(path)).get(a), Loop::default());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A fake `corral` in a directory of its own that answers `answer` and keeps its arguments;
    /// never the real one.
    fn fake(name: &str, answer: &str, code: i32) -> (PathBuf, Client) {
        use std::os::unix::fs::PermissionsExt;
        let dir =
            std::env::temp_dir().join(format!("paddock-loop-test-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("corral");
        let script = format!(
            "#!/bin/sh\nfor a in \"$@\"; do printf '%s\\n' \"$a\"; done >> \"$(dirname \"$0\")/calls\"\n\
             echo '{answer}'\nexit {code}\n"
        );
        std::fs::write(&path, script).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        (dir, Client::local(path.display().to_string()))
    }

    #[test]
    fn a_handing_over_is_taken_held_back_or_failed() {
        let no = AtomicBool::new(false);
        let words = start_words("P1-1", "活", "P1-1-x.md");
        let (dir, corral) = fake("taken", r#"{"ok":true,"merged_with_draft":false}"#, 0);
        assert_eq!(
            deliver(&corral, "paddock/main", &words, &no),
            Delivery::Taken { merged: false }
        );
        // Sent as it is, to the controller by name, never forced.
        let calls = std::fs::read_to_string(dir.join("calls")).unwrap();
        assert_eq!(calls, format!("send\npaddock/main\n{words}\n"));
        std::fs::remove_dir_all(&dir).unwrap();
        let (dir, corral) = fake("merged", r#"{"ok":true,"merged_with_draft":true}"#, 0);
        assert_eq!(
            deliver(&corral, "paddock/main", &words, &no),
            Delivery::Taken { merged: true }
        );
        std::fs::remove_dir_all(&dir).unwrap();
        // Busy, or someone typing in its pane: later, not an error.
        for code in ["not_idle", "human_active"] {
            let answer = format!(r#"{{"ok":false,"error":"{code}","message":"no"}}"#);
            let (dir, corral) = fake(code, &answer, 1);
            assert!(
                matches!(deliver(&corral, "p/main", "x", &no), Delivery::Later(_)),
                "{code}"
            );
            std::fs::remove_dir_all(&dir).unwrap();
        }
        // Paused, not delivered, anything else: failed, with why.
        for code in ["paused", "not_delivered", "something_else"] {
            let answer = format!(r#"{{"ok":false,"error":"{code}","message":"said {code}"}}"#);
            let (dir, corral) = fake(code, &answer, 1);
            assert_eq!(
                deliver(&corral, "p/main", "x", &no),
                Delivery::Failed(format!("said {code}")),
                "{code}"
            );
            std::fs::remove_dir_all(&dir).unwrap();
        }
        let (dir, corral) = fake("garbage", "not json", 0);
        assert_eq!(
            deliver(&corral, "p/main", "x", &no),
            Delivery::Failed("corral did not say how it went".into())
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
