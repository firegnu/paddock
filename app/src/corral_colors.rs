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
    /// Only a successful write counts. Check it after earlier queued writes have finished.
    last: Option<Colors>,
    current: Option<(Colors, String)>,
    task: Option<gpui::Task<()>>,
    _startup: Option<gpui::Subscription>,
}

impl gpui::Global for Sync {}

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
    if !cx.has_global::<Sync>() {
        let startup = cx.observe_global::<crate::diagnostics::Startup>(|cx| {
            if cx.global::<crate::diagnostics::Startup>().login_path
                == Some(crate::launch::LoginPath::Ready)
                && let Some((colors, corral)) = cx.global::<Sync>().current.clone()
            {
                enqueue(colors, corral, cx);
            }
        });
        cx.set_global(Sync {
            _startup: Some(startup),
            ..Sync::default()
        });
    }
    let colors = Colors::of(theme);
    let corral = corral.to_owned();
    cx.global_mut::<Sync>().current = Some((colors.clone(), corral.clone()));
    enqueue(colors, corral, cx);
}

fn enqueue(colors: Colors, corral: String, cx: &mut gpui::App) {
    let previous = cx.global_mut::<Sync>().task.take();
    let task = cx.spawn(async move |cx| {
        if let Some(previous) = previous {
            previous.await;
        }
        if cx.update(|cx| cx.global::<Sync>().last.as_ref() == Some(&colors)) {
            return;
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
        cx.update(|cx| {
            if attempt.result == Some(Ok(())) {
                cx.global_mut::<Sync>().last = Some(attempt.colors.clone());
            }
            cx.set_global(Status(Some(attempt)));
        });
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

    fn fake_corral(dir: &std::path::Path, success: bool) -> String {
        let corral = dir.join("corral");
        let reply = if success {
            "echo '{\"ok\":true}'"
        } else {
            "echo '{\"ok\":false,\"error\":\"usage\"}'; exit 1"
        };
        std::fs::write(
            &corral,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$*\" >> {}\n{reply}\n",
                quote(dir.join("calls").to_str().unwrap()),
            ),
        )
        .unwrap();
        std::fs::set_permissions(&corral, std::fs::Permissions::from_mode(0o755)).unwrap();
        corral.to_str().unwrap().into()
    }

    #[gpui::test]
    fn failed_colors_retry_until_success(cx: &mut gpui::TestAppContext) {
        let dir = std::env::temp_dir().join(format!("paddock-colors-retry-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let corral = fake_corral(&dir, false);
        let chosen = theme("theme = 'latte'");
        cx.update(|cx| sync(&chosen, &corral, cx));
        cx.run_until_parked();
        cx.update(|cx| {
            assert!(
                cx.global::<Status>()
                    .0
                    .as_ref()
                    .unwrap()
                    .result
                    .as_ref()
                    .unwrap()
                    .is_err()
            )
        });
        fake_corral(&dir, true);
        // The same colours after upgrading corral must retry. Concurrent windows deduplicate
        // after that attempt succeeds, rather than treating an in-flight call as a success.
        cx.update(|cx| {
            sync(&chosen, &corral, cx);
            sync(&chosen, &corral, cx);
        });
        cx.run_until_parked();
        assert_eq!(
            std::fs::read_to_string(dir.join("calls"))
                .unwrap()
                .lines()
                .count(),
            2
        );
        cx.update(|cx| {
            assert_eq!(
                cx.global::<Status>().0.as_ref().unwrap().result,
                Some(Ok(()))
            )
        });
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[gpui::test]
    fn ready_login_path_retries_the_current_theme(cx: &mut gpui::TestAppContext) {
        use crate::{diagnostics::Startup, launch::LoginPath};
        let dir = std::env::temp_dir().join(format!("paddock-colors-ready-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let corral = dir.join("corral").to_str().unwrap().to_owned();
        cx.update(|cx| {
            cx.set_global(Startup {
                desktop: true,
                login_path: Some(LoginPath::Loading),
            });
            sync(&theme("theme = 'dune'"), &corral, cx);
        });
        cx.run_until_parked();
        // A settings change while PATH is still unavailable is the theme to retry later.
        let chosen = theme("theme = 'latte'");
        cx.update(|cx| sync(&chosen, &corral, cx));
        cx.run_until_parked();
        cx.update(|cx| {
            assert!(
                cx.global::<Status>()
                    .0
                    .as_ref()
                    .unwrap()
                    .result
                    .as_ref()
                    .unwrap()
                    .is_err()
            )
        });
        fake_corral(&dir, true);
        cx.update(|cx| {
            cx.set_global(Startup {
                desktop: true,
                login_path: Some(LoginPath::Failed),
            })
        });
        cx.run_until_parked();
        assert!(!dir.join("calls").exists());
        cx.update(|cx| {
            cx.set_global(Startup {
                desktop: true,
                login_path: Some(LoginPath::Ready),
            })
        });
        cx.run_until_parked();
        assert!(
            dir.join("calls").exists(),
            "PATH Ready must trigger a retry"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("calls")).unwrap(),
            format!("{}\n", Colors::of(&chosen).args().join(" "))
        );
        cx.update(|cx| {
            assert_eq!(
                cx.global::<Status>().0.as_ref().unwrap().result,
                Some(Ok(()))
            )
        });
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[gpui::test]
    fn terminal_colors_start_once_change_once_and_ignore_fading_and_other_settings(
        cx: &mut gpui::TestAppContext,
    ) {
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
        let send = |theme: &Theme| {
            cx.update(|cx| sync(theme, corral.to_str().unwrap(), cx));
            cx.run_until_parked();
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
        send(&theme("theme = 'tide'"));
        cx.update(|cx| {
            assert!(
                cx.global::<Status>()
                    .0
                    .as_ref()
                    .unwrap()
                    .result
                    .as_ref()
                    .unwrap()
                    .as_ref()
                    .unwrap_err()
                    .contains("usage")
            );
            assert_eq!(cx.global::<Sync>().last, Some(Colors::of(&second)));
        });
        std::fs::remove_dir_all(dir).unwrap();
    }
}
