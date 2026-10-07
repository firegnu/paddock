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
Settings are read from ~/.config/paddock/config.toml.\nHeadless commands: paddock ctl --help; paddock install-skills --help.";

/// Identity this process may have inherited (for example when started from a Corral agent or a Claude
/// Code session) that must not reach the programs in the pane. Saddle's `spawn_shell` strips the
/// Corral/Saddle identity for shells; `Session::spawn` takes no environment, so paddock clears
/// these for every launch. Only these names: other `CLAUDE_CODE_*` variables are the user's own settings.
const INHERITED: [&str; 19] = [
    "PADDOCK_INSTANCE",
    "PADDOCK_PANE",
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
    // Headless commands keep caller identity and skip desktop PATH/config/window setup.
    let mut command_args = std::env::args().skip(1);
    match command_args.next().as_deref() {
        Some("ctl") => std::process::exit(paddock::control::run(command_args.collect())),
        Some("install-skills") => std::process::exit(paddock::skills::run(command_args.collect())),
        _ => {}
    }
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
    // Kept outside the event loop too, so both normal quit and startup unwind clean up.
    // paddock ctl is a helper: when it cannot start, paddock still opens and says why.
    let (server, ctl_problem) = match paddock::control::Server::start() {
        Ok(server) => (Some(server), None),
        Err(error) => (None, Some(format!("paddock ctl unavailable: {error:#}"))),
    };
    let control = std::rc::Rc::new(std::cell::RefCell::new(server));
    // The sidebar's "new shell" starts the same shell in the same directory; shells carry this
    // instance for `paddock ctl`.
    let new_shell = NewShell {
        program: program.clone(),
        cwd: cwd.clone(),
        instance: control
            .borrow()
            .as_ref()
            .map(|server| server.id.clone())
            .unwrap_or_default(),
        ctl_problem,
    };
    let launch = if !command.is_empty() {
        Launch::Command {
            argv: command,
            cwd: Some(cwd.into()),
        }
    } else if let Some(name) = attach {
        Launch::Agent {
            name,
            metadata: Default::default(),
        }
    } else {
        Launch::Shell {
            program,
            cwd,
            env: Vec::new(),
        }
    };
    // Started plainly, paddock opens the saved layout; asked for an agent or a program, it opens
    // that and leaves the saved layout alone.
    let path = paddock::layout_state::default_path();
    let (store, saved) = Store::open(path.clone());
    let window_size = saved
        .as_ref()
        .map(|layout| layout.window_size)
        .unwrap_or_default();
    let layout = if matches!(launch, Launch::Shell { .. }) {
        (store, saved)
    } else {
        (Store::skip(path), None)
    };
    let mut config = Config::load(&config::default_path())?;
    config.apply_font_overrides(font_family, fallbacks, font_size, line_height)?;
    config.sidebar_width = paddock::sidebar::fit_width(
        config.sidebar_width,
        &paddock::fonts::UiFont::from_config(&config),
    );
    let mut options = Options {
        launch,
        corral: corral.unwrap_or_else(|| config.corral.clone()),
        font_family: config.font.clone(),
        fallbacks: config.font_fallbacks.clone(),
        font_size: config.font_size,
        line_height: config.line_height,
        stats,
    };
    // `--attach NAME`: the instance to attach to, from public corral, so the pane can stand for
    // its agent in `paddock ctl`.
    if let Launch::Agent { name, metadata } = &mut options.launch {
        *metadata = paddock::viewer::public_metadata(&options.corral, name);
    }
    let theme = Theme::from_config(&config)?;

    let ui_control = control.clone();
    gpui_platform::application().run(move |cx: &mut App| {
        let bounds = match window {
            Some((x, y, w, h)) => Bounds::new(point(px(x), px(y)), size(px(w), px(h))),
            None => Bounds::centered(
                None,
                size(px(window_size.width), px(window_size.height)),
                cx,
            ),
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
        cx.set_menus(menu::menus(false, false, false, false));
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
                |main_window, cx| {
                    if window.is_none() {
                        // The native minimum is available only after the window is created.
                        paddock::window::restore_size(main_window, window_size, cx);
                    }
                    let window = main_window;
                    windows::announce(window);
                    cx.new(|cx| {
                        PaddockWindow::new(&config, theme, options, new_shell, layout, window, cx)
                    })
                },
            )
            .expect("open window");
        windows::set_main(main, cx);
        let quitting = ui_control.clone();
        cx.on_app_quit(move |_| {
            // Drop unlinks the registration and joins bounded transport workers.
            quitting.borrow_mut().take();
            async {}
        })
        .detach();
        let executor = cx.background_executor().clone();
        cx.spawn(async move |cx| {
            loop {
                executor.timer(std::time::Duration::from_millis(10)).await;
                let alive = cx.update(|cx| {
                    let mut server = ui_control.borrow_mut();
                    let Some(server) = server.as_mut() else {
                        return false;
                    };
                    server.process_pending(|message, records| {
                        main.update(cx, |main, window, cx| {
                            main.control(message, records, window, cx)
                        })
                        .unwrap_or_else(|_| {
                            let mut value = paddock::control::error(
                                "instance_unavailable",
                                "the window has closed",
                            );
                            value["state"] = "failed".into();
                            value
                        })
                    });
                    if let Ok(updates) =
                        main.update(cx, |main, window, cx| main.control_tick(window, cx))
                    {
                        for (request, value) in updates {
                            server.records.update(&request, value);
                        }
                    }
                    true
                });
                if !alive {
                    break;
                }
            }
        })
        .detach();
        // Test and screenshot runs set PADDOCK_NO_ACTIVATE so the window does not take the
        // keyboard from the app the user is typing in.
        if std::env::var_os("PADDOCK_NO_ACTIVATE").is_none() {
            cx.activate(true);
        }
    });
    drop(control);
    Ok(())
}
