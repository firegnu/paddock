//! Text selected in a terminal, sent to an agent (DESIGN §13 P5-148c): what goes, to whom it can
//! go and in what order, and the panel's state while it is chosen and sent. Apart from GPUI and
//! from corral, so that all of it can be tested; the window draws the panel and runs
//! `corral send`.
use crate::{agents::Status, corral::Refused};

/// What goes: where it came from, the selection in a code block, and the note after a blank line
/// when there is one.
pub fn message(source: &str, text: &str, note: &str) -> String {
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    let end = lines
        .iter()
        .rposition(|line| !line.is_empty())
        .map_or(0, |last| last + 1);
    let body = lines[..end].join("\n");
    // Longer than any run of backticks in it, so that none closes it early.
    let longest = body.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    let fence = "`".repeat(longest.max(2) + 1);
    let mut message = format!("From {source}:\n{fence}\n{body}\n{fence}");
    let note = note.trim();
    if !note.is_empty() {
        message.push_str("\n\n");
        message.push_str(note);
    }
    message
}

/// Where the selection came from, as its first line names it: a shell's directory with home
/// written `~` (`shell · ~/projects/paddock`); an agent with its kind (`dev-terminal · claude`);
/// anything else as the pane names it.
pub fn source(pane: &str, kind: Option<&str>, home: Option<&str>) -> String {
    if let Some(cwd) = pane.strip_prefix("shell · ") {
        return format!("shell · {}", crate::card::tilde(cwd, home));
    }
    match kind {
        Some(kind) => format!("{pane} · {kind}"),
        None => pane.to_owned(),
    }
}

/// An agent the selection can go to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    pub name: String,
    pub kind: Option<String>,
    /// The agent the pane shows.
    pub own: bool,
}

/// An agent as the sidebar lists it, in its order.
pub struct Listed<'a> {
    pub name: &'a str,
    pub kind: Option<&'a str>,
    pub status: Status,
}

/// Whom the selection can go to: the pane's own agent first, then the others in the sidebar's
/// order; not those that have ended.
pub fn targets(listed: &[Listed], own: Option<&str>) -> Vec<Target> {
    let mut targets: Vec<Target> = listed
        .iter()
        .filter(|agent| agent.status != Status::Exited)
        .map(|agent| Target {
            name: agent.name.to_owned(),
            kind: agent.kind.map(str::to_owned),
            own: Some(agent.name) == own,
        })
        .collect();
    if let Some(at) = targets.iter().position(|target| target.own) {
        let target = targets.remove(at);
        targets.insert(0, target);
    }
    targets
}

/// The panel while open: the selection, its source, the agent chosen, the note typed, and how
/// the last try went.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Panel {
    pub text: String,
    pub source: String,
    /// The agent chosen; none before the first move, when it is the first listed.
    chosen: Option<String>,
    pub note: String,
    /// To whom it is on its way.
    pub sending: Option<String>,
    /// Why corral did not take the last try.
    pub refused: Option<String>,
}

impl Panel {
    pub fn new(text: String, source: String) -> Self {
        Self {
            text,
            source,
            ..Self::default()
        }
    }

    /// What would go now.
    pub fn message(&self) -> String {
        message(&self.source, &self.text, &self.note)
    }

    /// The row chosen in `targets`.
    pub fn chosen(&self, targets: &[Target]) -> Option<usize> {
        if targets.is_empty() {
            return None;
        }
        let named = self
            .chosen
            .as_ref()
            .and_then(|name| targets.iter().position(|target| &target.name == name));
        Some(named.unwrap_or(0))
    }

    /// Chooses the row `step` away, staying within the list.
    pub fn step(&mut self, step: isize, targets: &[Target]) {
        let Some(at) = self.chosen(targets) else {
            return;
        };
        let to = (at as isize + step).clamp(0, targets.len() as isize - 1) as usize;
        self.chosen = Some(targets[to].name.clone());
    }

    /// Chooses `name`.
    pub fn choose(&mut self, name: &str) {
        self.chosen = Some(name.to_owned());
    }

    /// Sending starts: to whom and what, unless it is already on its way or there is no one.
    pub fn start(&mut self, targets: &[Target]) -> Option<(String, String)> {
        if self.sending.is_some() {
            return None;
        }
        let to = targets[self.chosen(targets)?].name.clone();
        self.sending = Some(to.clone());
        self.refused = None;
        Some((to, self.message()))
    }

