//! Settings › Sandbox (DESIGN §13 P5-120): fence's profiles and the rules one comes to, view
//! only. fence's version on top; the profiles, each with where it comes from, its description as
//! fence words it, whether it replaces the built-in one and why it can't be read; this Mac's
//! additions file and whether it is there. A profile, a kind fence has data for and a folder
//! chosen, `fence check` shows the layers it merged, the domains allowed, what can and can't be
//! read and written, what it skipped and its warnings, word for word. Nothing here changes a
//! profile: fence has no command for that, and paddock does not write fence's files.
use super::*;
use crate::fence::{self, Check, Profiles, Refusal, Version};
use gpui::PathPromptOptions;
use std::sync::atomic::Ordering;

/// What a check was asked for: the profile, the kind and the folder.
type Asked = (String, String, String);

/// The page's state in the Settings window.
#[derive(Default)]
pub(super) struct Sandbox {
    /// What was found: none while looking.
    found: Option<Found>,
    /// The profile, kind and folder the rules are shown for.
    profile: Option<String>,
    kind: Option<String>,
    folder: Option<String>,
    /// The latest check, for what it was asked (profile, kind, folder); its answer once in.
    check: Option<(Asked, Option<Result<Check, Refusal>>)>,
    /// Counts the checks asked for: only the latest one's answer is taken.
    asked: u64,
    /// Stops the look for fence, and the check, still running once they are not wanted.
    finding: Option<fence::Cancel>,
    checking: Option<fence::Cancel>,
    /// The folders to choose from: the recently used projects.
    folders: Vec<String>,
    /// The folder list is open.
    choosing: bool,
    /// The lists shown whole, by their heading.
    whole: Vec<&'static str>,
}

/// fence as found: not there (or another contract), or its version and profiles.
enum Found {
    Missing,
    Here(Version, Result<Profiles, Refusal>),
}

/// Lists longer than this show their first ones and `Show all`.
const SHORT: usize = 5;

/// fence's word for a path made only when the agent starts (`${tmp}`), and what is shown for it.
const AT_START: (&str, &str) = ("<运行时才有>", "temp dir (made at start)");

const VIEW_ONLY: &str = "View only. Profiles are changed in fence, not here.";

/// `path` with home written `~`, and fence's word for a path made at start in English.
fn shown(path: &str) -> String {
    if path == AT_START.0 {
        return AT_START.1.to_owned();
    }
    shown_path(std::path::Path::new(path))
}

impl SettingsView {
    /// Looks for fence and reads its profiles again, off the UI thread, then checks the rules for
    /// what is chosen: the first profile, Claude (or the first kind) and the latest project, at
    /// first.
    pub(super) fn refresh_sandbox(&mut self, cx: &mut Context<Self>) {
        self.sandbox.folders = crate::new_agent::recent_path()
            .as_deref()
            .map(crate::new_agent::load_recent)
            .unwrap_or_default();
        // A look still running from before is not wanted.
        let finding = fence::Cancel::default();
        let flag = finding.flag();
        let stopped = flag.clone();
        self.sandbox.finding = Some(finding);
        let task = cx.background_spawn(async move {
            match fence::version(fence::PROGRAM, &flag) {
                None => Found::Missing,
                Some(version) => Found::Here(version, fence::profiles(fence::PROGRAM, &flag)),
            }
        });
        cx.spawn(async move |this, cx| {
            let found = task.await;
            let _ = this.update(cx, |this, cx| {
                // Stopped for a newer look: its answer is not fence's.
                if stopped.load(Ordering::Relaxed) {
                    return;
                }
                let state = &mut this.sandbox;
                if let Found::Here(_, Ok(profiles)) = &found {
                    let profile_gone = !state.profile.as_ref().is_some_and(|p| profiles.has(p));
                    if profile_gone {
                        state.profile = profiles.profiles.first().map(|p| p.name.clone());
                    }
                    let kind_gone = !state.kind.as_ref().is_some_and(|k| profiles.supports(k));
                    if kind_gone {
                        state.kind = profiles
                            .kinds
                            .iter()
                            .find(|k| k.kind == "claude")
                            .or(profiles.kinds.first())
                            .map(|k| k.kind.clone());
                    }
                }
                if state.folder.is_none() {
                    state.folder = state
                        .folders
                        .first()
                        .cloned()
                        .or_else(|| std::env::var("HOME").ok());
                }
                state.found = Some(found);
                // The profiles may have changed: what was checked is checked again.
                state.check = None;
                this.check_sandbox(cx);
                cx.notify();
            });
        })
        .detach();
    }

