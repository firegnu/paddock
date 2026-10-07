//! The menu bar, its actions and their keyboard shortcuts, in the usual macOS places.
use gpui::{Action, KeyBinding, Menu, MenuItem, OsAction, SystemMenuType, actions};

actions!(
    paddock,
    [
        About,
        OpenSettings,
        SaveSettings,
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
        /// Collapse the sidebar to a narrow strip, or expand it again.
        ToggleSidebar,
        /// Open or close the right sidebar.
        ToggleRightSidebar,
        Minimize,
        Zoom,
        /// Fill the tab with the active pane, or back to the split.
        ZoomPane,
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
        /// Closes the Settings or About window (⌘W there).
        CloseWindow,
        Find,
        FindNext,
        FindPrevious,
        /// Closes the find bar.
        CloseFind,
        NewAgent,
        StopAgent,
        /// Create in the New Agent window.
        CreateAgent,
        /// Open or close the Attention list.
        ShowAttention,
        /// Open or close the command palette: agents, tabs, Settings pages and commands by typing.
        Search,
        /// Open or close the command palette at the commands (`>`).
        CommandPalette,
        /// Move in the Attention list or the command palette, and open the selected entry.
        SelectNext,
        SelectPrevious,
        OpenSelected,
        /// The Browser's address field takes the keyboard, all of it selected (⌘L).
        FocusAddress,
        /// The Browser loads its page again (⌘R).
        ReloadPage,
    ]
);

/// The key context of a pane's find bar: ⏎ and ⇧⏎ go on searching, Esc closes it.
pub const FIND: &str = "PaddockFind";

/// The key context of the window while a dialog is open, so Esc reaches the terminal otherwise.
pub const DIALOG: &str = "PaddockDialog";

/// The key context of the command palette: Tab moves in its list rather than out of it.
pub const PALETTE: &str = "PaddockPalette";

/// The key context of the right sidebar's Browser, while its page or address field has the
/// keyboard: ⌘L and ⌘R are its own there, and the pane's elsewhere.
pub const BROWSER: &str = "PaddockBrowser";

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
        KeyBinding::new("cmd-,", OpenSettings, None),
        KeyBinding::new("cmd-s", SaveSettings, Some(crate::settings_view::CONTEXT)),
        KeyBinding::new("cmd-w", CloseWindow, Some(crate::settings_view::CONTEXT)),
        KeyBinding::new("cmd-w", CloseWindow, Some(crate::about::CONTEXT)),
        KeyBinding::new("cmd-shift-n", NewAgent, None),
        KeyBinding::new("cmd-shift-a", ShowAttention, None),
        KeyBinding::new("cmd-b", ToggleSidebar, None),
        KeyBinding::new("alt-cmd-b", ToggleRightSidebar, None),
        KeyBinding::new("cmd-p", Search, None),
        KeyBinding::new("cmd-shift-p", CommandPalette, None),
        KeyBinding::new("cmd-shift-enter", ZoomPane, None),
        KeyBinding::new("cmd-f", Find, None),
        KeyBinding::new("cmd-g", FindNext, None),
        KeyBinding::new("cmd-shift-g", FindPrevious, None),
        KeyBinding::new("enter", FindNext, Some(FIND)),
        KeyBinding::new("shift-enter", FindPrevious, Some(FIND)),
        KeyBinding::new("escape", CloseFind, Some(FIND)),
        KeyBinding::new("down", SelectNext, Some(DIALOG)),
        KeyBinding::new("up", SelectPrevious, Some(DIALOG)),
        KeyBinding::new("enter", OpenSelected, Some(DIALOG)),
        KeyBinding::new("tab", SelectNext, Some(PALETTE)),
        KeyBinding::new("shift-tab", SelectPrevious, Some(PALETTE)),
        KeyBinding::new(
            "cmd-enter",
            CreateAgent,
            Some(crate::new_agent_view::CONTEXT),
        ),
        KeyBinding::new("cmd-w", CloseWindow, Some(crate::new_agent_view::CONTEXT)),
        KeyBinding::new("escape", Cancel, Some(crate::new_agent_view::CONTEXT)),
        KeyBinding::new("enter", OpenSelected, Some(crate::new_agent_view::FIELD)),
        KeyBinding::new("cmd-l", FocusAddress, Some(BROWSER)),
        KeyBinding::new("cmd-r", ReloadPage, Some(BROWSER)),
    ]
}

/// The menu commands that act on what has the keyboard rather than on the window: the pane
/// copies, pastes and finds in itself, the Browser's page copies and pastes in itself (and finds
/// in its find bar, see [`finds`]).
pub fn follows_keyboard(action: &dyn Action) -> bool {
    [
        &Copy as &dyn Action,
        &Paste,
        &Find,
        &FindNext,
        &FindPrevious,
    ]
    .into_iter()
    .any(|own| action.partial_eq(own))
}

