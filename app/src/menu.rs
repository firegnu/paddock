//! The menu bar, its actions and their keyboard shortcuts, in the usual macOS places.
use gpui::{KeyBinding, Menu, MenuItem, SystemMenuType, actions};

actions!(
    paddock,
    [
        About,
        Quit,
        NewTab,
        NewShell,
        SplitRight,
        SplitDown,
        SplitLeft,
        SplitUp,
        ClosePane,
        CloseTab,
        Copy,
        Paste,
        ToggleFold,
        ToggleSortByName,
        Minimize,
        Zoom,
        NextTab,
        PreviousTab,
        Tab1,
        Tab2,
        Tab3,
        Tab4,
        Tab5,
        Tab6,
        Tab7,
        Tab8,
        Tab9,
        /// Closes the open dialog.
        Cancel,
    ]
);

/// The key context of the window while a dialog is open, so Esc reaches the terminal otherwise.
pub const DIALOG: &str = "PaddockDialog";

/// The shortcuts. Everything else typed goes to the terminal.
pub fn bindings() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-t", NewTab, None),
        KeyBinding::new("cmd-n", NewShell, None),
        KeyBinding::new("cmd-d", SplitRight, None),
        KeyBinding::new("cmd-shift-d", SplitDown, None),
        KeyBinding::new("cmd-w", ClosePane, None),
        KeyBinding::new("cmd-shift-w", CloseTab, None),
        KeyBinding::new("cmd-c", Copy, None),
        KeyBinding::new("cmd-v", Paste, None),
        KeyBinding::new("cmd-m", Minimize, None),
        KeyBinding::new("cmd-shift-]", NextTab, None),
        KeyBinding::new("cmd-shift-[", PreviousTab, None),
        KeyBinding::new("cmd-1", Tab1, None),
        KeyBinding::new("cmd-2", Tab2, None),
        KeyBinding::new("cmd-3", Tab3, None),
        KeyBinding::new("cmd-4", Tab4, None),
        KeyBinding::new("cmd-5", Tab5, None),
        KeyBinding::new("cmd-6", Tab6, None),
        KeyBinding::new("cmd-7", Tab7, None),
        KeyBinding::new("cmd-8", Tab8, None),
        KeyBinding::new("cmd-9", Tab9, None),
        KeyBinding::new("escape", Cancel, Some(DIALOG)),
    ]
}

/// The menu bar; `fold` and `by_name` tick the View items as the sidebar has them.
pub fn menus(fold: bool, by_name: bool) -> Vec<Menu> {
    vec![
        Menu::new("paddock").items([
            MenuItem::action("About paddock", About),
            MenuItem::separator(),
            MenuItem::os_submenu("Services", SystemMenuType::Services),
            MenuItem::separator(),
            MenuItem::action("Quit paddock", Quit),
        ]),
        Menu::new("Shell").items([
            MenuItem::action("New Tab…", NewTab),
            MenuItem::action("New Shell", NewShell),
            MenuItem::separator(),
            MenuItem::action("Split Right…", SplitRight),
            MenuItem::action("Split Down…", SplitDown),
            MenuItem::action("Split Left…", SplitLeft),
            MenuItem::action("Split Up…", SplitUp),
            MenuItem::separator(),
            MenuItem::action("Close Pane", ClosePane),
            MenuItem::action("Close Tab", CloseTab),
        ]),
        Menu::new("Edit").items([
            MenuItem::action("Copy", Copy),
            MenuItem::action("Paste", Paste),
        ]),
        Menu::new("View").items([
            MenuItem::action("Fold Agents", ToggleFold).checked(fold),
            MenuItem::action("Sort Agents by Name", ToggleSortByName).checked(by_name),
        ]),
        Menu::new("Window").items([
            MenuItem::action("Minimize", Minimize),
            MenuItem::action("Zoom", Zoom),
            MenuItem::separator(),
            MenuItem::action("Next Tab", NextTab),
            MenuItem::action("Previous Tab", PreviousTab),
            MenuItem::separator(),
            MenuItem::action("Tab 1", Tab1),
            MenuItem::action("Tab 2", Tab2),
            MenuItem::action("Tab 3", Tab3),
            MenuItem::action("Tab 4", Tab4),
            MenuItem::action("Tab 5", Tab5),
            MenuItem::action("Tab 6", Tab6),
            MenuItem::action("Tab 7", Tab7),
            MenuItem::action("Tab 8", Tab8),
            MenuItem::action("Tab 9", Tab9),
        ]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each shortcut as typed, and the action it runs.
    fn shortcuts() -> Vec<(String, &'static str)> {
        bindings()
            .iter()
            .map(|binding| {
                let keys: Vec<String> = binding
                    .keystrokes()
                    .iter()
                    .map(|k| k.inner().unparse())
                    .collect();
                (keys.join(" "), binding.action().name())
            })
            .collect()
    }

    #[test]
    fn shortcuts_follow_the_macos_conventions_agreed() {
        let shortcuts = shortcuts();
        for (keys, action) in [
            ("cmd-q", "paddock::Quit"),
            ("cmd-t", "paddock::NewTab"),
            ("cmd-n", "paddock::NewShell"),
            ("cmd-d", "paddock::SplitRight"),
            ("cmd-shift-d", "paddock::SplitDown"),
            ("cmd-w", "paddock::ClosePane"),
            ("cmd-shift-w", "paddock::CloseTab"),
            ("cmd-c", "paddock::Copy"),
            ("cmd-v", "paddock::Paste"),
            ("cmd-m", "paddock::Minimize"),
            ("cmd-shift-]", "paddock::NextTab"),
            ("cmd-shift-[", "paddock::PreviousTab"),
            ("cmd-1", "paddock::Tab1"),
            ("cmd-9", "paddock::Tab9"),
            ("escape", "paddock::Cancel"),
        ] {
            assert!(
                shortcuts.iter().any(|(k, a)| k == keys && *a == action),
                "{keys} → {action} missing from {shortcuts:?}"
            );
        }
        // One action per shortcut.
        let mut keys: Vec<_> = shortcuts.iter().map(|(k, _)| k.clone()).collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), shortcuts.len());
    }

    #[test]
    fn the_menu_bar_has_the_agreed_menus_and_ticks_the_view_items() {
        let names: Vec<String> = menus(false, false)
            .iter()
            .map(|m| m.name.to_string())
            .collect();
        assert_eq!(names, ["paddock", "Shell", "Edit", "View", "Window"]);
        let view = |fold, by_name| -> Vec<bool> {
            menus(fold, by_name)[3]
                .items
                .iter()
                .map(|item| matches!(item, MenuItem::Action { checked: true, .. }))
                .collect()
        };
        assert_eq!(view(true, false), [true, false]);
        assert_eq!(view(false, true), [false, true]);
    }
}