    /// Another page: the look for fence and the check still running stop; showing the page again
    /// looks and checks afresh.
    pub(super) fn leave_sandbox(&mut self) {
        self.sandbox.finding = None;
        self.sandbox.checking = None;
        self.sandbox.check = None;
    }

    /// Checks what is chosen, unless that is what was last checked.
    fn check_sandbox(&mut self, cx: &mut Context<Self>) {
        let state = &mut self.sandbox;
        let (Some(profile), Some(kind), Some(folder)) = (
            state.profile.clone(),
            state.kind.clone(),
            state.folder.clone(),
        ) else {
            return;
        };
        let asked = (profile, kind, folder);
        if state.check.as_ref().is_some_and(|(was, _)| *was == asked) {
            return;
        }
        state.asked += 1;
        let number = state.asked;
        state.check = Some((asked.clone(), None));
        // The check before this one is not wanted: it stops.
        let checking = fence::Cancel::default();
        let flag = checking.flag();
        let stopped = flag.clone();
        state.checking = Some(checking);
        let task = cx.background_spawn(async move {
            let (profile, kind, folder) = asked;
            fence::check(fence::PROGRAM, &profile, Some(&kind), Some(&folder), &flag)
        });
        cx.spawn(async move |this, cx| {
            let answer = task.await;
            let _ = this.update(cx, |this, cx| {
                let state = &mut this.sandbox;
                if state.asked == number
                    && !stopped.load(Ordering::Relaxed)
                    && let Some((_, slot)) = &mut state.check
                {
                    *slot = Some(answer);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn choose_sandbox(&mut self, cx: &mut Context<Self>, choose: impl FnOnce(&mut Sandbox)) {
        choose(&mut self.sandbox);
        self.sandbox.choosing = false;
        self.check_sandbox(cx);
        cx.notify();
    }

    /// The system's folder chooser, for the folder the rules are shown for.
    fn browse_sandbox(&mut self, cx: &mut Context<Self>) {
        self.sandbox.choosing = false;
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Choose".into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = paths.await
                && let Some(path) = paths.into_iter().next()
            {
                let _ = this.update(cx, |this, cx| {
                    let folder = path.display().to_string();
                    this.choose_sandbox(cx, |state| state.folder = Some(folder));
                });
            }
        })
        .detach();
        cx.notify();
    }

    /// The page.
    pub(super) fn sandbox_page(
        &self,
        width: f32,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let (dim, dimmer, green, red) = (
            self.fg(|t| t.agents_dim),
            self.fg(|t| t.agents_dimmer),
            self.fg(|t| t.agents_green),
            self.fg(|t| t.agents_red),
        );
        let quiet = |text: String| {
            div()
                .px(px(4.0))
                .text_size(ui.px(12.5))
                .text_color(dim)
                .child(text)
                .into_any_element()
        };
        let (version, profiles) = match &self.sandbox.found {
            None => return vec![quiet("Looking for fence…".into())],
            Some(Found::Missing) => {
                return vec![quiet(
                    "fence isn't installed, or isn't a version paddock reads (contract 1), so \
                     New Agent offers no sandbox."
                        .into(),
                )];
            }
            Some(Found::Here(version, profiles)) => (version, profiles),
        };
        let mut about = vec![format!("fence {}", version.version)];
        about.extend(version.srt.as_ref().map(|srt| format!("srt {srt}")));
        about.extend(version.node.as_ref().map(|node| format!("node {node}")));
        let status = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(10.0))
            .child(
                div()
                    .flex_shrink_0()
                    .h(ui.px(22.0))
                    .px(px(9.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .rounded_full()
                    .bg(green.opacity(0.12))
                    .text_size(ui.px(12.0))
                    .text_color(green)
                    .child(div().size(px(6.0)).rounded_full().bg(green))
                    .child(format!("{} ready", about.remove(0))),
            )
            .child(
                div()
                    .text_size(ui.px(12.0))
                    .text_color(dimmer)
                    .child(about.join(" · ")),
            )
            .child(div().flex_1())
            .child(
                div()
                    .text_size(ui.px(12.0))
                    .text_color(dimmer)
                    .child(VIEW_ONLY),
            );
        let profiles = match profiles {
            Ok(profiles) => profiles,
            Err(refusal) => {
                return vec![
                    status.into_any_element(),
                    div()
                        .px(px(4.0))
                        .text_size(ui.px(12.5))
                        .text_color(red)
                        .child(format!("Couldn't read the profiles: {refusal}"))
                        .into_any_element(),
                ];
            }
        };
        let list = self.sandbox_profiles(profiles, ui, cx);
        let rules = self.sandbox_rules(profiles, width, ui, cx);
        let body = if width >= ui.scale(620.0) {
            div()
                .flex()
                .items_start()
                .gap(ui.px(22.0))
                .child(div().w(ui.px(250.0)).flex_shrink_0().child(list))
                .child(div().flex_1().min_w(px(0.0)).child(rules))
        } else {
            div()
                .flex()
                .flex_col()
                .gap(ui.px(22.0))
                .child(list)
                .child(rules)
        };
        vec![status.into_any_element(), body.into_any_element()]
    }

    /// The profiles, one picked for the rules beside them; this Mac's additions under them.
    fn sandbox_profiles(&self, profiles: &Profiles, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let (text, dim, dimmer, accent, sandbox, red) = (
            self.fg(|t| t.agents_text),
            self.fg(|t| t.agents_dim),
            self.fg(|t| t.agents_dimmer),
            self.fg(|t| t.agents_accent),
            self.fg(|t| t.agents_sandbox),
            self.fg(|t| t.agents_red),
        );
        let hover = self.faint(0.05);
        let mut column = div()
            .flex()
            .flex_col()
            .gap(ui.px(8.0))
            .child(self.heading("Profiles", ui));
        if profiles.profiles.is_empty() {
            column = column.child(
                div()
                    .px(px(4.0))
                    .text_size(ui.px(12.5))
                    .text_color(dim)
                    .child("fence lists no profiles."),
            );
        }
        for (index, profile) in profiles.profiles.iter().enumerate() {
            let on = self.sandbox.profile.as_deref() == Some(profile.name.as_str());
            let name = profile.name.clone();
            let (stroke, fill) = if on {
                (sandbox, Some(sandbox.opacity(0.22)))
            } else {
                (dim, None)
            };
            let mut title = div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(7.0))
                .child(footer_icon::lock(false, stroke, fill, ui.scale(13.0)))
                .child(
                    div()
                        .text_size(ui.px(13.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(text)
                        .child(profile.name.clone()),
                )
                .child(
                    div()
                        .text_size(ui.px(11.0))
                        .text_color(dimmer)
                        .child(profile.origin()),
                );
            if profile.overrides_builtin {
                title = title.child(
                    div()
                        .text_size(ui.px(11.0))
                        .text_color(dimmer)
                        .child("replaces the built-in one"),
                );
            }
            let line = |words: String, color: Hsla| {
                div()
                    .text_size(ui.px(12.0))
                    .line_height(relative(1.55))
                    .text_color(color)
                    .child(words)
            };
            let card = div()
                .id(ElementId::NamedInteger(
                    "sandbox-profile".into(),
                    index as u64,
                ))
                .px(px(12.0))
                .py(px(11.0))
                .flex()
                .flex_col()
                .gap(px(5.0))
                .rounded(px(10.0))
                .border_1()
                .cursor_pointer()
                .child(title)
                .children(
                    profile
                        .description
                        .clone()
                        .filter(|d| !d.is_empty())
                        .map(|d| line(d, dim)),
                )
                .children(
                    profile
                        .path
                        .as_deref()
                        .map(|path| line(shown(path), dimmer)),
                )
                .children(
                    profile
                        .problem()
                        .map(|problem| line(format!("Can't be read: {problem}"), red)),
                )
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    let name = name.clone();
                    this.choose_sandbox(cx, |state| state.profile = Some(name));
                }));
            column = column.child(if on {
                card.border_color(accent.opacity(0.4))
                    .bg(accent.opacity(0.08))
            } else {
                card.border_color(gpui::transparent_black())
                    .bg(self.card_ground())
                    .hover(move |style| style.bg(hover))
            });
        }
        if let Some(local) = &profiles.local {
            column = column.child(
                div()
                    .mt(px(6.0))
                    .px(px(4.0))
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .text_size(ui.px(12.0))
                    .text_color(dimmer)
                    .child("This Mac's additions")
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .font_family(MONO)
                                    .text_color(dim)
                                    .child(shown(&local.path)),
                            )
                            .child(if local.exists {
                                "· there"
                            } else {
                                "· not created"
                            }),
                    ),
            );
        }
        if let Some(servers) = &profiles.mcp {
            let mut search = div()
                .mt(px(6.0))
                .px(px(4.0))
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(6.0))
                .text_size(ui.px(12.0))
                .text_color(dimmer)
                .child("Web search:");
            if servers.is_empty() {
                search = search.child(div().text_color(dim).child("none"));
            }
            for (index, server) in servers.iter().enumerate() {
                if index > 0 {
                    search = search.child("·");
                }
                search = search.child(match server.problem() {
                    Some(problem) => div()
                        .text_color(red)
                        .child(format!("{} (can't be read: {problem})", server.name)),
                    None => div().text_color(dim).child(format!(
                        "{} ({})",
                        server.name,
                        server.origin()
                    )),
                });
            }
            column = column.child(search);
        }
        column
    }