/// The Browser's own commands, ⌘L and ⌘R.
pub fn browser(action: &dyn Action) -> bool {
    action.partial_eq(&FocusAddress) || action.partial_eq(&ReloadPage)
}

/// Finding, ⌘F, ⌘G and ⇧⌘G: in the Browser's own find bar while its page or one of its fields has
/// the keyboard, in the pane otherwise.
pub fn finds(action: &dyn Action) -> bool {
    [&Find as &dyn Action, &FindNext, &FindPrevious]
        .into_iter()
        .any(|find| action.partial_eq(find))
}

/// The menu bar; `fold` and `by_name` tick the View items as the sidebar has them, and
/// `collapsed` and `right_open` say which way the two sidebar items go.
pub fn menus(fold: bool, by_name: bool, collapsed: bool, right_open: bool) -> Vec<Menu> {
    vec![
        Menu::new("paddock").items([
            MenuItem::action("About paddock", About),
            MenuItem::separator(),
            MenuItem::action("Settings…", OpenSettings),
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
        // The system's own copy and paste: whatever has the keyboard takes them, the Browser's
        // page included, and paddock's views through GPUI.
        Menu::new("Edit").items([
            MenuItem::os_action("Copy", Copy, OsAction::Copy),
            MenuItem::os_action("Paste", Paste, OsAction::Paste),
            MenuItem::separator(),
            MenuItem::action("Find…", Find),
            MenuItem::action("Find Next", FindNext),
            MenuItem::action("Find Previous", FindPrevious),
        ]),
        Menu::new("View").items([
            MenuItem::action("Fold Agents", ToggleFold).checked(fold),
            MenuItem::action("Sort Agents by Name", ToggleSortByName).checked(by_name),
            MenuItem::action(
                if collapsed {
                    "Expand Sidebar"
                } else {
                    "Collapse Sidebar"
                },
                ToggleSidebar,
            ),
            MenuItem::action(
                if right_open {
                    "Hide Right Sidebar"
                } else {
                    "Show Right Sidebar"
                },
                ToggleRightSidebar,
            ),
            MenuItem::separator(),
            MenuItem::action("Zoom Pane", ZoomPane),
        ]),
        Menu::new("Agent").items([
            MenuItem::action("New Agent…", NewAgent),
            MenuItem::action("Stop Agent…", StopAgent),
            MenuItem::separator(),
            MenuItem::action("Search…", Search),
            MenuItem::action("Command Palette…", CommandPalette),
            MenuItem::action("Attention…", ShowAttention),
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

/// A menu bar command as the command palette lists it.
pub struct Command {
    pub title: String,
    /// What the menu item runs.
    pub action: Box<dyn Action>,
    /// Its shortcut as keycaps; none without one.
    pub keys: Vec<String>,
}

/// Every command in the menu bar, the app menu's last, but the two that open the palette. Out of
/// their menu, `Find…`, `Attention…` and `Zoom` say what they act on, and the sidebar items say
/// they go either way.
pub fn commands() -> Vec<Command> {
    let mut menus = menus(false, false, false, false);
    menus.rotate_left(1);
    menus
        .into_iter()
        .flat_map(|menu| menu.items)
        .filter_map(|item| match item {
            MenuItem::Action { name, action, .. } => Some((name, action)),
            _ => None,
        })
        .filter(|(_, action)| !action.partial_eq(&Search) && !action.partial_eq(&CommandPalette))
        .map(|(name, action)| {
            let title = match name.as_ref() {
                "Find…" => "Find in Terminal",
                "Attention…" => "Show Attention",
                "Zoom" => "Zoom Window",
                "Collapse Sidebar" => "Toggle Sidebar",
                "Show Right Sidebar" => "Toggle Right Sidebar",
                name => name,
            };
            Command {
                title: title.to_owned(),
                keys: keys(action.as_ref()),
                action,
            }
        })
        .collect()
}

/// The keys of `action`'s shortcut outside dialogs and fields, as keycaps: `⌘ ⇧ D`.
pub fn keys(action: &dyn Action) -> Vec<String> {
    let Some(binding) = bindings()
        .into_iter()
        .find(|binding| binding.predicate().is_none() && binding.action().partial_eq(action))
    else {
        return Vec::new();
    };
    let mut caps = Vec::new();
    for keystroke in binding.keystrokes() {
        let keystroke = keystroke.inner();
        let held = &keystroke.modifiers;
        for (down, cap) in [
            (held.control, "⌃"),
            (held.alt, "⌥"),
            (held.platform, "⌘"),
            (held.shift, "⇧"),
        ] {
            if down {
                caps.push(cap.to_owned());
            }
        }
        caps.push(match keystroke.key.as_str() {
            "enter" => "↵".to_owned(),
            "escape" => "esc".to_owned(),
            "up" => "↑".to_owned(),
            "down" => "↓".to_owned(),
            "left" => "←".to_owned(),
            "right" => "→".to_owned(),
            "tab" => "⇥".to_owned(),
            "space" => "Space".to_owned(),
            "backspace" => "⌫".to_owned(),
            key => key.to_uppercase(),
        });
    }
    caps
}

/// What ⌘`n` runs, for n from 1 to 9.
pub fn tab(n: usize) -> Option<Box<dyn Action>> {
    let tabs: [Box<dyn Action>; 9] = [
        Box::new(Tab1),
        Box::new(Tab2),
        Box::new(Tab3),
        Box::new(Tab4),
        Box::new(Tab5),
        Box::new(Tab6),
        Box::new(Tab7),
        Box::new(Tab8),
        Box::new(Tab9),
    ];
    tabs.into_iter().nth(n.checked_sub(1)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each shortcut as typed, the action it runs, and the key context it is bound in.
    fn shortcuts() -> Vec<(String, &'static str, String)> {
        bindings()
            .iter()
            .map(|binding| {
                let keys: Vec<String> = binding
                    .keystrokes()
                    .iter()
                    .map(|k| k.inner().unparse())
                    .collect();
                let context = binding
                    .predicate()
                    .map(|p| p.to_string())
                    .unwrap_or_default();
                (keys.join(" "), binding.action().name(), context)
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
            ("cmd-,", "paddock::OpenSettings"),
            ("cmd-s", "paddock::SaveSettings"),
            ("cmd-w", "paddock::CloseWindow"),
            ("cmd-shift-n", "paddock::NewAgent"),
            ("cmd-enter", "paddock::CreateAgent"),
            ("cmd-shift-a", "paddock::ShowAttention"),
            ("cmd-b", "paddock::ToggleSidebar"),
            ("alt-cmd-b", "paddock::ToggleRightSidebar"),
            ("cmd-p", "paddock::Search"),
            ("cmd-shift-p", "paddock::CommandPalette"),
            ("cmd-shift-enter", "paddock::ZoomPane"),
            ("cmd-f", "paddock::Find"),
            ("cmd-g", "paddock::FindNext"),
            ("cmd-shift-g", "paddock::FindPrevious"),
            ("cmd-l", "paddock::FocusAddress"),
            ("cmd-r", "paddock::ReloadPage"),
        ] {
            assert!(
                shortcuts.iter().any(|(k, a, _)| k == keys && *a == action),
                "{keys} → {action} missing from {shortcuts:?}"
            );
        }
        // One action per shortcut in each key context.
        let mut keys: Vec<_> = shortcuts
            .iter()
            .map(|(k, _, context)| (k.clone(), context.clone()))
            .collect();
        let others = keys.len();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), others);
    }

    #[test]
    fn the_menu_bar_has_the_agreed_menus_and_ticks_the_view_items() {
        let names: Vec<String> = menus(false, false, false, false)
            .iter()
            .map(|m| m.name.to_string())
            .collect();
        assert_eq!(
            names,
            ["paddock", "Shell", "Edit", "View", "Agent", "Window"]
        );
        let view = |fold, by_name| -> Vec<bool> {
            menus(fold, by_name, false, false)[3]
                .items
                .iter()
                .map(|item| matches!(item, MenuItem::Action { checked: true, .. }))
                .collect()
        };
        // Fold, Sort, the two sidebars, a separator and Zoom Pane; the last four are never ticked.
        assert_eq!(view(true, false), [true, false, false, false, false, false]);
        assert_eq!(view(false, true), [false, true, false, false, false, false]);
        // The sidebar items say what they will do; the right one comes after the left.
        let item =
            |collapsed, right_open, index: usize| match &menus(false, false, collapsed, right_open)
                [3]
            .items[index]
            {
                MenuItem::Action { name, .. } => name.to_string(),
                _ => String::new(),
            };
        assert_eq!(item(false, false, 2), "Collapse Sidebar");
        assert_eq!(item(true, false, 2), "Expand Sidebar");
        assert_eq!(item(false, false, 3), "Show Right Sidebar");
        assert_eq!(item(false, true, 3), "Hide Right Sidebar");
        // Copy and Paste are the system's, so the Browser's page takes them when it has the
        // keyboard; Find is paddock's own.
        let edit: Vec<Option<OsAction>> = menus(false, false, false, false)[2]
            .items
            .iter()
            .filter_map(|item| match item {
                MenuItem::Action { os_action, .. } => Some(*os_action),
                _ => None,
            })
            .collect();
        // (`OsAction` has no `Debug`.)
        assert!(
            edit == [
                Some(OsAction::Copy),
                Some(OsAction::Paste),
                None,
                None,
                None
            ]
        );
    }

    #[test]
    fn only_the_panes_own_commands_follow_the_keyboard_and_the_browser_binds_l_and_r_alone() {
        for action in [
            &Copy as &dyn Action,
            &Paste,
            &Find,
            &FindNext,
            &FindPrevious,
        ] {
            assert!(follows_keyboard(action), "{}", action.name());
        }
        for action in [
            &ClosePane as &dyn Action,
            &NewTab,
            &ToggleRightSidebar,
            &Search,
            &ToggleSidebar,
            &CommandPalette,
            &SplitRight,
            &Quit,
            &FocusAddress,
        ] {
            assert!(!follows_keyboard(action), "{}", action.name());
        }
        assert!(browser(&FocusAddress) && browser(&ReloadPage) && !browser(&Copy));
        // ⌘L and ⌘R only where the Browser has the keyboard; the pane keeps them otherwise.
        for binding in bindings().iter().filter(|b| browser(b.action())) {
            let context = binding.predicate().map(|p| p.to_string());
            assert_eq!(context.as_deref(), Some(BROWSER));
        }
    }

    #[test]
    fn the_palette_lists_every_menu_command_with_the_shortcut_it_has() {
        let commands = commands();
        let titles: Vec<&str> = commands.iter().map(|c| c.title.as_str()).collect();
        let in_menus = menus(false, false, false, false)
            .iter()
            .flat_map(|menu| &menu.items)
            .filter(|item| matches!(item, MenuItem::Action { .. }))
            .count();
        // All but Search… and Command Palette…, which open the palette itself.
        assert_eq!(commands.len(), in_menus - 2);
        assert!(!titles.contains(&"Search…") && !titles.contains(&"Command Palette…"));
        // The app menu comes last.
        assert_eq!(titles.first(), Some(&"New Tab…"));
        assert_eq!(titles.last(), Some(&"Quit paddock"));
        for title in [
            "New Agent…",
            "New Shell",
            "Split Right…",
            "Split Down…",
            "Split Left…",
            "Split Up…",
            "Close Pane",
            "Close Tab",
            "Zoom Pane",
            "Stop Agent…",
            "Show Attention",
            "Find in Terminal",
            "Settings…",
            "Fold Agents",
            "Sort Agents by Name",
            "Toggle Sidebar",
            "Toggle Right Sidebar",
            "Next Tab",
            "Previous Tab",
            "About paddock",
            "Zoom Window",
        ] {
            assert!(titles.contains(&title), "{title} missing from {titles:?}");
        }
        let keys_of = |title: &str| -> Vec<String> {
            commands
                .iter()
                .find(|c| c.title == title)
                .unwrap()
                .keys
                .clone()
        };
        assert_eq!(keys_of("New Agent…"), ["⌘", "⇧", "N"]);
        assert_eq!(keys_of("Split Right…"), ["⌘", "D"]);
        assert_eq!(keys_of("Split Down…"), ["⌘", "⇧", "D"]);
        assert_eq!(keys_of("Zoom Pane"), ["⌘", "⇧", "↵"]);
        assert_eq!(keys_of("Show Attention"), ["⌘", "⇧", "A"]);
        assert_eq!(keys_of("Find in Terminal"), ["⌘", "F"]);
        assert_eq!(keys_of("Settings…"), ["⌘", ","]);
        assert_eq!(keys_of("Toggle Sidebar"), ["⌘", "B"]);
        assert_eq!(keys_of("Toggle Right Sidebar"), ["⌥", "⌘", "B"]);
        assert_eq!(keys_of("Next Tab"), ["⌘", "⇧", "]"]);
        assert!(keys_of("Stop Agent…").is_empty());
        assert!(keys_of("Split Left…").is_empty());
        // Each command runs the menu item's action, and every shortcut outside dialogs that runs a
        // menu command shows on it.
        for binding in bindings().iter().filter(|b| b.predicate().is_none()) {
            if let Some(command) = commands
                .iter()
                .find(|c| c.action.partial_eq(binding.action()))
            {
                assert!(!command.keys.is_empty(), "{} shows no keys", command.title);
            }
        }
        assert_eq!(keys(&Search), ["⌘", "P"]);
        assert_eq!(keys(&CommandPalette), ["⌘", "⇧", "P"]);
    }

    #[test]
    fn tab_shortcuts_go_from_one_to_nine() {
        assert_eq!(keys(tab(1).unwrap().as_ref()), ["⌘", "1"]);
        assert_eq!(keys(tab(9).unwrap().as_ref()), ["⌘", "9"]);
        assert!(tab(0).is_none());
        assert!(tab(10).is_none());
    }
}
