//! `paddock [OPTIONS] [-- PROGRAM ARG…]`: one window, the Agents sidebar and one terminal pane.
use anyhow::{Context as _, Result, bail};
use gpui::{
    App, AppContext as _, Bounds, TitlebarOptions, WindowBounds, WindowOptions, point, px, size,
};
use paddock::{
    config::{self, Config},
    layout_state::Store,
    menu,
    theme::Theme,
    view::{Launch, Options},
    window::NewShell,
    window::PaddockWindow,
    windows,
};

const HELP: &str = "paddock [--attach NAME] [--corral PROGRAM] [--cwd DIR] [--font FAMILY] \
[--fallback FAMILY]… [--size PX] [--line-height FACTOR] [--bounds X,Y,W,H] [--stats] [-- PROGRAM ARG…]

Without --attach or a program it starts $SHELL -i in --cwd (default: the current directory).
--attach shows an existing corral agent with `corral attach NAME`, as Saddle does.
--stats prints frame timing to stderr once a second.
Settings are read from ~/.config/paddock/config.toml.";

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
    // Started from Finder or the Dock: take the login shell's PATH, still before any thread.
    let desktop = paddock::launch::from_desktop();
    let mut startup = paddock::diagnostics::Startup {
        desktop,
        login_path: None,
    };
    if desktop {
        let shell = std::env::var("SHELL")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or("/bin/zsh".into());
        let timeout = std::time::Duration::from_secs(3);
        let path = paddock::launch::login_path(std::path::Path::new(&shell), timeout);
        startup.login_path = Some(path.is_some());
        if let Some(path) = path {
            // SAFETY: the login shell ran on this thread; no other thread exists yet.
            unsafe { std::env::set_var("PATH", path) };
        }
    }
    let mut args = std::env::args().skip(1);
    let mut attach = None;
    let mut corral = None;
    let mut cwd = None;
    let mut font_family = None;
    let mut fallbacks = Vec::new();
    let mut font_size = None;
    let mut line_height = None;
    let mut window = None;
    let mut stats = false;
    let mut command = Vec::new();
    while let Some(arg) = args.next() {
        let mut value = || args.next().with_context(|| format!("{arg} needs a value"));
        match arg.as_str() {
            "--attach" => attach = Some(value()?),
            "--corral" => corral = Some(value()?),
            "--cwd" => cwd = Some(value()?),
            "--font" => font_family = Some(value()?),
            "--fallback" => fallbacks.push(value()?),
            "--size" => {
                font_size = Some(
                    value()?
                        .parse()
                        .context("--size (font_size) must be a number")?,
                )
            }
            "--line-height" => {
                line_height = Some(value()?.parse().context("--line-height must be a number")?)
            }
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
        // From the desktop the working directory is `/`; shells start at home instead.
        None if desktop => std::env::var("HOME").unwrap_or_else(|_| "/".into()),
        None => std::env::current_dir()?.display().to_string(),
    };
    let program = std::env::var("SHELL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or("/bin/sh".into());
    // The sidebar's "new shell" starts the same shell in the same directory.
    let new_shell = NewShell {
        program: program.clone(),
        cwd: cwd.clone(),
    };
    let launch = if !command.is_empty() {
        Launch::Command {
            argv: command,
            cwd: Some(cwd.into()),
        }
    } else if let Some(name) = attach {
        Launch::Agent { name }
    } else {
        Launch::Shell { program, cwd }
    };
    // Started plainly, paddock opens the saved layout; asked for an agent or a program, it opens
    // that and leaves the saved layout alone.
    let path = paddock::layout_state::default_path();
    let layout = if matches!(launch, Launch::Shell { .. }) {
        Store::open(path)
    } else {
        (Store::skip(path), None)
    };
    let mut config = Config::load(&config::default_path())?;
    config.apply_font_overrides(font_family, fallbacks, font_size, line_height)?;
    let options = Options {
        launch,
        corral: corral.unwrap_or_else(|| config.corral.clone()),
        font_family: config.font.clone(),
        fallbacks: config.font_fallbacks.clone(),
        font_size: config.font_size,
        line_height: config.line_height,
        stats,
    };
    let theme = Theme::from_config(&config)?;

    gpui_platform::application().run(move |cx: &mut App| {
        let bounds = match window {
            Some((x, y, w, h)) => Bounds::new(point(px(x), px(y)), size(px(w), px(h))),
            None => Bounds::centered(None, size(px(1280.0), px(800.0)), cx),
        };
        cx.set_global(startup);
        cx.set_global(paddock::fonts::UiFont::from_config(&config));
        cx.bind_keys(menu::bindings());
        cx.bind_keys(paddock::text_input::bindings());
        cx.on_action(|_: &menu::Quit, cx| windows::quit(cx));
        // Opening or raising a window reads it from App; wait until this event's window is back.
        cx.on_action(|_: &menu::OpenSettings, cx| cx.defer(windows::open_settings));
        cx.on_action(|_: &menu::About, cx| cx.defer(windows::open_about));
        cx.on_action(|_: &menu::NewAgent, cx| {
            cx.defer(|cx| windows::open_new_agent(paddock::new_agent::Place::Current, cx))
        });
        cx.set_menus(menu::menus(false, false, false));
        let main = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    // The tabs share the title bar's row with the traffic lights; the title is
                    // still set, for Mission Control and the Window menu, just not drawn.
                    titlebar: Some(TitlebarOptions {
                        title: Some("paddock".into()),
                        appears_transparent: true,
                        // Where the first frame puts them; the window keeps them centred after.
                        traffic_light_position: Some(paddock::window::traffic_lights(
                            if config.mascot_enabled {
                                paddock::window::PET_TITLE_BAR
                            } else {
                                paddock::window::TITLE_BAR
                            },
                        )),
                    }),
                    // The window drags itself from the empty parts of its title bar.
                    app_owns_titlebar_drag: true,
                    ..Default::default()
                },
                |window, cx| {
                    windows::announce(window);
                    cx.new(|cx| {
                        PaddockWindow::new(&config, theme, options, new_shell, layout, window, cx)
                    })
                },
            )
            .expect("open window");
        windows::set_main(main, cx);
        // Test and screenshot runs set PADDOCK_NO_ACTIVATE so the window does not take the
        // keyboard from the app the user is typing in.
        if std::env::var_os("PADDOCK_NO_ACTIVATE").is_none() {
            cx.activate(true);
        }
    });
    Ok(())
}
