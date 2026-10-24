//! The terminal's own colours sent to corral for agents started from now on (P5-181).
use crate::theme::Theme;
use std::{
    sync::atomic::AtomicBool,
    time::{Duration, SystemTime},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Colors {
    pub fg: String,
    pub bg: String,
}

impl Colors {
    pub fn of(theme: &Theme) -> Self {
        let hex = |(r, g, b)| format!("#{r:02x}{g:02x}{b:02x}");
        let terminal = theme.own_terminal();
        Self {
            fg: hex(terminal.foreground),
            bg: hex(terminal.background),
        }
    }

    pub fn args(&self) -> [&str; 5] {
        ["terminal-colors", "--fg", &self.fg, "--bg", &self.bg]
    }

    fn send(&self, corral: &str, timeout: Duration) -> Result<(), String> {
        crate::corral::Client::local(corral)
            .json(&self.args(), timeout, &AtomicBool::new(false))
            .map(|_| ())
            .map_err(|error| format!("{error:#}"))
    }
}

#[derive(Default)]
struct Sync {
    last: Option<Colors>,
    task: Option<gpui::Task<()>>,
}

impl gpui::Global for Sync {}

impl Sync {
    fn changed(&mut self, colors: &Colors) -> bool {
        if self.last.as_ref() == Some(colors) {
            return false;
        }
        self.last = Some(colors.clone());
        true
    }
}

/// The latest call, including its colours while it is still running.
#[derive(Clone, Default)]
pub struct Status(pub Option<Attempt>);

impl gpui::Global for Status {}

#[derive(Clone)]
pub struct Attempt {
    pub colors: Colors,
    pub at: SystemTime,
    pub result: Option<Result<(), String>>,
}

/// Called at startup and when applying a theme. All windows share one queue; each write finishes
/// before the next begins, so an older theme cannot overwrite a newer one. Never waits on the UI.
pub fn sync(theme: &Theme, corral: &str, cx: &mut gpui::App) {
    let colors = Colors::of(theme);
    let state = cx.default_global::<Sync>();
    if !state.changed(&colors) {
        return;
    }
    let previous = state.task.take();
    let corral = corral.to_owned();
    let task = cx.spawn(async move |cx| {
        if let Some(previous) = previous {
            previous.await;
        }
        let mut attempt = Attempt {
            colors: colors.clone(),
            at: SystemTime::now(),
            result: None,
        };
        cx.update(|cx| cx.set_global(Status(Some(attempt.clone()))));
        let result = cx
            .background_executor()
            .spawn(async move { colors.send(&corral, Duration::from_secs(5)) })
            .await;
        attempt.result = Some(result);
        cx.update(|cx| cx.set_global(Status(Some(attempt))));
    });
    cx.default_global::<Sync>().task = Some(task);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::Config, remote::quote};
    use std::os::unix::fs::PermissionsExt;

    fn theme(text: &str) -> Theme {
        Theme::from_config(&Config::parse(text).unwrap()).unwrap()
    }

    #[test]
    fn terminal_colors_start_once_change_once_and_ignore_fading_and_other_settings() {
        let dir = std::env::temp_dir().join(format!("paddock-colors-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("args");
        let corral = dir.join("corral");
        std::fs::write(
            &corral,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$*\" >> {}\necho '{{\"ok\":true}}'\n",
                quote(log.to_str().unwrap()),
            ),
        )
        .unwrap();
        std::fs::set_permissions(&corral, std::fs::Permissions::from_mode(0o755)).unwrap();
        let mut sync = Sync::default();
        let mut send = |theme: &Theme| {
            let colors = Colors::of(theme);
            if sync.changed(&colors) {
                colors
                    .send(corral.to_str().unwrap(), Duration::from_secs(5))
                    .unwrap();
            }
        };
        let first = theme("theme = 'dune'");
        send(&first);
        assert!(log.exists(), "startup must send terminal colors");
        send(&first); // Another window with the shared config.
        send(&first.faded());
        send(&theme("theme = 'dune'\nfont_size = 18"));
        let second =
            theme("theme = 'latte'\n[colors]\nterminal_text = '#ABCDEF'\nterminal_bg = '#123456'");
        send(&second.faded());
        send(&second);
        let lines = std::fs::read_to_string(&log).unwrap();
        let first = Colors::of(&first);
        assert_eq!(
            lines,
            format!(
                "terminal-colors --fg {} --bg {}\nterminal-colors --fg #abcdef --bg #123456\n",
                first.fg, first.bg,
            )
        );
        // An old corral's failure is recorded, not returned to the caller changing the theme.
        std::fs::write(
            &corral,
            "#!/bin/sh\necho '{\"ok\":false,\"error\":\"usage\"}'\nexit 1\n",
        )
        .unwrap();
        let third = Colors::of(&theme("theme = 'tide'"));
        assert!(sync.changed(&third));
        assert!(
            third
                .send(corral.to_str().unwrap(), Duration::from_secs(5))
                .unwrap_err()
                .contains("usage")
        );
        assert!(!sync.changed(&third));
        assert!(sync.changed(&first));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