    /// What the chosen profile comes to for the kind and folder chosen.
    fn sandbox_rules(
        &self,
        profiles: &Profiles,
        width: f32,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Div {
        let (text, dim, dimmer, red, yellow, accent) = (
            self.fg(|t| t.agents_text),
            self.fg(|t| t.agents_dim),
            self.fg(|t| t.agents_dimmer),
            self.fg(|t| t.agents_red),
            self.fg(|t| t.agents_yellow),
            self.fg(|t| t.agents_accent),
        );
        let state = &self.sandbox;
        // Rules for [kinds] in [folder].
        let mut kinds = div()
            .flex()
            .p(px(2.0))
            .gap(px(2.0))
            .rounded(px(8.0))
            .bg(gpui::black().opacity(self.theme.shade(0.22)));
        for (index, kind) in profiles.kinds.iter().enumerate() {
            let on = state.kind.as_deref() == Some(kind.kind.as_str());
            let name = kind.kind.clone();
            let label = match name.as_str() {
                "claude" => "Claude".to_owned(),
                "codex" => "Codex".to_owned(),
                other => other.to_owned(),
            };
            let hover = self.faint(0.06);
            let button = div()
                .id(ElementId::NamedInteger("sandbox-kind".into(), index as u64))
                .h(ui.px(26.0))
                .px(px(10.0))
                .flex()
                .items_center()
                .rounded(px(6.0))
                .text_size(ui.px(12.5))
                .cursor_pointer()
                .child(label)
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    let name = name.clone();
                    this.choose_sandbox(cx, |state| state.kind = Some(name));
                }));
            kinds = kinds.child(if on {
                button.bg(self.lifted(0.12)).text_color(text)
            } else {
                button.text_color(dim).hover(move |style| style.bg(hover))
            });
        }
        let folder = state.folder.clone().unwrap_or_default();
        let folder_name = folder
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| shown(&folder));
        let field = self.field_ground();
        let lit = self.lifted(0.11);
        let picker = div()
            .relative()
            .child(
                div()
                    .id("sandbox-folder")
                    .h(ui.px(28.0))
                    .px(px(10.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .rounded(px(7.0))
                    .bg(if state.choosing { lit } else { field })
                    .hover(move |style| style.bg(lit))
                    .cursor_pointer()
                    .text_size(ui.px(12.5))
                    .child(folder_name)
                    .child(footer_icon::icon(
                        Icon::Down,
                        dimmer,
                        ui.scale(10.0) / footer_icon::SIZE,
                    ))
                    // Open; a press outside the list closes it.
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.sandbox.choosing = true;
                        cx.notify();
                    })),
            )
            .children(state.choosing.then(|| self.sandbox_folders(ui, cx)));
        let choose = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(10.0))
            .text_size(ui.px(12.5))
            .child(div().text_color(dim).child("Rules for"))
            .child(kinds)
            .child(div().text_color(dim).child("in"))
            .child(picker);
        let mut column = div().flex().flex_col().gap(ui.px(14.0)).child(choose);
        let answer = match &state.check {
            Some((_, Some(answer))) => answer,
            Some((_, None)) => {
                return column.child(
                    div()
                        .text_size(ui.px(12.5))
                        .text_color(dim)
                        .child("Checking…"),
                );
            }
            None => return column,
        };
        let check = match answer {
            Ok(check) => check,
            Err(refusal) => {
                let mut line = div().flex().flex_wrap().gap(px(8.0)).text_size(ui.px(12.5));
                if !refusal.code.is_empty() {
                    line = line.child(
                        div()
                            .font_family(MONO)
                            .text_color(red)
                            .child(refusal.code.clone()),
                    );
                }
                return column.child(
                    line.child(
                        div()
                            .text_color(red.opacity(0.8))
                            .child(refusal.message.clone()),
                    ),
                );
            }
        };
        // Built from: each layer as a tag.
        let tag = |words: String| {
            div()
                .h(ui.px(20.0))
                .px(px(7.0))
                .flex()
                .items_center()
                .rounded(px(5.0))
                .bg(self.faint(0.06))
                .text_color(self.fg(|t| t.agents_branch))
                .child(words)
        };
        let mut built = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(6.0))
            .text_size(ui.px(12.0))
            .text_color(dimmer)
            .child("Built from");
        for (index, layer) in check.layers.iter().enumerate() {
            if index > 0 {
                built = built.child("+");
            }
            built = built.child(tag(layer.tag()));
        }
        if !check.layers.iter().any(|layer| layer.source == "repo") {
            built = built.child("· no .fence.toml in this folder");
        }
        column = column.child(built);
        // The web search server this profile and kind give the agent.
        if profiles.mcp.is_some() {
            let servers = check.search();
            let words = if servers.is_empty() {
                "none".to_owned()
            } else {
                servers
                    .iter()
                    .map(|server| server.words())
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            column = column.child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(6.0))
                    .text_size(ui.px(12.0))
                    .text_color(dimmer)
                    .child("Web search:")
                    .child(div().text_color(dim).child(words)),
            );
        }
        // The rules, in cards two to a row when there is room.
        let rules = &check.settings;
        let card = |body: Vec<AnyElement>| {
            div()
                .flex_1()
                .min_w(px(0.0))
                .px(px(14.0))
                .py(px(12.0))
                .flex()
                .flex_col()
                .gap(px(6.0))
                .rounded(px(10.0))
                .bg(self.card_ground())
                .children(body)
                .into_any_element()
        };
        let label = |words: String| {
            div()
                .text_size(ui.px(10.5))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(dimmer)
                .child(words.to_uppercase())
                .into_any_element()
        };
        let empty = |words: &'static str| {
            div()
                .text_size(ui.px(11.5))
                .text_color(dimmer)
                .child(words)
                .into_any_element()
        };
        let paths = |key: &'static str, list: &[String], color: Hsla, cx: &mut Context<Self>| {
            let whole = self.sandbox.whole.contains(&key) || list.len() <= SHORT;
            let shown_list: Vec<String> = list
                .iter()
                .take(if whole { list.len() } else { SHORT })
                .map(|path| shown(path))
                .collect();
            let mut items = vec![
                div()
                    .font_family(MONO)
                    .text_size(ui.px(11.5))
                    .line_height(relative(1.6))
                    .text_color(color)
                    .children(shown_list.into_iter().map(|path| div().child(path)))
                    .into_any_element(),
            ];
            if !whole {
                items.push(
                    div()
                        .id(key)
                        .text_size(ui.px(12.0))
                        .text_color(accent)
                        .cursor_pointer()
                        .child(format!("Show all {}", list.len()))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.sandbox.whole.push(key);
                            cx.notify();
                        }))
                        .into_any_element(),
                );
            }
            items
        };
        let network = {
            let mut body = vec![label("Network · allowed".into())];
            if rules.network.allowed_domains.is_empty() {
                body.push(empty("No domain."));
            } else {
                let blue = self.fg(|t| t.agents_blue);
                body.push(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap(px(5.0))
                        .font_family(MONO)
                        .text_size(ui.px(11.5))
                        .children(rules.network.allowed_domains.iter().map(|domain| {
                            div()
                                .px(px(7.0))
                                .py(px(2.0))
                                .rounded(px(5.0))
                                .bg(blue.opacity(0.12))
                                .text_color(blue)
                                .child(domain.clone())
                        }))
                        .into_any_element(),
                );
            }
            if !rules.network.denied_domains.is_empty() {
                body.push(label("Network · denied".into()));
                body.extend(paths(
                    "denied-domains",
                    &rules.network.denied_domains,
                    red,
                    cx,
                ));
            }
            body.push(empty("Everything else is blocked."));
            card(body)
        };
        let fs = &rules.filesystem;
        let denied = {
            let mut body = vec![label("Can't read".into())];
            if fs.deny_read.is_empty() {
                body.push(empty("Nothing."));
            } else {
                body.extend(paths("deny-read", &fs.deny_read, red, cx));
            }
            body.push(label("Can't write".into()));
            if fs.deny_write.is_empty() {
                body.push(empty("Nothing."));
            } else {
                body.extend(paths("deny-write", &fs.deny_write, red, cx));
            }
            card(body)
        };
        let writes = {
            let mut body = vec![label(format!("Can write · {}", fs.allow_write.len()))];
            body.extend(paths(
                "allow-write",
                &fs.allow_write,
                self.fg(|t| t.agents_green),
                cx,
            ));
            card(body)
        };
        let reads = {
            let mut body = vec![label(format!("Can read · {}", fs.allow_read.len()))];
            body.extend(paths("allow-read", &fs.allow_read, text, cx));
            card(body)
        };
        let pair = |left: AnyElement, right: AnyElement| {
            if width >= ui.scale(900.0) {
                div()
                    .flex()
                    .items_start()
                    .gap(px(12.0))
                    .child(left)
                    .child(right)
                    .into_any_element()
            } else {
                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .child(left)
                    .child(right)
                    .into_any_element()
            }
        };
        column = column
            .child(pair(network, denied))
            .child(pair(writes, reads));
        // What it left out, and its warnings, word for word.
        let mut notes = div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .text_size(ui.px(12.0))
            .text_color(dimmer);
        for skipped in &check.skipped {
            notes = notes.child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(4.0))
                    .child(div().text_color(dim).child("Skipped"))
                    .child("·")
                    .child(div().font_family(MONO).child(skipped.entry.clone()))
                    .child(format!(": {}", skipped.reason)),
            );
        }
        for warning in &check.warnings {
            notes = notes.child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(8.0))
                    .text_color(yellow)
                    .child(div().mt(px(2.0)).child(footer_icon::icon(
                        Icon::Warning,
                        yellow,
                        ui.scale(12.0) / footer_icon::SIZE,
                    )))
                    .child(div().flex_1().min_w(px(0.0)).child(warning.clone())),
            );
        }
        column.child(notes)
    }

    /// The folders to show the rules for: the recently used projects, then `Other…`.
    fn sandbox_folders(&self, ui: &UiFont, cx: &mut Context<Self>) -> impl IntoElement {
        let (text, dimmer) = (self.fg(|t| t.agents_text), self.fg(|t| t.agents_dimmer));
        let hover = self.faint(0.07);
        let row = |id: ElementId, title: String, note: String| {
            div()
                .id(id)
                .px(px(10.0))
                .py(px(6.0))
                .flex()
                .flex_col()
                .rounded(px(6.0))
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .child(div().text_size(ui.px(12.5)).text_color(text).child(title))
                .children(
                    (!note.is_empty())
                        .then(|| div().text_size(ui.px(11.0)).text_color(dimmer).child(note)),
                )
        };
        let mut list = div()
            .id("sandbox-folders")
            .occlude()
            .on_mouse_down_out(cx.listener(|this, _: &MouseDownEvent, _, cx| {
                this.sandbox.choosing = false;
                cx.notify();
            }))
            .w(ui.px(300.0))
            .max_h(ui.px(320.0))
            .overflow_y_scroll()
            .p(px(6.0))
            .flex()
            .flex_col()
            .gap(px(2.0))
            .rounded(px(10.0))
            .bg(self.lifted(0.08))
            .border_1()
            .border_color(self.faint(0.1))
            .shadow_lg();
        for (index, folder) in self.sandbox.folders.iter().enumerate() {
            let name = folder
                .trim_end_matches('/')
                .rsplit('/')
                .next()
                .unwrap_or(folder)
                .to_owned();
            let chosen = folder.clone();
            list = list.child(
                row(
                    ElementId::NamedInteger("sandbox-folder-row".into(), index as u64),
                    name,
                    shown(folder),
                )
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    let folder = chosen.clone();
                    this.choose_sandbox(cx, |state| state.folder = Some(folder));
                })),
            );
        }
        list = list.child(
            row(
                "sandbox-folder-other".into(),
                "Other…".into(),
                "Choose a folder".into(),
            )
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.browse_sandbox(cx))),
        );
        deferred(
            anchored()
                .snap_to_window_with_margin(px(8.0))
                .child(div().mt(px(4.0)).child(list)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_are_shown_from_home_and_the_start_placeholder_in_words() {
        assert_eq!(shown("<运行时才有>"), "temp dir (made at start)");
        if let Ok(home) = std::env::var("HOME") {
            assert_eq!(shown(&format!("{home}/.claude")), "~/.claude");
        }
        assert_eq!(shown("/etc/hosts"), "/etc/hosts");
    }
}
