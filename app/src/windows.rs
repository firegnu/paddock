//! paddock's windows: the main one, and at most one Settings and one About window beside it.
//! Opening one that is already open brings it to the front. Quitting, or closing the main window,
//! first lets a Settings window with unsaved edits ask what to do with them.
use crate::{
    about::AboutView,
    new_agent::Place,
    new_agent_view::{NewAgentEvent, NewAgentView},
    settings_view::{SettingsEvent, SettingsView},
    window::PaddockWindow,
};
use gpui::{
    App, AppContext as _, Bounds, Entity, Focusable, Global, Render, TitlebarOptions, Window,
    WindowBounds, WindowHandle, WindowOptions, px, size,
};

#[derive(Default)]
struct Windows {
    main: Option<WindowHandle<PaddockWindow>>,
    settings: Option<WindowHandle<SettingsView>>,
    about: Option<WindowHandle<AboutView>>,
    new_agent: Option<WindowHandle<NewAgentView>>,
    /// A quit is waiting on Settings' answer.
    quitting: bool,
}

impl Global for Windows {}

/// Remembers the main window; closing it quits, after the same questions as ⌘Q.
pub fn set_main(handle: WindowHandle<PaddockWindow>, cx: &mut App) {
    cx.default_global::<Windows>().main = Some(handle);
    let _ = handle.update(cx, |_, window, cx| {
        window.on_window_should_close(cx, |_, cx| {
            quit(cx);
            false
        })
    });
}

/// Brings `handle`'s window to the front; false when it is gone.
fn raise<V: 'static>(handle: Option<WindowHandle<V>>, cx: &mut App) -> bool {
    handle.is_some_and(|handle| {
        handle
            .update(cx, |_, window, _| window.activate_window())
            .is_ok()
    })
}

fn options(
    title: &'static str,
    width: f32,
    height: f32,
    resizable: bool,
    cx: &App,
) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
            None,
            size(px(width), px(height)),
            cx,
        ))),
        titlebar: Some(TitlebarOptions {
            title: Some(title.into()),
            ..Default::default()
        }),
        is_resizable: resizable,
        is_minimizable: resizable,
        window_min_size: Some(size(px(width * 0.75), px(height * 0.75))),
        ..Default::default()
    }
}

/// Opens a window showing `make`'s view and focuses it.
fn open<V: Render + Focusable>(
    options: WindowOptions,
    make: impl FnOnce(&mut Window, &mut gpui::Context<V>) -> V + 'static,
    cx: &mut App,
) -> Option<(WindowHandle<V>, Entity<V>)> {
    let mut made = None;
    let handle = cx
        .open_window(options, |window, cx| {
            announce(window);
            let view = cx.new(|cx| make(window, cx));
            let focus = view.read(cx).focus_handle(cx);
            window.focus(&focus, cx);
            made = Some(view.clone());
            view
        })
        .ok()?;
    Some((handle, made?))
}

pub fn open_settings(cx: &mut App) {
    let windows = cx.default_global::<Windows>();
    let (existing, main) = (windows.settings, windows.main);
    if raise(existing, cx) {
        return;
    }
    let Some(main) = main else { return };
    let Ok(theme) = main.read(cx).map(PaddockWindow::theme) else {
        return;
    };
    let options = options("Settings", 900.0, 640.0, true, cx);
    let Some((handle, settings)) = open(
        options,
        |window, cx| {
            let view = SettingsView::new(theme, crate::config::default_path(), cx);
            let this = cx.entity().downgrade();
            window.on_window_should_close(cx, move |window, cx| {
                let _ = this.update(cx, |view, cx| view.request_close(window, cx));
                false
            });
            view
        },
        cx,
    ) else {
        return;
    };
    // The main window takes up what Settings saves.
    let _ = main.update(cx, |_, _, cx| {
        cx.subscribe(&settings, |main, _, event: &SettingsEvent, cx| {
            let SettingsEvent::Saved { config, .. } = event;
            main.apply(config, cx);
        })
        .detach()
    });
    cx.default_global::<Windows>().settings = Some(handle);
}

/// The New Agent window, set to open the agent at `place`; one already open comes to the front and
/// keeps what was typed, taking `place` when it is not the current pane.
pub fn open_new_agent(place: Place, cx: &mut App) {
    let windows = cx.default_global::<Windows>();
    let (existing, main) = (windows.new_agent, windows.main);
    if let Some(handle) = existing
        && handle
            .update(cx, |view, window, cx| {
                window.activate_window();
                if place != Place::Current {
                    view.set_place(place, cx);
                }
            })
            .is_ok()
    {
        return;
    }
    let Some(main) = main else { return };
    let Ok(seed) = main.read(cx).map(|main| main.seed(cx)) else {
        return;
    };
    let options = options("New Agent", 640.0, 600.0, true, cx);
    let Some((handle, view)) = open(options, move |_, cx| NewAgentView::new(seed, place, cx), cx)
    else {
        return;
    };
    // The main window opens what was started.
    let _ = main.update(cx, |_, window, cx| {
        cx.subscribe_in(
            &view,
            window,
            |main, _, event: &NewAgentEvent, window, cx| {
                let NewAgentEvent::Started {
                    started,
                    cwd,
                    place,
                } = event;
                main.open_started(started, cwd, *place, window, cx);
            },
        )
        .detach()
    });
    cx.default_global::<Windows>().new_agent = Some(handle);
}

/// The Settings window, at `page`.
pub fn open_settings_at(page: crate::settings::Page, cx: &mut App) {
    open_settings(cx);
    if let Some(settings) = cx.default_global::<Windows>().settings {
        let _ = settings.update(cx, |view, _, cx| view.show_page(page, cx));
    }
}

pub fn open_about(cx: &mut App) {
    let windows = cx.default_global::<Windows>();
    let (existing, main) = (windows.about, windows.main);
    if raise(existing, cx) {
        return;
    }
    let Some(theme) = main.and_then(|main| main.read(cx).ok().map(PaddockWindow::theme)) else {
        return;
    };
    let options = options("About paddock", 380.0, 230.0, false, cx);
    let handle = open(options, |_, cx| AboutView::new(theme, cx), cx);
    cx.default_global::<Windows>().about = handle.map(|(handle, _)| handle);
}

/// Quits, once a Settings window with unsaved edits has been saved or its edits dropped, and live
/// shells may end; either Cancel keeps paddock running.
pub fn quit(cx: &mut App) {
    let windows = cx.default_global::<Windows>();
    if windows.quitting {
        return;
    }
    windows.quitting = true;
    let (settings, main) = (windows.settings, windows.main);
    cx.spawn(async move |cx| {
        let mut go = true;
        if let Some(settings) = settings
            && let Ok(answer) = settings.update(cx, |view, window, cx| {
                window.activate_window();
                view.confirm_close(window, cx)
            })
        {
            go = answer.await;
        }
        if go
            && let Some(main) = main
            && let Ok(answer) = main.update(cx, |view, window, cx| {
                window.activate_window();
                view.confirm_quit(window, cx)
            })
        {
            go = answer.await;
        }
        cx.update(|cx| {
            if go {
                cx.quit();
            } else {
                cx.default_global::<Windows>().quitting = false;
            }
        });
    })
    .detach();
}

/// With `GPUI_TERM_WINDOW_ID` set, prints the macOS window number (`screencapture -l N`) so test
/// screenshots never include other windows on the screen.
pub fn announce(window: &Window) {
    use objc2::{msg_send, runtime::AnyObject};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    if std::env::var_os("GPUI_TERM_WINDOW_ID").is_none() {
        return;
    }
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