    /// corral answered: taken, the panel is done (true); refused, it stays with why, the text and
    /// the note as they were, in the Inbox's words.
    pub fn finish(&mut self, result: Result<(), Refused>) -> bool {
        self.sending = None;
        match result {
            Ok(()) => true,
            Err(refused) => {
                self.refused = Some(crate::inbox::refusal(&refused));
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_message_is_the_source_then_the_selection_in_a_code_block_then_the_note() {
        assert_eq!(
            message("shell · ~/projects/paddock", "cargo test\nok", ""),
            "From shell · ~/projects/paddock:\n```\ncargo test\nok\n```"
        );
        assert_eq!(
            message("dev-terminal · claude", "error[E0308]", "  why this?  "),
            "From dev-terminal · claude:\n```\nerror[E0308]\n```\n\nwhy this?"
        );
        // A note of spaces alone is no note.
        assert_eq!(message("a", "b", " \n "), "From a:\n```\nb\n```");
    }

    #[test]
    fn the_selection_goes_as_it_was_without_the_spaces_at_its_line_ends() {
        // The grid pads lines with spaces; a line selection ends with its newline.
        assert_eq!(
            message("a", "  fn main() {   \n      body();\t\n  }    \n", ""),
            "From a:\n```\n  fn main() {\n      body();\n  }\n```"
        );
        // Blank lines inside stay; Windows line ends are line ends.
        assert_eq!(
            message("a", "one\r\n\r\ntwo  \r\n", ""),
            "From a:\n```\none\n\ntwo\n```"
        );
    }

    #[test]
    fn a_selection_with_backticks_gets_a_longer_fence() {
        assert_eq!(
            message("a", "```rust\nx\n```", ""),
            "From a:\n````\n```rust\nx\n```\n````"
        );
        assert_eq!(
            message("a", "a ````` b", ""),
            "From a:\n``````\na ````` b\n``````"
        );
        // Fewer than three in a row need nothing more.
        assert_eq!(
            message("a", "`x` ``y``", ""),
            "From a:\n```\n`x` ``y``\n```"
        );
    }

    #[test]
    fn the_source_names_a_shell_by_its_directory_and_an_agent_by_its_kind() {
        let home = Some("/Users/me");
        assert_eq!(
            source("shell · /Users/me/projects/paddock", None, home),
            "shell · ~/projects/paddock"
        );
        assert_eq!(source("shell · /Users/me", None, home), "shell · ~");
        assert_eq!(source("shell · /tmp", None, home), "shell · /tmp");
        assert_eq!(
            source("paddock/dev-terminal", Some("claude"), home),
            "paddock/dev-terminal · claude"
        );
        assert_eq!(source("main", None, home), "main");
        assert_eq!(source("htop -d 5", None, None), "htop -d 5");
    }

    fn listed<'a>(name: &'a str, status: Status) -> Listed<'a> {
        Listed {
            name,
            kind: Some("claude"),
            status,
        }
    }

    fn names(targets: &[Target]) -> Vec<(&str, bool)> {
        targets.iter().map(|t| (t.name.as_str(), t.own)).collect()
    }

    #[test]
    fn the_panes_own_agent_comes_first_then_the_sidebars_order() {
        let sidebar = [
            listed("p/a", Status::Waiting),
            listed("p/b", Status::Working),
            listed("p/c", Status::Idle),
            listed("p/gone", Status::Exited),
        ];
        assert_eq!(
            names(&targets(&sidebar, Some("p/c"))),
            [("p/c", true), ("p/a", false), ("p/b", false)]
        );
        assert_eq!(
            targets(&sidebar, Some("p/c"))[0].kind.as_deref(),
            Some("claude")
        );
        // A shell pane has none of its own.
        assert_eq!(
            names(&targets(&sidebar, None)),
            [("p/a", false), ("p/b", false), ("p/c", false)]
        );
        // Its agent no longer listed, or ended: none is marked.
        assert_eq!(
            names(&targets(&sidebar, Some("p/gone"))),
            [("p/a", false), ("p/b", false), ("p/c", false)]
        );
        // One alone is listed all the same.
        assert_eq!(names(&targets(&sidebar[..1], Some("p/a"))), [("p/a", true)]);
    }

    fn three() -> Vec<Target> {
        ["own", "b", "c"]
            .iter()
            .enumerate()
            .map(|(at, name)| Target {
                name: (*name).into(),
                kind: None,
                own: at == 0,
            })
            .collect()
    }

    #[test]
    fn the_first_row_is_chosen_and_the_keys_move_within_the_list() {
        let targets = three();
        let mut panel = Panel::new("x".into(), "shell · ~".into());
        assert_eq!(panel.chosen(&targets), Some(0));
        panel.step(-1, &targets);
        assert_eq!(panel.chosen(&targets), Some(0));
        panel.step(1, &targets);
        panel.step(1, &targets);
        panel.step(1, &targets);
        assert_eq!(panel.chosen(&targets), Some(2));
        // The one chosen stays chosen as the list changes around it.
        let fewer = vec![targets[2].clone()];
        assert_eq!(panel.chosen(&fewer), Some(0));
        // Gone from the list: the first is.
        panel.choose("b");
        assert_eq!(panel.chosen(&fewer), Some(0));
        assert_eq!(panel.chosen(&[]), None);
    }

    #[test]
    fn sending_goes_once_to_the_one_chosen_and_a_refusal_keeps_everything() {
        let targets = three();
        let mut panel = Panel::new("let x = 1;   ".into(), "shell · ~".into());
        panel.note = "look".into();
        panel.choose("c");
        let (to, words) = panel.start(&targets).unwrap();
        assert_eq!(to, "c");
        assert_eq!(words, "From shell · ~:\n```\nlet x = 1;\n```\n\nlook");
        // Not twice while on its way.
        assert_eq!(panel.start(&targets), None);
        assert_eq!(panel.sending.as_deref(), Some("c"));
        let refused = Refused {
            code: Some("not_idle".into()),
            message: "agent is working".into(),
        };
        assert!(!panel.finish(Err(refused)));
        assert_eq!(panel.sending, None);
        assert_eq!(
            panel.refused.as_deref(),
            Some("It's busy — try again when this turn ends")
        );
        assert_eq!(
            (panel.text.as_str(), panel.note.as_str()),
            ("let x = 1;   ", "look")
        );
        assert_eq!(panel.chosen(&targets), Some(2));
        // Tried again, what was said goes; taken, the panel is done.
        assert!(panel.start(&targets).is_some());
        assert_eq!(panel.refused, None);
        assert!(panel.finish(Ok(())));
        // No one to send to: nothing goes.
        let mut empty = Panel::new("x".into(), "a".into());
        assert_eq!(empty.start(&[]), None);
        assert_eq!(empty.sending, None);
    }

    /// The whole way through a fake `corral`, never a real one: refused, the panel keeps the text
    /// and the note; taken, corral got the message as one argument.
    #[test]
    fn through_a_fake_corral_a_refusal_keeps_the_text_and_the_note() {
        use crate::corral::Client;
        use std::{os::unix::fs::PermissionsExt, sync::atomic::AtomicBool};
        let dir = std::env::temp_dir().join(format!("paddock-send-to-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let fake = |name: &str, script: &str| {
            let path = dir.join(name);
            std::fs::write(&path, format!("#!/bin/sh\n{script}\n")).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
            Client::local(path.display().to_string())
        };
        let busy = fake(
            "busy",
            r#"echo '{"ok":false,"error":"not_idle","message":"agent is working"}'; exit 1"#,
        );
        let targets = three();
        let mut panel = Panel::new("a\nb  ".into(), "shell · ~".into());
        panel.note = "why?".into();
        let (to, words) = panel.start(&targets).unwrap();
        let result = busy.send(&to, &words, &AtomicBool::new(false));
        assert!(!panel.finish(result));
        assert_eq!(
            panel.refused.as_deref(),
            Some("It's busy — try again when this turn ends")
        );
        assert_eq!(
            (panel.text.as_str(), panel.note.as_str()),
            ("a\nb  ", "why?")
        );
        // corral not answering in its JSON: its sentence, all kept the same.
        let broken = fake("broken", "echo oops >&2; exit 3");
        let (to, words) = panel.start(&targets).unwrap();
        assert!(!panel.finish(broken.send(&to, &words, &AtomicBool::new(false))));
        assert_eq!(
            panel.refused.as_deref(),
            Some("corral did not say how it went")
        );
        assert_eq!(panel.note, "why?");
        // Taken: corral had `send NAME MESSAGE`, the message one argument.
        let calls = dir.join("calls");
        let taken = fake(
            "taken",
            &format!(
                r#"printf '%s|' "$@" >> '{}'; echo '{{"ok":true}}'"#,
                calls.display()
            ),
        );
        let (to, words) = panel.start(&targets).unwrap();
        assert!(panel.finish(taken.send(&to, &words, &AtomicBool::new(false))));
        assert_eq!(
            std::fs::read_to_string(&calls).unwrap(),
            "send|own|From shell · ~:\n```\na\nb\n```\n\nwhy?|"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
