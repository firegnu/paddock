//! `paddock [OPTIONS] [-- PROGRAM ARG…]`: one window, the Agents sidebar and one terminal pane.
use anyhow::{Context as _, Result, bail};
use gpui::{App, AppContext as _, Bounds, WindowBounds, WindowOptions, point, px, size};
use paddock::{
    config::{self, Config},
    theme::Theme,
    view::{Launch, Options},
    window::PaddockWindow,
};

const HELP: &str = "paddock [--attach NAME] [--corral PROGRAM] [--cwd DIR] [--font FAMILY] \
[--fallback FAMILY]… [--size PX] [--line-height FACTOR] [--bounds X,Y,W,H] [--stats] [-- PROGRAM ARG…]

Without --attach or a program it starts $SHELL -i in --cwd (default: the current directory).
--attach shows an existing corral agent with `corral attach NAME`, as Saddle does.
--stats prints frame timing to stderr once a second.
Settings are read from ~/.config/paddock/config.toml.";

const DEFAULT_FALLBACKS: [&str; 3] = [
    "Symbols Nerd Font Mono",
    "FiraCode Nerd Font Mono",
    "FiraCode Nerd Font",
];

/// Identity this process may have inherited (for example when started from a Corral agent or a Claude
/// Code session) that must not reach the programs in the pane. Saddle's `spawn_shell` strips the
/// first five for shells; `Session::spawn` takes no environment, so paddock clears them for every
/// launch. Only these names: other `CLAUDE_CODE_*` variables are the user's own settings.
const INHERITED: [&str; 17] = [
    "CORRAL_NAME",
    "CORRAL_INSTANCE",
    "SADDLE_INSTANCE",
    "SADDLE_PANE",
    "SADDLE_REVISION",
    "CORRAL_EVENTS",
    "CLAUDECODE",
    "CLAUDE_PID",
    "CLAUDE_CODE_SESSION_ID",
    "CLAUDE_CODE_CHILD_SESSION",
    "CLAUDE_CODE_SESSION_ATTENDED",
    "CLAUDE_CODE_ENTRYPOINT",
    "CLAUDE_CODE_EXECPATH",
    "CLAUDE_CODE_MESSAGING_SOCKET",
    "CLAUDE_CODE_MESSAGING_TOKEN",
    "CODEX_COMPANION_SESSION_ID",
    "CODEX_COMPANION_TRANSCRIPT_PATH",
];

fn main() -> Result<()> {
    for key in INHERITED {
        // SAFETY: first thing in main, before any other thread exists.
        unsafe { std::env::remove_var(key) };
    }
    let mut args = std::env::args().skip(1);
    let mut attach = None;
    let mut corral = "corral".to_owned();
    let mut cwd = None;
    let mut font_family = "Menlo".to_owned();
    let mut fallbacks = Vec::new();
    let mut font_size = 14.0;
    let mut line_height = 1.3;
    let mut window = None;
    let mut stats = false;
    let mut command = Vec::new();
    while let Some(arg) = args.next() {
        let mut value = || args.next().with_context(|| format!("{arg} needs a value"));
        match arg.as_str() {
            "--attach" => attach = Some(value()?),
            "--corral" => corral = value()?,
            "--cwd" => cwd = Some(value()?),
            "--font" => font_family = value()?,
            "--fallback" => fallbacks.push(value()?),
            "--size" => font_size = value()?.parse()?,
            "--line-height" => line_height = value()?.parse()?,
            "--bounds" => {
                let parts: Vec<f32> = value()?
                    .split(',')
                    .map(str::parse)
                    .collect::<Result<_, _>>()?;
                let [x, y, w, h] = parts[..] else {
                    bail!("--bounds takes X,Y,W,H")
                };
                window = Some((x, y, w, h));
            }
            "--stats" => stats = true,
            "--help" | "-h" => {
                println!("{HELP}");
                return Ok(());
            }
            "--" => {
                command = args.by_ref().collect();
                break;
            }
            other => bail!("unknown argument {other}; see --help"),
        }
    }
    let cwd = match cwd {
        Some(dir) => dir,
        None => std::env::current_dir()?.display().to_string(),
    };
    let launch = if !command.is_empty() {
        Launch::Command {
            argv: command,
            cwd: Some(cwd.into()),
        }
    } else if let Some(name) = attach {
        Launch::Agent { corral, name }
    } else {
        let program = std::env::var("SHELL")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or("/bin/sh".into());
        Launch::Shell { program, cwd }
    };
    if fallbacks.is_empty() {
        // Common Nerd Font families for prompt icons; missing ones are skipped.
        fallbacks = DEFAULT_FALLBACKS.iter().map(|s| s.to_string()).collect();
    }
    let options = Options {
        launch,
        font_family,
        fallbacks,
        font_size,
        line_height,
        stats,
    };
    let config = Config::load(&config::default_path())?;
    let theme = Theme::from_config(&config)?;

    gpui_platform::application().run(move |cx: &mut App| {
        let bounds = match window {
            Some((x, y, w, h)) => Bounds::new(point(px(x), px(y)), size(px(w), px(h))),
            None => Bounds::centered(None, size(px(1000.0), px(640.0)), cx),
        };
        cx.on_window_closed(|cx, _| cx.quit()).detach();
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| {
                if std::env::var_os("GPUI_TERM_WINDOW_ID").is_some() {
                    print_window_number(window);
                }
                cx.new(|cx| PaddockWindow::new(&config, theme, options, window, cx))
            },
        )
        .expect("open window");
        cx.activate(true);
    });
    Ok(())
}

/// Prints the macOS window number (`screencapture -l N`) so test screenshots never include other
/// windows on the screen.
fn print_window_number(window: &gpui::Window) {
    use objc2::{msg_send, runtime::AnyObject};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return;
    };
    if let RawWindowHandle::AppKit(appkit) = handle.as_raw() {
        let view = appkit.ns_view.as_ptr().cast::<AnyObject>();
        // SAFETY: GPUI hands out its live NSView; `window` and `windowNumber` are plain getters.
        let number: isize = unsafe {
            let ns_window: *mut AnyObject = msg_send![view, window];
            msg_send![ns_window, windowNumber]
        };
        eprintln!("window-id: {number}");
    }
}
